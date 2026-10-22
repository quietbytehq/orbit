mod commands;
pub mod error;
pub mod paths;
mod reveal;
pub mod rclone;
pub mod settings;
pub mod status;
mod tray;

use paths::AppPaths;
use reveal::{RevealGate, RevealState};
use rclone::{RcdState, Supervisor};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tray::TrayHandle;

pub const RCD_STATE_EVENT: &str = "rcd://state";

/// Closes the splash and shows the main window.
pub fn reveal_main_window(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    tray::show_main_window(app);
}

fn on_rcd_change(app: &AppHandle, state: &RcdState) {
    let _ = app.emit(RCD_STATE_EVENT, state);
    tray::refresh(app, state, status::overall(state, 0, 0));

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
            app.manage(TrayHandle::empty());
            let handle = app.handle().clone();
            let supervisor = Supervisor::start(spec, move |state| on_rcd_change(&handle, state));
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
