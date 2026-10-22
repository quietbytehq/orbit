use crate::error::{AppError, ErrorKind};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// The binary name for the current platform.
pub fn binary_name() -> &'static str {
    if cfg!(windows) {
        "rclone.exe"
    } else {
        "rclone"
    }
}

/// Picks the rclone binary to run, in order:
/// 1. the advanced override from settings,
/// 2. the sidecar that ships next to the Orbit executable,
/// 3. `rclone` found on `path_env`.
pub fn resolve_binary(
    override_path: Option<&Path>,
    exe_dir: Option<&Path>,
    path_env: Option<&OsStr>,
) -> Result<PathBuf, AppError> {
    if let Some(path) = override_path {
        return if path.is_file() {
            Ok(path.to_path_buf())
        } else {
            Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("The rclone path in settings does not exist: {}", path.display()),
            ))
        };
    }

    if let Some(dir) = exe_dir {
        let sidecar = dir.join(binary_name());
        if sidecar.is_file() {
            return Ok(sidecar);
        }
    }

    if let Some(path_env) = path_env {
        if let Some(found) = std::env::split_paths(path_env)
            .map(|dir| dir.join(binary_name()))
            .find(|candidate| candidate.is_file())
        {
            return Ok(found);
        }
    }

    Err(AppError::new(
        ErrorKind::NotRunning,
        "Could not find rclone. Reinstall Orbit or set a custom rclone path in Settings.",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, b"").unwrap();
        path
    }

    #[test]
    fn override_wins_over_everything() {
        let dir = tempfile::tempdir().unwrap();
        let custom = touch(dir.path(), "my-rclone");
        let sidecar_dir = tempfile::tempdir().unwrap();
        touch(sidecar_dir.path(), binary_name());
        let got = resolve_binary(Some(&custom), Some(sidecar_dir.path()), None).unwrap();
        assert_eq!(got, custom);
    }

    #[test]
    fn missing_override_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope");
        let err = resolve_binary(Some(&missing), None, None).unwrap_err();
        assert_eq!(err.kind, ErrorKind::InvalidInput);
    }

    #[test]
    fn sidecar_beats_path() {
        let sidecar_dir = tempfile::tempdir().unwrap();
        let sidecar = touch(sidecar_dir.path(), binary_name());
        let path_dir = tempfile::tempdir().unwrap();
        touch(path_dir.path(), binary_name());
        let path_env = std::env::join_paths([path_dir.path()]).unwrap();
        let got = resolve_binary(None, Some(sidecar_dir.path()), Some(&path_env)).unwrap();
        assert_eq!(got, sidecar);
    }

    #[test]
    fn falls_back_to_path() {
        let empty = tempfile::tempdir().unwrap();
        let path_dir = tempfile::tempdir().unwrap();
        let on_path = touch(path_dir.path(), binary_name());
        let path_env = std::env::join_paths([empty.path(), path_dir.path()]).unwrap();
        let got = resolve_binary(None, Some(empty.path()), Some(&path_env)).unwrap();
        assert_eq!(got, on_path);
    }

    #[test]
    fn reports_when_nothing_is_found() {
        let empty = tempfile::tempdir().unwrap();
        let path_env = std::env::join_paths([empty.path()]).unwrap();
        let err = resolve_binary(None, Some(empty.path()), Some(&path_env)).unwrap_err();
        assert_eq!(err.kind, ErrorKind::NotRunning);
    }
}
