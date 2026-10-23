use std::path::{Path, PathBuf};

/// Where Orbit keeps its files. All paths sit under the app data directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// rclone's own config. This is the only place credentials are stored.
    pub fn rclone_config(&self) -> PathBuf {
        self.data_dir.join("rclone.conf")
    }

    pub fn settings(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    pub fn jobs(&self) -> PathBuf {
        self.data_dir.join("jobs.json")
    }

    /// Remembered drives and folders (which storage, where, and whether to connect at startup).
    pub fn mounts(&self) -> PathBuf {
        self.data_dir.join("mounts.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn rclone_log(&self) -> PathBuf {
        self.logs_dir().join("rclone.log")
    }

    pub fn job_log(&self, file_name: &str) -> PathBuf {
        self.logs_dir().join(file_name)
    }
}

/// Directory that holds the Orbit executable, used to find the bundled rclone.
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_under_data_dir() {
        let paths = AppPaths::new("/data/orbit");
        assert_eq!(
            paths.rclone_config(),
            PathBuf::from("/data/orbit/rclone.conf")
        );
        assert_eq!(
            paths.rclone_log(),
            PathBuf::from("/data/orbit/logs/rclone.log")
        );
    }
}
