mod commands;
pub mod error;
pub mod files;
pub mod jobs;
pub mod mounts;
pub mod paths;
pub mod quit;
pub mod rclone;
pub mod remotes;
mod reveal;
pub mod settings;
pub mod status;
pub mod text;
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
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, RunEvent, WindowEvent};
use tray::TrayHandle;

/// Argument the autostart entry passes so Orbit starts in the tray without a window.
pub const MINIMIZED_ARG: &str = "--minimized";

/// True when this launch came from the sign-in autostart entry: no splash, no main window.
pub struct StartHidden(pub bool);

/// Set once the automatic drives and folders have been reconnected after the engine came up.
struct MountsRestored(AtomicBool);

pub const RCD_STATE_EVENT: &str = "rcd://state";
pub const TRANSFERS_EVENT: &str = "transfers://update";
pub const JOBS_EVENT: &str = "jobs://update";

/// Closes the splash and shows the main window, unless this launch should stay in the tray.
pub fn reveal_main_window(app: &AppHandle) {
    if let Some(splash) = app.get_webview_window("splash") {
        let _ = splash.close();
    }
    let hidden = app
        .try_state::<StartHidden>()
        .is_some_and(|hidden| hidden.0);
    if !hidden {
        tray::show_main_window(app);
    }
}

/// Reconnects remembered drives and folders the first time the engine is healthy.
fn restore_mounts_once(app: &AppHandle) {
    let Some(flag) = app.try_state::<MountsRestored>() else {
        return;
    };
    if flag.0.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let supervisor = handle.state::<Supervisor>().inner().clone();
        let Ok(client) = supervisor.client() else {
            return;
        };
        let store = handle.state::<mounts::MountStore>();
        mounts::restore(&client, &store).await;
    });
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
    if matches!(state, RcdState::Healthy { .. }) {
        restore_mounts_once(app);
    }

    if matches!(state, RcdState::Healthy { .. } | RcdState::Failed { .. }) {
        let reveal = app.state::<RevealState>();
        let ready = reveal.0.lock().expect("reveal lock").rclone_settled();
        if ready {
            reveal_main_window(app);
        }
    }
}

pub fn run() {
    // A plain `orbit --quit` with no Orbit running has nothing to do. When one is running, the
    // single-instance plugin below hands the argument to it and this process ends there.
    let own_args: Vec<String> = std::env::args().collect();
    let app = tauri::Builder::default()
        // Must come first: a second launch is forwarded to the running Orbit and exits.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            if quit::wants_quit(&args) {
                quit::request(app, true);
            } else if !args.iter().any(|arg| arg == MINIMIZED_ARG) {
                tray::show_main_window(app);
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg(MINIMIZED_ARG)
                .build(),
        )
        .setup(move |app| {
            if quit::wants_quit(&own_args) {
                // No other Orbit took the request (we got this far), so there is nothing to quit.
                std::process::exit(0);
            }
            app.manage(quit::QuitState(AtomicBool::new(false)));
            let data_dir = app.path().app_data_dir()?;
            let paths = AppPaths::new(&data_dir);
            std::fs::create_dir_all(paths.logs_dir())?;
            let start_hidden = std::env::args().skip(1).any(|arg| arg == MINIMIZED_ARG);
            app.manage(StartHidden(start_hidden));
            app.manage(MountsRestored(AtomicBool::new(false)));

            let mut settings = settings::load(&paths.settings()).unwrap_or_else(|err| {
                eprintln!("Orbit: could not read settings, using defaults: {err}");
                settings::Settings::default()
            });
            let spec = rclone::spec_for(&paths, &settings);
            let app_version = app.package_info().version.to_string();
            let splash_plan = if start_hidden {
                settings::SplashPlan::Off
            } else {
                settings.splash_plan(&app_version)
            };
            // Keep the sign-in entry in step with the setting (it lives outside Orbit's files).
            if let Err(err) = settings::apply_start_on_login(app.handle(), settings.start_on_login)
            {
                eprintln!("Orbit: could not update the start-at-sign-in entry: {err}");
            }
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
            app.manage(mounts::MountStore::load(paths.mounts()));
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
            commands::app_info,
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
            mounts::list_saved_mounts,
            mounts::set_mount_auto_connect,
            mounts::forget_saved_mount,
            mounts::mount_remote,
            mounts::unmount_remote,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Orbit");

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            // Quit from the tray or `--quit` has already stopped everything. Any other exit
            // (the session ending, a signal handled by Tauri) gets the same clean stop, bounded.
            let already = app_handle
                .try_state::<quit::QuitState>()
                .is_some_and(|state| state.0.load(Ordering::SeqCst));
            if !already {
                let handle = app_handle.clone();
                tauri::async_runtime::block_on(async move {
                    let _ = tokio::time::timeout(quit::WATCHDOG, quit::shutdown_services(&handle))
                        .await;
                });
            }
        }
    });
}
