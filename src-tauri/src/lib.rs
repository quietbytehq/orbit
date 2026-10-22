mod commands;
pub mod error;
pub mod files;
pub mod paths;
mod reveal;
pub mod remotes;
pub mod rclone;
pub mod settings;
pub mod status;
mod tray;

use paths::AppPaths;
use files::transfers::{Item as TransferItem, TransferQueue};
use remotes::health::HealthStore;
use std::sync::Arc;
use reveal::{RevealGate, RevealState};
use rclone::{RcdState, Supervisor};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tray::TrayHandle;

pub const RCD_STATE_EVENT: &str = "rcd://state";
pub const TRANSFERS_EVENT: &str = "transfers://update";

/// Closes the splash and shows the main window.
pub fn reveal_main_window(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    tray::show_main_window(app);
}

/// Recomputes the tray state from rclone health and current activity.
pub fn refresh_status(app: &AppHandle) {
    let supervisor = app.state::<Supervisor>();
    let state = supervisor.state();
    let transfers = app.state::<Arc<TransferQueue>>().active();
    tray::refresh(app, &state, status::overall(&state, transfers, 0));
}

fn on_rcd_change(app: &AppHandle, state: &RcdState) {
    let _ = app.emit(RCD_STATE_EVENT, state);
    refresh_status(app);

    if matches!(state, RcdState::Healthy { .. } | RcdState::Failed { .. }) {
        let reveal = app.state::<RevealState>();
        let ready = reveal.0.lock().expect("reveal lock").rclone_settled();
        if ready {
            reveal_main_window(app);
        }
    }
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let paths = AppPaths::new(&data_dir);
            std::fs::create_dir_all(paths.logs_dir())?;

            let settings = settings::load(&paths.settings()).unwrap_or_else(|err| {
                eprintln!("Orbit: could not read settings, using defaults: {err}");
                settings::Settings::default()
            });
            let spec = rclone::spec_for(&paths, &settings);

            app.manage(RevealState(Mutex::new(RevealGate::new(settings.splash_enabled))));
            app.manage(HealthStore::default());
            app.manage(TrayHandle::empty());
            let handle = app.handle().clone();
            let supervisor = Supervisor::start(spec, move |state| on_rcd_change(&handle, state));
            let transfer_handle = app.handle().clone();
            let queue = TransferQueue::start(supervisor.clone(), settings.transfer_concurrency as usize, move |items: &[TransferItem]| {
                let _ = transfer_handle.emit(TRANSFERS_EVENT, items);
                refresh_status(&transfer_handle);
            });
            app.manage(queue);
            app.manage(supervisor);
            app.manage(paths);

            let variant = app
                .get_webview_window("main")
                .and_then(|window| window.theme().ok())
                .map(tray::Variant::for_theme)
                .unwrap_or(tray::Variant::Dark);
            tray::install(app.handle(), variant)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    // Closing the window keeps Orbit in the tray. Quit from the tray menu exits.
                    api.prevent_close();
                    let _ = window.hide();
                }
                WindowEvent::ThemeChanged(theme) => tray::set_theme(window.app_handle(), *theme),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_rcd_state,
            commands::restart_rcd,
            commands::splash_finished,
            remotes::commands::list_providers,
            remotes::commands::list_remotes,
            remotes::commands::get_remote_form,
            remotes::commands::create_remote,
            remotes::commands::start_browser_sign_in,
            remotes::commands::cancel_browser_sign_in,
            remotes::commands::update_remote,
            remotes::commands::rename_remote,
            remotes::commands::delete_remote,
            remotes::commands::test_remote,
            files::commands::inspect_local_paths,
            files::commands::list_files,
            files::commands::create_folder,
            files::commands::delete_items,
            files::commands::rename_item,
            files::commands::enqueue_transfers,
            files::commands::list_transfers,
            files::commands::cancel_transfer,
            files::commands::clear_finished_transfers,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Orbit");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            let supervisor = app_handle.state::<Supervisor>().inner().clone();
            tauri::async_runtime::block_on(supervisor.shutdown());
        }
    });
}
