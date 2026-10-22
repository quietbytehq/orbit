use crate::error::AppResult;
use crate::paths::AppPaths;
use crate::rclone::{self, RcdState, Supervisor};
use crate::reveal::RevealState;
use crate::settings;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_rcd_state(supervisor: State<'_, Supervisor>) -> RcdState {
    supervisor.state()
}

/// Restarts rclone, picking up any change to the rclone path in settings.
#[tauri::command]
pub fn restart_rcd(paths: State<'_, AppPaths>, supervisor: State<'_, Supervisor>) -> AppResult<()> {
    let settings = settings::load(&paths.settings())?;
    supervisor.restart(rclone::spec_for(&paths, &settings));
    Ok(())
}

/// Called by the splash when its animation ends or the user skips it.
#[tauri::command]
pub fn splash_finished(app: AppHandle, reveal: State<'_, RevealState>) {
    let ready = reveal.0.lock().expect("reveal lock").splash_finished();
    if ready {
        crate::reveal_main_window(&app);
    }
}
