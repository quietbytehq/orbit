use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// User preferences. Holds no credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: Theme,
    /// Number of transfers that run at the same time (1..=8).
    pub transfer_concurrency: u8,
    /// Advanced: use this rclone binary instead of the bundled one.
    pub rclone_path: Option<PathBuf>,
    pub rclone_log_level: LogLevel,
    pub splash_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Error,
    Notice,
    Info,
    Debug,
}

impl LogLevel {
    pub fn as_rclone_arg(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Notice => "NOTICE",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            transfer_concurrency: 3,
            rclone_path: None,
            rclone_log_level: LogLevel::Info,
            splash_enabled: true,
        }
    }
}

impl Settings {
    pub const MAX_CONCURRENCY: u8 = 8;

    /// Returns an error if a value is out of range.
    pub fn validate(&self) -> AppResult<()> {
        if self.transfer_concurrency == 0 || self.transfer_concurrency > Self::MAX_CONCURRENCY {
            return Err(AppError::invalid(format!(
                "Concurrent transfers must be between 1 and {}.",
                Self::MAX_CONCURRENCY
            )));
        }
        Ok(())
    }
}

/// The settings in use, shared with commands and the rest of the app.
pub struct SettingsStore(pub Mutex<Settings>);

impl SettingsStore {
    pub fn new(settings: Settings) -> Self {
        Self(Mutex::new(settings))
    }

    pub fn get(&self) -> Settings {
        self.0.lock().expect("settings lock").clone()
    }

    pub fn set(&self, settings: Settings) {
        *self.0.lock().expect("settings lock") = settings;
    }
}

/// Loads settings from `path`. A missing file yields the defaults.
pub fn load(path: &Path) -> AppResult<Settings> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(err) => Err(err.into()),
    }
}

/// Writes settings atomically: a temp file is written, then renamed over `path`.
pub fn save(path: &Path, settings: &Settings) -> AppResult<()> {
    settings.validate()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load(&dir.path().join("settings.json")).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = Settings {
            theme: Theme::Dark,
            transfer_concurrency: 5,
            rclone_path: Some(PathBuf::from("/opt/rclone")),
            rclone_log_level: LogLevel::Debug,
            splash_enabled: false,
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap(), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn rejects_out_of_range_concurrency() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            transfer_concurrency: 0,
            ..Settings::default()
        };
        assert!(save(&dir.path().join("s.json"), &settings).is_err());
    }

    #[test]
    fn partial_file_fills_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"theme":"dark"}"#).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.transfer_concurrency, 3);
    }
}
