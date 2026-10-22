//! When scheduled jobs run. Schedules only fire while Orbit is running: there is no catch-up for
//! runs missed while it was closed, and nothing runs on login.
//!
//! The clock for a job is its *anchor*: the later of the moment its schedule was armed (Orbit
//! start, or the job being saved or enabled) and the start of its newest run. "Every N minutes"
//! counts from the anchor; "daily at a time" picks the first such time after the anchor.

use super::{Job, Schedule};
use chrono::{DateTime, Days, Local, LocalResult, NaiveDateTime, NaiveTime, TimeZone};
use std::time::Duration;

/// How often the runner looks for due jobs.
pub const TICK: Duration = Duration::from_secs(15);

/// Longest interval accepted for "every N minutes": seven days.
pub const MAX_INTERVAL_MINUTES: u32 = 7 * 24 * 60;

/// Unix time of the job's next automatic run, or `None` for manual or disabled jobs.
pub fn next_run_at(job: &Job, armed_at: u64) -> Option<u64> {
    if !job.enabled {
        return None;
    }
    next_after(job.schedule, anchor(job, armed_at))
}

/// Unix time of the first run after `anchor` for this schedule.
pub fn next_after(schedule: Schedule, anchor: u64) -> Option<u64> {
    match schedule {
        Schedule::Manual => None,
        Schedule::Every { minutes } => Some(anchor + u64::from(minutes) * 60),
        Schedule::Daily { hour, minute } => {
            let local = Local
                .timestamp_opt(i64::try_from(anchor).ok()?, 0)
                .single()?;
            let next = next_daily(local.naive_local(), u32::from(hour), u32::from(minute));
            Some(u64::try_from(local_to_utc(next).timestamp()).ok()?)
        }
    }
}

/// Later of the schedule's arming time and the start of the newest run.
pub fn anchor(job: &Job, armed_at: u64) -> u64 {
    job.runs
        .iter()
        .map(|run| run.started_at)
        .max()
        .map_or(armed_at, |latest| latest.max(armed_at))
}

/// First `hour:minute` strictly after `anchor`, in the same (local) calendar.
pub fn next_daily(anchor: NaiveDateTime, hour: u32, minute: u32) -> NaiveDateTime {
    let time = NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or_default();
    let today = anchor.date().and_time(time);
    if today > anchor {
        today
    } else {
        today.checked_add_days(Days::new(1)).unwrap_or(today)
    }
}

/// Resolves a local wall-clock time, picking the earlier reading when clocks go back and the
/// next valid instant when they go forward.
fn local_to_utc(naive: NaiveDateTime) -> DateTime<Local> {
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => t,
        LocalResult::None => {
            let later = naive + chrono::Duration::hours(1);
            Local
                .from_local_datetime(&later)
                .earliest()
                .unwrap_or_else(|| Local.timestamp_opt(0, 0).single().expect("epoch"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Endpoint;
    use crate::jobs::{JobKind, RunOutcome, RunRecord};
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
    }

    fn job(schedule: Schedule, run_starts: &[u64]) -> Job {
        Job {
            id: "j".into(),
            name: "n".into(),
            kind: JobKind::Copy,
            source: Endpoint::Local { path: "/a".into() },
            destination: Endpoint::Remote {
                remote: "w".into(),
                path: String::new(),
            },
            enabled: true,
            schedule,
            runs: run_starts
                .iter()
                .map(|start| RunRecord {
                    id: start.to_string(),
                    started_at: *start,
                    finished_at: Some(*start),
                    outcome: RunOutcome::Succeeded,
                    files: 0,
                    bytes: 0,
                    errors: 0,
                    log_file: String::new(),
                    scheduled: false,
                })
                .collect(),
        }
    }

    #[test]
    fn daily_picks_today_when_the_time_is_still_ahead() {
        assert_eq!(
            next_daily(at(2026, 3, 10, 8, 50), 9, 0),
            at(2026, 3, 10, 9, 0)
        );
    }

    #[test]
    fn daily_rolls_to_tomorrow_once_the_time_has_passed() {
        assert_eq!(
            next_daily(at(2026, 3, 10, 9, 0), 9, 0),
            at(2026, 3, 11, 9, 0)
        );
        assert_eq!(
            next_daily(at(2026, 3, 10, 23, 59), 0, 30),
            at(2026, 3, 11, 0, 30)
        );
        // Month and year boundaries.
        assert_eq!(
            next_daily(at(2026, 12, 31, 22, 0), 7, 15),
            at(2027, 1, 1, 7, 15)
        );
    }

    #[test]
    fn every_counts_from_the_anchor() {
        assert_eq!(
            next_after(Schedule::Every { minutes: 30 }, 1_000),
            Some(1_000 + 30 * 60)
        );
        assert_eq!(next_after(Schedule::Manual, 1_000), None);
    }

    #[test]
    fn anchor_is_the_newest_run_or_the_arming_time() {
        let never_ran = job(Schedule::Every { minutes: 5 }, &[]);
        assert_eq!(anchor(&never_ran, 500), 500);
        let ran = job(Schedule::Every { minutes: 5 }, &[100, 400, 250]);
        assert_eq!(anchor(&ran, 50), 400);
        assert_eq!(
            anchor(&ran, 900),
            900,
            "re-arming after a run resets the clock"
        );
    }

    #[test]
    fn next_run_follows_enabled_and_schedule() {
        let mut every = job(Schedule::Every { minutes: 10 }, &[1_000]);
        assert_eq!(next_run_at(&every, 0), Some(1_600));
        every.enabled = false;
        assert_eq!(next_run_at(&every, 0), None);
        assert_eq!(next_run_at(&job(Schedule::Manual, &[1_000]), 0), None);
    }

    #[test]
    fn daily_next_run_is_in_the_future_and_at_the_right_wall_time() {
        let now = crate::jobs::runner::now_secs();
        let daily = job(
            Schedule::Daily {
                hour: 6,
                minute: 45,
            },
            &[],
        );
        let next = next_run_at(&daily, now).unwrap();
        assert!(next > now && next <= now + 24 * 60 * 60);
        let local = Local.timestamp_opt(next as i64, 0).single().unwrap();
        assert_eq!(
            (
                chrono::Timelike::hour(&local),
                chrono::Timelike::minute(&local)
            ),
            (6, 45)
        );
    }
}
