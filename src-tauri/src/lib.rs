mod commands;
pub mod error;
pub mod files;
pub mod jobs;
pub mod mounts;
pub mod paths;
pub mod rclone;
pub mod remotes;
mod reveal;
pub mod settings;
pub mod status;
mod tray;

use files::transfers::{Item as TransferItem, TransferQueue};
use jobs::JobView;
use jobs::runner::Runner;
use paths::AppPaths;
use rclone::{RcdState, Supervisor};
use remotes::health::HealthStore;
use reveal::{RevealGate, RevealState};
use settings::SettingsStore;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tray::TrayHandle;

pub const RCD_STATE_EVENT: &str = "rcd://state";
pub const TRANSFERS_EVENT: &str = "transfers://update";
pub const JOBS_EVENT: &str = "jobs://update";

/// Closes the splash and shows the main window.
pub fn reveal_main_window(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    tray::show_main_window(app);
}

/// Recomputes the tray state from rclone health and current activity.
pub fn refresh_status(app: &AppHandle) {
    let (Some(supervisor), Some(queue), Some(runner)) = (
        app.try_state::<Supervisor>(),
        app.try_state::<Arc<TransferQueue>>(),
        app.try_state::<Arc<Runner>>(),
    ) else {
        return;
    };
    let state = supervisor.state();
    tray::refresh(
        app,
        &state,
        status::overall(&state, queue.active(), runner.active()),
    );
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

            let mut settings = settings::load(&paths.settings()).unwrap_or_else(|err| {
                eprintln!("Orbit: could not read settings, using defaults: {err}");
                settings::Settings::default()
            });
            let spec = rclone::spec_for(&paths, &settings);
            let app_version = app.package_info().version.to_string();
            let splash_plan = settings.splash_plan(&app_version);
            if splash_plan == settings::SplashPlan::Full {
                // Remember that this version has shown its full intro, so the next launch is short.
                settings.last_splash_version = Some(app_version);
                if let Err(err) = settings::save(&paths.settings(), &settings) {
                    eprintln!("Orbit: could not save settings: {err}");
                }
            }
            if splash_plan == settings::SplashPlan::Off
                && let Some(splash) = app.get_webview_window("splash")
            {
                let _ = splash.close();
            }
            app.manage(splash_plan);
            app.manage(SettingsStore::new(settings.clone()));

            app.manage(RevealState(Mutex::new(RevealGate::new(
                splash_plan != settings::SplashPlan::Off,
            ))));
            app.manage(HealthStore::default());
            app.manage(TrayHandle::empty());
            let handle = app.handle().clone();
            let supervisor = Supervisor::start(spec, move |state| on_rcd_change(&handle, state));
            let transfer_handle = app.handle().clone();
            let queue = TransferQueue::start(
                supervisor.clone(),
                settings.transfer_concurrency as usize,
                move |items: &[TransferItem]| {
                    let _ = transfer_handle.emit(TRANSFERS_EVENT, items);
                    refresh_status(&transfer_handle);
                },
            );
            app.manage(queue);
            let jobs_handle = app.handle().clone();
            let runner = Runner::start(supervisor.clone(), &paths, move || {
                if let Some(runner) = jobs_handle.try_state::<Arc<Runner>>() {
                    let views: Vec<JobView> = runner.list();
                    let _ = jobs_handle.emit(JOBS_EVENT, views);
                }
                refresh_status(&jobs_handle);
            })?;
            app.manage(runner);
            app.manage(mounts::MountRegistry::default());
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
            commands::get_splash_plan,
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
            commands::get_settings,
            commands::save_settings,
            jobs::commands::list_jobs,
            jobs::commands::create_job,
            jobs::commands::update_job,
            jobs::commands::set_job_enabled,
            jobs::commands::delete_job,
            jobs::commands::run_job,
            jobs::commands::cancel_job,
            jobs::commands::read_job_log,
            mounts::mount_support,
            mounts::suggest_mount_location,
            mounts::list_mounts,
            mounts::mount_remote,
            mounts::unmount_remote,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Orbit");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            let supervisor = app_handle.state::<Supervisor>().inner().clone();
            tauri::async_runtime::block_on(async {
                if let Ok(client) = supervisor.client() {
                    mounts::unmount_all(&client).await;
                }
                supervisor.shutdown().await;
            });
        }
    });
}
