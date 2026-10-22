use crate::error::AppResult;
use crate::files::transfers::TransferQueue;
use crate::paths::AppPaths;
use crate::rclone::{self, RcdState, Supervisor};
use crate::reveal::RevealState;
use crate::settings::{self, Settings, SettingsStore};
use std::sync::Arc;
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

#[tauri::command]
pub fn get_settings(store: State<'_, SettingsStore>) -> Settings {
    store.get()
}

/// Saves settings and applies them. Theme and concurrency take effect at once. A new rclone path
/// or log level restarts rclone. The splash setting applies on the next launch.
#[tauri::command]
pub fn save_settings(
    settings: Settings,
    paths: State<'_, AppPaths>,
    store: State<'_, SettingsStore>,
    supervisor: State<'_, Supervisor>,
    queue: State<'_, Arc<TransferQueue>>,
) -> AppResult<Settings> {
    settings.validate()?;
    let previous = store.get();
    settings::save(&paths.settings(), &settings)?;
    store.set(settings.clone());

    if previous.rclone_path != settings.rclone_path
        || previous.rclone_log_level != settings.rclone_log_level
    {
        supervisor.restart(rclone::spec_for(&paths, &settings));
    }
    if previous.transfer_concurrency != settings.transfer_concurrency {
        queue.set_limit(settings.transfer_concurrency as usize);
    }
    Ok(settings)
}
