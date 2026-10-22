//! Integration tests that run a real `rclone rcd`.
//!
//! They look for rclone via `ORBIT_TEST_RCLONE`, then the bundled sidecar, then `PATH`,
//! and skip with a message when none is found.

use orbit_lib::error::ErrorKind;
use orbit_lib::paths::AppPaths;
use orbit_lib::rclone::process::{RcdProcess, RcdSpec};
use orbit_lib::rclone::resolve::resolve_binary;
use orbit_lib::rclone::{RcdState, Supervisor};
use orbit_lib::settings::LogLevel;
use serde_json::json;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn find_rclone() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("ORBIT_TEST_RCLONE") {
        return Some(PathBuf::from(path));
    }
    // The fetch script names the sidecar `rclone-<target-triple>`, as Tauri expects.
    let sidecar = std::fs::read_dir(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("rclone-")));
    if sidecar.is_some() {
        return sidecar;
    }
    let path_env = std::env::var_os("PATH");
    resolve_binary(None, None, path_env.as_deref()).ok()
}

macro_rules! require_rclone {
    () => {
        match find_rclone() {
            Some(path) => path,
            None => {
                eprintln!("skipping: no rclone found (set ORBIT_TEST_RCLONE)");
                return;
            }
        }
    };
}

fn spec(binary: PathBuf, dir: &std::path::Path) -> RcdSpec {
    let paths = AppPaths::new(dir);
    RcdSpec {
        binary,
        config_path: paths.rclone_config(),
        log_path: paths.rclone_log(),
        log_level: LogLevel::Error,
    }
}

async fn wait_for<F: Fn(&RcdState) -> bool>(supervisor: &Supervisor, timeout: Duration, pred: F) -> RcdState {
    let deadline = Instant::now() + timeout;
    loop {
        let state = supervisor.state();
        if pred(&state) || Instant::now() >= deadline {
            return state;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn is_healthy(state: &RcdState) -> bool {
    matches!(state, RcdState::Healthy { .. })
}

/// rcd needs a moment to bind its port, so poll until it answers.
async fn wait_for_version(process: &RcdProcess) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match process.version().await {
            Ok(version) => return version,
            Err(err) if Instant::now() >= deadline => panic!("rcd never answered: {err}"),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

#[tokio::test]
async fn rcd_answers_with_auth_and_rejects_without() {
    let binary = require_rclone!();
    let dir = tempfile::tempdir().unwrap();
    let process = RcdProcess::spawn(&spec(binary, dir.path())).unwrap();

    let version = wait_for_version(&process).await;
    assert!(version.starts_with('v'), "unexpected version {version}");

    // Same endpoint, no credentials: rclone must refuse.
    let port = process.port();
    let anon = reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/core/version"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(anon.status(), reqwest::StatusCode::UNAUTHORIZED);

    process.stop().await;
    assert!(
        std::net::TcpStream::connect(("127.0.0.1", port)).is_err(),
        "rcd should no longer listen after stop"
    );
}

#[tokio::test]
async fn supervisor_reaches_healthy_then_stops_cleanly() {
    let binary = require_rclone!();
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::start(spec(binary, dir.path()), |_| {});

    let state = wait_for(&supervisor, Duration::from_secs(30), is_healthy).await;
    assert!(is_healthy(&state), "expected healthy, got {state:?}");
    assert!(supervisor.client().is_ok());

    supervisor.shutdown().await;
    assert_eq!(supervisor.state(), RcdState::Stopped);
    assert_eq!(supervisor.client().unwrap_err().kind, ErrorKind::NotRunning);
}

#[tokio::test]
async fn supervisor_restarts_with_new_spec() {
    let binary = require_rclone!();
    let dir = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::start(spec(binary.clone(), dir.path()), |_| {});
    assert!(is_healthy(&wait_for(&supervisor, Duration::from_secs(30), is_healthy).await));

    supervisor.restart(spec(binary, dir.path()));

    // Give the supervisor a moment to leave the healthy state, then wait for it to return.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let state = wait_for(&supervisor, Duration::from_secs(30), is_healthy).await;
    assert!(is_healthy(&state), "expected healthy after restart, got {state:?}");

    supervisor.shutdown().await;
}

#[tokio::test]
async fn supervisor_gives_up_after_retry_budget() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no-such-rclone");
    let supervisor = Supervisor::start(spec(missing, dir.path()), |_| {});

    // Backoff is 1 s + 2 s + 4 s between attempts, so allow a generous window.
    let state = wait_for(
        &supervisor,
        Duration::from_secs(30),
        |s| matches!(s, RcdState::Failed { .. }),
    )
    .await;
    match state {
        RcdState::Failed { message } => assert!(message.contains("Could not start rclone"), "{message}"),
        other => panic!("expected Failed, got {other:?}"),
    }

    supervisor.shutdown().await;
}
