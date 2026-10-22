//! Saved sync and copy jobs. Each run is an rclone async job whose progress and errors are
//! recorded, with a log file per run.

pub mod commands;
pub mod runner;
pub mod schedule;
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

/// When a job runs on its own. Schedules fire only while Orbit is running; runs missed while it
/// was closed are not caught up, and a disabled job never runs on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Schedule {
    /// Only when the user presses "Run now".
    #[default]
    Manual,
    /// Every `minutes` minutes, counted from the newest run or from when the schedule was armed.
    Every { minutes: u32 },
    /// Once a day at this local time.
    Daily { hour: u8, minute: u8 },
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
    pub schedule: Schedule,
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
    /// True when the schedule started this run rather than the user.
    #[serde(default)]
    pub scheduled: bool,
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
    /// Unix time of the next automatic run. `None` for manual or disabled jobs.
    pub next_run_at: Option<u64>,
}

/// Input for creating or editing a job.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobInput {
    pub name: String,
    pub kind: JobKind,
    pub source: Endpoint,
    pub destination: Endpoint,
    #[serde(default)]
    pub schedule: Schedule,
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
        return Err(AppError::invalid(
            "A job needs a remote on one side. Orbit does not sync within this computer.",
        ));
    }
    if input.source == input.destination {
        return Err(AppError::invalid(
            "The source and destination must be different folders.",
        ));
    }
    validate_schedule(input.schedule)?;
    Ok(name.to_owned())
}

/// Rejects intervals outside one minute to seven days and times that do not exist.
pub fn validate_schedule(schedule: Schedule) -> crate::error::AppResult<()> {
    use crate::error::AppError;
    match schedule {
        Schedule::Manual => Ok(()),
        Schedule::Every { minutes: 0 } => Err(AppError::invalid("Run at least one minute apart.")),
        Schedule::Every { minutes } if minutes > schedule::MAX_INTERVAL_MINUTES => {
            Err(AppError::invalid(
                "Runs can be up to seven days apart. For longer, run the job by hand.",
            ))
        }
        Schedule::Every { .. } => Ok(()),
        Schedule::Daily { hour, minute } if hour > 23 || minute > 59 => Err(AppError::invalid(
            "Choose a time of day between 00:00 and 23:59.",
        )),
        Schedule::Daily { .. } => Ok(()),
    }
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
            schedule: Schedule::Manual,
        }
    }

    fn remote(name: &str, path: &str) -> Endpoint {
        Endpoint::Remote {
            remote: name.into(),
            path: path.into(),
        }
    }

    #[test]
    fn accepts_remote_and_local_and_trims_name() {
        let job = input(
            remote("work", "Photos"),
            Endpoint::Local {
                path: "/home/ana/Photos".into(),
            },
        );
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
        let local_only = input(
            Endpoint::Local { path: "/a".into() },
            Endpoint::Local { path: "/b".into() },
        );
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
            schedule: Schedule::Every { minutes: 30 },
            runs: vec![],
        };
        let json = serde_json::to_value(&job).unwrap();
        assert_eq!(json["kind"], "copy");
        assert_eq!(json["source"]["kind"], "remote");
        assert_eq!(json["schedule"]["kind"], "every");
        assert_eq!(json["schedule"]["minutes"], 30);
    }

    #[test]
    fn jobs_saved_before_schedules_existed_load_as_manual() {
        let json = r#"{"id":"j","name":"n","kind":"copy","source":{"kind":"local","path":"/a"},
            "destination":{"kind":"remote","remote":"w","path":""},"enabled":true,
            "runs":[{"id":"r","startedAt":1,"finishedAt":2,"outcome":{"state":"succeeded"},
            "files":0,"bytes":0,"errors":0,"logFile":"x.log"}]}"#;
        let job: Job = serde_json::from_str(json).unwrap();
        assert_eq!(job.schedule, Schedule::Manual);
        assert!(!job.runs[0].scheduled);
    }

    #[test]
    fn schedule_limits() {
        assert!(validate_schedule(Schedule::Manual).is_ok());
        assert!(validate_schedule(Schedule::Every { minutes: 1 }).is_ok());
        assert!(validate_schedule(Schedule::Every { minutes: 0 }).is_err());
        assert!(
            validate_schedule(Schedule::Every {
                minutes: 7 * 24 * 60
            })
            .is_ok()
        );
        assert!(
            validate_schedule(Schedule::Every {
                minutes: 7 * 24 * 60 + 1
            })
            .is_err()
        );
        assert!(
            validate_schedule(Schedule::Daily {
                hour: 23,
                minute: 59
            })
            .is_ok()
        );
        assert!(
            validate_schedule(Schedule::Daily {
                hour: 24,
                minute: 0
            })
            .is_err()
        );
        assert!(
            validate_schedule(Schedule::Daily {
                hour: 9,
                minute: 60
            })
            .is_err()
        );
        let mut input = input(remote("a", "x"), remote("b", "y"));
        input.schedule = Schedule::Every { minutes: 0 };
        assert!(validate(&input).is_err());
    }
}
