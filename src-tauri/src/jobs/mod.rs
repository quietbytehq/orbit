//! Saved sync and copy jobs. Each run is an rclone async job whose progress and errors are
//! recorded, with a log file per run.

pub mod commands;
pub mod runner;
pub mod store;

use crate::files::Endpoint;
use serde::{Deserialize, Serialize};

/// How many runs are kept per job. Older runs and their log files are removed.
pub const HISTORY_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobKind {
    /// Makes the destination match the source. Files that only exist in the destination are deleted.
    Sync,
    /// Copies new and changed files. Nothing is deleted.
    Copy,
}

/// A saved job. Holds names and locations only, never credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub name: String,
    pub kind: JobKind,
    pub source: Endpoint,
    pub destination: Endpoint,
    pub enabled: bool,
    #[serde(default)]
    pub runs: Vec<RunRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum RunOutcome {
    Running,
    Succeeded,
    Failed { message: String },
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub id: String,
    /// Unix time in seconds.
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub outcome: RunOutcome,
    pub files: u64,
    pub bytes: u64,
    pub errors: u64,
    /// File name inside the logs folder.
    pub log_file: String,
}

/// Live progress for a running job, shown on the Sync page.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub run_id: String,
    pub bytes: u64,
    pub total_bytes: u64,
    pub files: u64,
    pub errors: u64,
    pub speed: f64,
    pub eta_seconds: Option<u64>,
    pub current_file: Option<String>,
}

/// A job plus whether and how it is running right now.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    #[serde(flatten)]
    pub job: Job,
    pub progress: Option<Progress>,
}

/// Input for creating or editing a job.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInput {
    pub name: String,
    pub kind: JobKind,
    pub source: Endpoint,
    pub destination: Endpoint,
}

/// Checks the input and returns the trimmed name.
pub fn validate(input: &JobInput) -> crate::error::AppResult<String> {
    use crate::error::AppError;
    let name = input.name.trim();
    if name.is_empty() {
        return Err(AppError::invalid("Give this job a name."));
    }
    if name.chars().count() > 60 {
        return Err(AppError::invalid("Job names can be up to 60 characters."));
    }
    if let (Endpoint::Local { .. }, Endpoint::Local { .. }) = (&input.source, &input.destination) {
        return Err(AppError::invalid("A job needs a remote on one side. Orbit does not sync within this computer."));
    }
    if input.source == input.destination {
        return Err(AppError::invalid("The source and destination must be different folders."));
    }
    Ok(name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(source: Endpoint, destination: Endpoint) -> JobInput {
        JobInput {
            name: "  Photos  ".into(),
            kind: JobKind::Sync,
            source,
            destination,
        }
    }

    fn remote(name: &str, path: &str) -> Endpoint {
        Endpoint::Remote { remote: name.into(), path: path.into() }
    }

    #[test]
    fn accepts_remote_and_local_and_trims_name() {
        let job = input(remote("work", "Photos"), Endpoint::Local { path: "/home/ana/Photos".into() });
        assert_eq!(validate(&job).unwrap(), "Photos");
    }

    #[test]
    fn rejects_blank_long_and_identical_endpoints() {
        let mut blank = input(remote("a", ""), remote("b", ""));
        blank.name = "  ".into();
        assert!(validate(&blank).is_err());

        let mut long = input(remote("a", ""), remote("b", ""));
        long.name = "x".repeat(61);
        assert!(validate(&long).is_err());

        assert!(validate(&input(remote("a", "x"), remote("a", "x"))).is_err());
        let local_only = input(Endpoint::Local { path: "/a".into() }, Endpoint::Local { path: "/b".into() });
        assert!(validate(&local_only).is_err());
    }

    #[test]
    fn job_serializes_for_the_ui() {
        let job = Job {
            id: "j1".into(),
            name: "Photos".into(),
            kind: JobKind::Copy,
            source: remote("work", "Photos"),
            destination: Endpoint::Local { path: "/x".into() },
            enabled: true,
            runs: vec![],
        };
        let json = serde_json::to_value(&job).unwrap();
        assert_eq!(json["kind"], "copy");
        assert_eq!(json["source"]["kind"], "remote");
    }
}
