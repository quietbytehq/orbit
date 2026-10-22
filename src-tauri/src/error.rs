use serde::Serialize;
use std::fmt;

/// Error returned by every Tauri command. Serializes to `{ kind, message }`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// rclone is not running or cannot be reached.
    NotRunning,
    /// rclone answered with an error.
    Rclone,
    /// The caller passed a value we refuse to use.
    InvalidInput,
    /// A local file or folder operation failed.
    Io,
    /// The feature is not available on this platform.
    Unsupported,
    /// Something went wrong inside Orbit.
    Internal,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::internal(format!("unexpected data from rclone: {err}"))
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_kind_and_message() {
        let err = AppError::new(ErrorKind::NotRunning, "rclone is starting");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["kind"], "notRunning");
        assert_eq!(json["message"], "rclone is starting");
    }
}
