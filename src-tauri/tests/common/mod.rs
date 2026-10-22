//! Helpers shared by the integration tests. Each test binary includes this module.
#![allow(dead_code)]

use orbit_lib::paths::AppPaths;
use orbit_lib::rclone::process::{RcdProcess, RcdSpec};
use orbit_lib::rclone::resolve::resolve_binary;
use orbit_lib::settings::LogLevel;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub fn find_rclone() -> Option<PathBuf> {
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

pub fn spec(binary: PathBuf, dir: &std::path::Path) -> RcdSpec {
    let paths = AppPaths::new(dir);
    RcdSpec {
        binary,
        config_path: paths.rclone_config(),
        log_path: paths.rclone_log(),
        log_level: LogLevel::Error,
    }
}

/// rcd needs a moment to bind its port, so poll until it answers.
pub async fn wait_for_version(process: &RcdProcess) -> String {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match process.version().await {
            Ok(version) => return version,
            Err(err) if Instant::now() >= deadline => panic!("rcd never answered: {err}"),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
}

