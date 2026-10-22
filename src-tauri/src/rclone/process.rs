use super::client::{Endpoint, RcClient};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::settings::LogLevel;
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use serde_json::json;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const RC_USER: &str = "orbit";
const TOKEN_LEN: usize = 40;
const QUIT_GRACE: Duration = Duration::from_secs(3);

/// Everything needed to start `rclone rcd`.
#[derive(Debug, Clone)]
pub struct RcdSpec {
    pub binary: PathBuf,
    pub config_path: PathBuf,
    pub log_path: PathBuf,
    pub log_level: LogLevel,
}

/// A running `rclone rcd` child process.
pub struct RcdProcess {
    child: Child,
    endpoint: Endpoint,
}

#[derive(Deserialize)]
struct VersionReply {
    version: String,
}

impl RcdProcess {
    /// Starts `rclone rcd` on a free loopback port with a fresh random password.
    pub fn spawn(spec: &RcdSpec) -> AppResult<Self> {
        if let Some(parent) = spec.log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let port = free_loopback_port()?;
        let pass = Alphanumeric.sample_string(&mut rand::rng(), TOKEN_LEN);
        let endpoint = Endpoint {
            port,
            user: RC_USER.to_owned(),
            pass: pass.clone(),
        };

        let mut command = Command::new(&spec.binary);
        command
            .arg("rcd")
            .arg("--rc-addr")
            .arg(format!("127.0.0.1:{port}"))
            .arg("--rc-user")
            .arg(RC_USER)
            .arg("--config")
            .arg(&spec.config_path)
            .arg("--log-file")
            .arg(&spec.log_path)
            .arg("--log-level")
            .arg(spec.log_level.as_rclone_arg())
            // Passed through the environment so the password stays out of the process list.
            .env("RCLONE_RC_PASS", &pass)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        hide_console(&mut command);
        ask_kernel_to_end_child_with_parent(&mut command);

        let child = command.spawn().map_err(|err| {
            AppError::new(
                ErrorKind::NotRunning,
                format!("Could not start rclone ({}): {err}", spec.binary.display()),
            )
        })?;
        Ok(Self { child, endpoint })
    }

    pub fn client(&self) -> RcClient {
        RcClient::new(&self.endpoint)
    }

    pub fn port(&self) -> u16 {
        self.endpoint.port
    }

    /// Returns the child's exit status if it has exited.
    pub fn try_exit_status(&mut self) -> AppResult<Option<std::process::ExitStatus>> {
        Ok(self.child.try_wait()?)
    }

    /// Asks rclone to quit, then kills it if it is still running after a short grace period.
    pub async fn stop(mut self) {
        let client = self.client();
        // Best effort: a dead process will not answer, and the kill below covers that.
        let _ = client.call_value("core/quit", &json!({})).await;

        let deadline = Instant::now() + QUIT_GRACE;
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    /// Reads the rclone version. Used as the health check.
    pub async fn version(&self) -> AppResult<String> {
        let reply: VersionReply = self.client().call("core/version", &json!({})).await?;
        Ok(reply.version)
    }
}

impl Drop for RcdProcess {
    fn drop(&mut self) {
        // Never leave an orphaned rcd behind if the owner forgets to call `stop`.
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn free_loopback_port() -> AppResult<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

/// If Orbit dies without running its exit hook (crash, SIGKILL), the kernel signals rcd so it
/// does not outlive us and keep its port open.
#[cfg(target_os = "linux")]
fn ask_kernel_to_end_child_with_parent(command: &mut Command) {
    use std::os::unix::process::CommandExt;
    // SAFETY: the closure only calls prctl, which is async-signal-safe and allocates nothing.
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn ask_kernel_to_end_child_with_parent(_command: &mut Command) {}

#[cfg(windows)]
fn hide_console(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_port_is_usable() {
        let port = free_loopback_port().unwrap();
        assert!(port > 0);
    }

    #[test]
    fn spawn_fails_with_clear_message_for_missing_binary() {
        let dir = tempfile::tempdir().unwrap();
        let spec = RcdSpec {
            binary: dir.path().join("does-not-exist"),
            config_path: dir.path().join("rclone.conf"),
            log_path: dir.path().join("rclone.log"),
            log_level: LogLevel::Info,
        };
        let err = RcdProcess::spawn(&spec).err().expect("spawn should fail");
        assert_eq!(err.kind, ErrorKind::NotRunning);
        assert!(err.message.contains("Could not start rclone"));
    }
}
