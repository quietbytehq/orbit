use super::client::RcClient;
use super::process::{RcdProcess, RcdSpec};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::text;
use serde::Serialize;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

/// How often the running rcd is health-checked.
pub const HEALTH_INTERVAL: Duration = Duration::from_secs(2);
/// Consecutive failed health checks before a running rcd is treated as crashed.
pub const HEALTH_MISSES_BEFORE_RESTART: u32 = 3;
/// Restarts in a row before the supervisor gives up and reports `Failed`.
pub const MAX_RESTARTS: u32 = 3;
/// How long a freshly started rcd may take to answer its first health check.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const STARTUP_POLL: Duration = Duration::from_millis(250);
const STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// Health of the rclone daemon as seen by the UI and the tray.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum RcdState {
    Starting,
    Healthy { version: String },
    Restarting { attempt: u32 },
    Failed { message: String },
    Stopped,
}

/// Delay before restart `attempt` (1-based): 1 s, 2 s, 4 s, capped at 16 s.
pub fn restart_delay(attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1).min(4);
    Duration::from_secs(1u64 << exponent)
}

enum Command {
    Restart(RcdSpec),
    Stop(oneshot::Sender<()>),
}

type ChangeHook = Arc<dyn Fn(&RcdState) + Send + Sync>;

struct Shared {
    state: RwLock<RcdState>,
    client: RwLock<Option<RcClient>>,
    on_change: ChangeHook,
}

impl Shared {
    fn set_state(&self, state: RcdState) {
        *self.state.write().expect("state lock") = state.clone();
        (self.on_change)(&state);
    }
}

/// Owns the rclone daemon and keeps it running. Cheap to clone.
#[derive(Clone)]
pub struct Supervisor {
    tx: mpsc::UnboundedSender<Command>,
    shared: Arc<Shared>,
}

impl Supervisor {
    /// Starts supervising in the background on Tauri's async runtime.
    pub fn start(spec: RcdSpec, on_change: impl Fn(&RcdState) + Send + Sync + 'static) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let shared = Arc::new(Shared {
            state: RwLock::new(RcdState::Starting),
            client: RwLock::new(None),
            on_change: Arc::new(on_change),
        });
        tauri::async_runtime::spawn(run(rx, shared.clone(), spec));
        Self { tx, shared }
    }

    pub fn state(&self) -> RcdState {
        self.shared.state.read().expect("state lock").clone()
    }

    /// Returns a client for the running rcd, or `NotRunning` if it is not healthy.
    pub fn client(&self) -> AppResult<RcClient> {
        self.shared
            .client
            .read()
            .expect("client lock")
            .clone()
            .ok_or_else(|| AppError::new(ErrorKind::NotRunning, text::ENGINE_NOT_READY))
    }

    /// Restarts rcd with a new spec (for example after the rclone path changes).
    pub fn restart(&self, spec: RcdSpec) {
        let _ = self.tx.send(Command::Restart(spec));
    }

    /// Stops rcd and waits for it to exit. Safe to call more than once.
    pub async fn shutdown(&self) {
        let (done_tx, done_rx) = oneshot::channel();
        if self.tx.send(Command::Stop(done_tx)).is_err() {
            return;
        }
        let _ = tokio::time::timeout(STOP_TIMEOUT, done_rx).await;
    }
}

enum Monitor {
    Crashed(String),
    Stop(Option<oneshot::Sender<()>>),
    Restart(RcdSpec),
}

async fn run(mut rx: mpsc::UnboundedReceiver<Command>, shared: Arc<Shared>, mut spec: RcdSpec) {
    let mut failures: u32 = 0;
    loop {
        shared.set_state(RcdState::Starting);
        let reason = match start(&spec).await {
            Ok((mut process, version)) => {
                failures = 0;
                *shared.client.write().expect("client lock") = Some(process.client());
                shared.set_state(RcdState::Healthy { version });
                let outcome = monitor(&mut process, &mut rx).await;
                *shared.client.write().expect("client lock") = None;
                match outcome {
                    Monitor::Stop(done) => {
                        process.stop().await;
                        shared.set_state(RcdState::Stopped);
                        if let Some(done) = done {
                            let _ = done.send(());
                        }
                        return;
                    }
                    Monitor::Restart(next) => {
                        process.stop().await;
                        spec = next;
                        continue;
                    }
                    Monitor::Crashed(reason) => {
                        drop(process);
                        reason
                    }
                }
            }
            Err(err) => err.message,
        };

        failures += 1;
        if failures > MAX_RESTARTS {
            shared.set_state(RcdState::Failed { message: reason });
            // Stay idle until the user asks for a restart or the app quits.
            match wait_while_failed(&mut rx).await {
                Some(next) => {
                    spec = next;
                    failures = 0;
                    continue;
                }
                None => {
                    shared.set_state(RcdState::Stopped);
                    return;
                }
            }
        }

        shared.set_state(RcdState::Restarting { attempt: failures });
        tokio::select! {
            _ = tokio::time::sleep(restart_delay(failures)) => {}
            cmd = rx.recv() => match cmd {
                Some(Command::Restart(next)) => {
                    spec = next;
                    failures = 0;
                }
                Some(Command::Stop(done)) => {
                    shared.set_state(RcdState::Stopped);
                    let _ = done.send(());
                    return;
                }
                None => {
                    shared.set_state(RcdState::Stopped);
                    return;
                }
            }
        }
    }
}

/// Starts rcd and waits until it answers a health check.
async fn start(spec: &RcdSpec) -> AppResult<(RcdProcess, String)> {
    let mut process = RcdProcess::spawn(spec)?;
    let deadline = tokio::time::Instant::now() + STARTUP_TIMEOUT;
    loop {
        if let Some(status) = process.try_exit_status()? {
            return Err(
                AppError::new(ErrorKind::NotRunning, text::ENGINE_EXITED_WHILE_STARTING)
                    .with_details(format!("exit status {status}")),
            );
        }
        match process.version().await {
            Ok(version) => return Ok((process, version)),
            Err(err) if tokio::time::Instant::now() >= deadline => {
                return Err(
                    AppError::new(ErrorKind::NotRunning, text::ENGINE_SLOW_START)
                        .with_details(err.details.unwrap_or(err.message)),
                );
            }
            Err(_) => tokio::time::sleep(STARTUP_POLL).await,
        }
    }
}

async fn monitor(process: &mut RcdProcess, rx: &mut mpsc::UnboundedReceiver<Command>) -> Monitor {
    let mut ticker = tokio::time::interval(HEALTH_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    ticker.tick().await; // the first tick completes immediately
    let mut misses = 0;
    loop {
        tokio::select! {
            cmd = rx.recv() => {
                return match cmd {
                    Some(Command::Stop(done)) => Monitor::Stop(Some(done)),
                    Some(Command::Restart(next)) => Monitor::Restart(next),
                    None => Monitor::Stop(None),
                };
            }
            _ = ticker.tick() => {
                match process.try_exit_status() {
                    Ok(Some(status)) => return Monitor::Crashed(format!("{} ({status})", text::ENGINE_EXITED)),
                    Ok(None) => {}
                    Err(err) => return Monitor::Crashed(err.message),
                }
                match process.version().await {
                    Ok(_) => misses = 0,
                    Err(_) => {
                        misses += 1;
                        if misses >= HEALTH_MISSES_BEFORE_RESTART {
                            return Monitor::Crashed(text::ENGINE_UNRESPONSIVE.into());
                        }
                    }
                }
            }
        }
    }
}

/// While failed, only a restart request or a stop request is meaningful.
async fn wait_while_failed(rx: &mut mpsc::UnboundedReceiver<Command>) -> Option<RcdSpec> {
    match rx.recv().await {
        Some(Command::Restart(next)) => Some(next),
        Some(Command::Stop(done)) => {
            let _ = done.send(());
            None
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_delays_back_off_and_cap() {
        assert_eq!(restart_delay(1), Duration::from_secs(1));
        assert_eq!(restart_delay(2), Duration::from_secs(2));
        assert_eq!(restart_delay(3), Duration::from_secs(4));
        assert_eq!(restart_delay(4), Duration::from_secs(8));
        assert_eq!(restart_delay(9), Duration::from_secs(16));
    }

    #[test]
    fn retry_budget_is_three_restarts() {
        assert_eq!(MAX_RESTARTS, 3);
    }

    #[test]
    fn state_serializes_with_tag() {
        let json = serde_json::to_value(RcdState::Healthy {
            version: "v1.75.1".into(),
        })
        .unwrap();
        assert_eq!(json["state"], "healthy");
        assert_eq!(json["version"], "v1.75.1");
    }
}
