use crate::rclone::RcdState;
use crate::status::{self, OverallStatus};
use std::sync::Mutex;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime, Theme, Wry};

const TRAY_ID: &str = "orbit-tray";
const MENU_SHOW: &str = "show";
const MENU_STATUS: &str = "status";
const MENU_QUIT: &str = "quit";

/// Which icon set to use. White glyphs suit a dark system theme, dark glyphs a light one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    White,
    Dark,
}

impl Variant {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Variant::White,
            _ => Variant::Dark,
        }
    }
}

/// Which artwork state to show. `Base` is the neutral mark used while rclone is starting or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconState {
    Base,
    Synced,
    Syncing,
    Disconnected,
}

/// Picks the artwork state from rclone health and the overall status.
pub fn icon_state(rcd: &RcdState, status: OverallStatus) -> IconState {
    match rcd {
        RcdState::Starting | RcdState::Stopped => IconState::Base,
        _ => match status {
            OverallStatus::Synced => IconState::Synced,
            OverallStatus::Syncing => IconState::Syncing,
            OverallStatus::Disconnected => IconState::Disconnected,
        },
    }
}

/// Size used for the tray. Windows scales from 32 px; the Linux appindicator expects 24 px.
fn tray_size() -> u32 {
    if cfg!(windows) { 32 } else { 24 }
}

/// Returns PNG bytes for the given variant, state and tray size.
pub fn icon_bytes(variant: Variant, state: IconState, size: u32) -> &'static [u8] {
    macro_rules! tray_png {
        ($size:literal, $name:literal) => {
            include_bytes!(concat!(
                "../../assets/tray-icons-v2/",
                $size,
                "/",
                $name,
                ".png"
            ))
        };
    }
    match (size, variant, state) {
        (24, Variant::White, IconState::Base) => tray_png!("24", "white-base"),
        (24, Variant::White, IconState::Synced) => tray_png!("24", "white-synced"),
        (24, Variant::White, IconState::Syncing) => tray_png!("24", "white-syncing"),
        (24, Variant::White, IconState::Disconnected) => tray_png!("24", "white-disconnected"),
        (24, Variant::Dark, IconState::Base) => tray_png!("24", "dark-base"),
        (24, Variant::Dark, IconState::Synced) => tray_png!("24", "dark-synced"),
        (24, Variant::Dark, IconState::Syncing) => tray_png!("24", "dark-syncing"),
        (24, Variant::Dark, IconState::Disconnected) => tray_png!("24", "dark-disconnected"),
        (_, Variant::White, IconState::Base) => tray_png!("32", "white-base"),
        (_, Variant::White, IconState::Synced) => tray_png!("32", "white-synced"),
        (_, Variant::White, IconState::Syncing) => tray_png!("32", "white-syncing"),
        (_, Variant::White, IconState::Disconnected) => tray_png!("32", "white-disconnected"),
        (_, Variant::Dark, IconState::Base) => tray_png!("32", "dark-base"),
        (_, Variant::Dark, IconState::Synced) => tray_png!("32", "dark-synced"),
        (_, Variant::Dark, IconState::Syncing) => tray_png!("32", "dark-syncing"),
        (_, Variant::Dark, IconState::Disconnected) => tray_png!("32", "dark-disconnected"),
    }
}

/// The icon for a variant and state, in the form the platform's tray expects:
/// a multi-size `.ico` on Windows and a PNG elsewhere.
fn tray_image(variant: Variant, state: IconState) -> tauri::Result<Image<'static>> {
    Image::from_bytes(icon_file(variant, state))
}

#[cfg(windows)]
fn icon_file(variant: Variant, state: IconState) -> &'static [u8] {
    ico_bytes(variant, state)
}

#[cfg(not(windows))]
fn icon_file(variant: Variant, state: IconState) -> &'static [u8] {
    icon_bytes(variant, state, tray_size())
}

/// Returns the multi-size `.ico` for a variant and state. Windows picks the frame that fits the DPI.
#[cfg(windows)]
pub fn ico_bytes(variant: Variant, state: IconState) -> &'static [u8] {
    macro_rules! tray_ico {
        ($name:literal) => {
            include_bytes!(concat!("../../assets/tray-icons-v2/ico/", $name, ".ico"))
        };
    }
    match (variant, state) {
        (Variant::White, IconState::Base) => tray_ico!("white-base"),
        (Variant::White, IconState::Synced) => tray_ico!("white-synced"),
        (Variant::White, IconState::Syncing) => tray_ico!("white-syncing"),
        (Variant::White, IconState::Disconnected) => tray_ico!("white-disconnected"),
        (Variant::Dark, IconState::Base) => tray_ico!("dark-base"),
        (Variant::Dark, IconState::Synced) => tray_ico!("dark-synced"),
        (Variant::Dark, IconState::Syncing) => tray_ico!("dark-syncing"),
        (Variant::Dark, IconState::Disconnected) => tray_ico!("dark-disconnected"),
    }
}

/// The live tray, kept so its icon and menu can be updated.
pub struct TrayState {
    icon: TrayIcon<Wry>,
    status_item: MenuItem<Wry>,
    variant: Variant,
    current: Option<(IconState, OverallStatus)>,
}

pub struct TrayHandle(pub Mutex<Option<TrayState>>);

impl TrayHandle {
    pub fn empty() -> Self {
        Self(Mutex::new(None))
    }
}

/// Creates the tray icon and its menu. Call once from setup.
pub fn install(app: &AppHandle, variant: Variant) -> tauri::Result<()> {
    let status_item = MenuItem::with_id(app, MENU_STATUS, "Starting…", false, None::<&str>)?;
    let show = MenuItem::with_id(app, MENU_SHOW, "Show Orbit", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "Quit Orbit", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&status_item, &separator, &show, &quit])?;

    let icon = TrayIconBuilder::with_id(TRAY_ID)
        .icon(tray_image(variant, IconState::Base)?)
        .tooltip("Orbit for rclone")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => show_main_window(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    let handle = app.state::<TrayHandle>();
    *handle.0.lock().expect("tray lock") = Some(TrayState {
        icon,
        status_item,
        variant,
        current: None,
    });
    Ok(())
}

/// Updates the icon, tooltip and status line to match the current state. No-op if nothing changed.
pub fn refresh<R: Runtime>(app: &AppHandle<R>, rcd: &RcdState, overall: OverallStatus) {
    let handle = app.state::<TrayHandle>();
    let mut guard = handle.0.lock().expect("tray lock");
    let Some(tray) = guard.as_mut() else { return };

    let state = icon_state(rcd, overall);
    if tray.current == Some((state, overall)) {
        return;
    }
    let _ = tray.icon.set_icon(Some(
        tray_image(tray.variant, state).expect("bundled tray icon is valid"),
    ));
    let description = match rcd {
        RcdState::Starting | RcdState::Stopped => "Starting…".to_owned(),
        _ => status::describe(overall).to_owned(),
    };
    let _ = tray
        .icon
        .set_tooltip(Some(format!("Orbit for rclone: {description}")));
    let _ = tray.status_item.set_text(description);
    tray.current = Some((state, overall));
}

/// Switches between the white and dark icon sets when the system theme changes.
pub fn set_theme<R: Runtime>(app: &AppHandle<R>, theme: Theme) {
    let handle = app.state::<TrayHandle>();
    let mut guard = handle.0.lock().expect("tray lock");
    let Some(tray) = guard.as_mut() else { return };
    let variant = Variant::for_theme(theme);
    if tray.variant == variant {
        return;
    }
    tray.variant = variant;
    if let Some((state, _)) = tray.current {
        let _ = tray.icon.set_icon(Some(
            tray_image(variant, state).expect("bundled tray icon is valid"),
        ));
    }
}

pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_follows_system_theme() {
        assert_eq!(Variant::for_theme(Theme::Dark), Variant::White);
        assert_eq!(Variant::for_theme(Theme::Light), Variant::Dark);
    }

    #[test]
    fn starting_and_stopped_use_base_mark() {
        for rcd in [RcdState::Starting, RcdState::Stopped] {
            assert_eq!(
                icon_state(&rcd, OverallStatus::Disconnected),
                IconState::Base
            );
        }
    }

    #[test]
    fn healthy_states_map_to_status_icons() {
        let healthy = RcdState::Healthy {
            version: "v1".into(),
        };
        assert_eq!(
            icon_state(&healthy, OverallStatus::Synced),
            IconState::Synced
        );
        assert_eq!(
            icon_state(&healthy, OverallStatus::Syncing),
            IconState::Syncing
        );
        assert_eq!(
            icon_state(
                &RcdState::Failed {
                    message: String::new()
                },
                OverallStatus::Disconnected
            ),
            IconState::Disconnected
        );
    }

    #[test]
    fn every_variant_and_state_has_bundled_png() {
        for variant in [Variant::White, Variant::Dark] {
            for state in [
                IconState::Base,
                IconState::Synced,
                IconState::Syncing,
                IconState::Disconnected,
            ] {
                for size in [24, 32] {
                    let bytes = icon_bytes(variant, state, size);
                    assert_eq!(&bytes[1..4], b"PNG", "{variant:?} {state:?} {size}");
                }
            }
        }
    }
}
