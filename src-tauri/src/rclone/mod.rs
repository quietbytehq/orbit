pub mod client;
pub mod process;
pub mod resolve;
pub mod supervisor;

use crate::paths::{AppPaths, exe_dir};
use crate::settings::Settings;

pub use client::RcClient;
pub use process::RcdSpec;
pub use supervisor::{RcdState, Supervisor};

/// Builds the spec for `rclone rcd` from the app paths and current settings.
///
/// If no binary can be found, this points at where the bundled sidecar should be, so the
/// supervisor reports a concrete path in its error message.
pub fn spec_for(paths: &AppPaths, settings: &Settings) -> RcdSpec {
    let exe_dir = exe_dir();
    let path_env = std::env::var_os("PATH");
    let binary = resolve::resolve_binary(
        settings.rclone_path.as_deref(),
        exe_dir.as_deref(),
        path_env.as_deref(),
    )
    .unwrap_or_else(|_| exe_dir.unwrap_or_default().join(resolve::binary_name()));
    RcdSpec {
        binary,
        config_path: paths.rclone_config(),
        log_path: paths.rclone_log(),
        log_level: settings.rclone_log_level,
    }
}
