//! Quitting Orbit cleanly, from the tray or from a second launch with `--quit`.
//!
//! Order: stop running transfers and backups, disconnect drives and folders, stop the storage
//! engine, drop the tray icon, exit the process. Every step has a timeout, and a watchdog ends the
//! process even if a step hangs, so Quit always quits.

use crate::files::transfers::{State as TransferState, TransferQueue};
use crate::jobs::runner::Runner;
use crate::rclone::{RcdState, Supervisor};
use crate::{mounts, text, tray};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

/// Argument that asks an already running Orbit to quit, through the single-instance channel.
pub const QUIT_ARG: &str = "--quit";

/// How long cancelled transfers and backups get to wind down before the engine is stopped.
pub const CANCEL_WAIT: Duration = Duration::from_secs(5);
/// Longest wait for drives and folders to disconnect.
pub const UNMOUNT_TIMEOUT: Duration = Duration::from_secs(8);
/// After this the process exits whatever is still going on.
pub const WATCHDOG: Duration = Duration::from_secs(25);

/// Set once quitting has begun, so the sequence runs at most once.
pub struct QuitState(pub AtomicBool);

/// What the clean stop managed to do. Returned for logs and tests.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub cancelled_transfers: usize,
    pub cancelled_jobs: usize,
    /// True when nothing was still running after the cancel wait.
    pub idle: bool,
    pub engine_stopped: bool,
}

/// Number of transfers and backups running or waiting right now.
pub fn active_work(queue: &TransferQueue, runner: &Runner) -> usize {
    queue.active() + runner.active()
}

/// Stops everything the engine is doing, then the engine itself. Usable without an app, which
/// is how the integration test drives it.
pub async fn stop_everything(
    queue: &TransferQueue,
    runner: &Runner,
    supervisor: &Supervisor,
) -> Report {
    let mut report = Report::default();
    for item in queue.snapshot() {
        if matches!(item.state, TransferState::Queued | TransferState::Running) {
            let _ = queue.cancel(item.id).await;
            report.cancelled_transfers += 1;
        }
    }
    for view in runner.list() {
        if view.progress.is_some() {
            let _ = runner.cancel(&view.job.id).await;
            report.cancelled_jobs += 1;
        }
    }
    let deadline = Instant::now() + CANCEL_WAIT;
    while active_work(queue, runner) > 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    report.idle = active_work(queue, runner) == 0;

    if let Ok(client) = supervisor.client() {
        let _ = tokio::time::timeout(UNMOUNT_TIMEOUT, mounts::unmount_all(&client)).await;
    }
    supervisor.shutdown().await;
    report.engine_stopped = matches!(supervisor.state(), RcdState::Stopped);
    report
}

/// Entry point from the tray and from `--quit`. Asks first when something is running, unless
/// `force` is set (a scripted quit).
pub fn request(app: &AppHandle, force: bool) {
    let busy = match (
        app.try_state::<Arc<TransferQueue>>(),
        app.try_state::<Arc<Runner>>(),
    ) {
        (Some(queue), Some(runner)) => active_work(&queue, &runner),
        _ => 0,
    };
    if busy > 0 && !force {
        use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
        let handle = app.clone();
        app.dialog()
            .message(text::quit_busy(busy))
            .title(text::QUIT_TITLE)
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom(
                text::QUIT_ANYWAY.to_owned(),
                text::KEEP_RUNNING.to_owned(),
            ))
            .show(move |confirmed| {
                if confirmed {
                    begin(&handle);
                }
            });
        return;
    }
    begin(app);
}

/// Starts the quit sequence once. Windows and the tray icon go away at once so quitting feels
/// immediate; the engine is stopped in the background, then the process exits.
pub fn begin(app: &AppHandle) {
    if let Some(state) = app.try_state::<QuitState>()
        && state.0.swap(true, Ordering::SeqCst)
    {
        return;
    }
    for window in app.webview_windows().values() {
        let _ = window.hide();
    }
    tray::remove(app);

    std::thread::spawn(|| {
        std::thread::sleep(WATCHDOG);
        eprintln!("Orbit: quit took too long, exiting now.");
        std::process::exit(0);
    });

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        shutdown_services(&handle).await;
        handle.exit(0);
    });
}

/// Runs the clean stop against the app's services. Also used as the fallback when the app exits
/// another way (for example the session ending) and nothing has stopped the engine yet.
pub async fn shutdown_services(app: &AppHandle) {
    let (Some(queue), Some(runner), Some(supervisor)) = (
        app.try_state::<Arc<TransferQueue>>(),
        app.try_state::<Arc<Runner>>(),
        app.try_state::<Supervisor>(),
    ) else {
        return;
    };
    let report = stop_everything(&queue, &runner, &supervisor).await;
    if !report.engine_stopped {
        eprintln!("Orbit: the storage engine did not confirm it stopped; it is killed on exit.");
    }
}

/// True when the launch arguments ask a running Orbit to quit.
pub fn wants_quit(args: &[String]) -> bool {
    args.iter().any(|arg| arg == QUIT_ARG)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quit_argument_is_recognised() {
        assert!(wants_quit(&["orbit".into(), QUIT_ARG.into()]));
        assert!(!wants_quit(&["orbit".into(), "--minimized".into()]));
    }

    #[test]
    fn report_defaults_to_nothing_done() {
        let report = Report::default();
        assert_eq!(report.cancelled_transfers, 0);
        assert!(!report.engine_stopped);
    }
}
