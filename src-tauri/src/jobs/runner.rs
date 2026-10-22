//! Runs jobs. Each run is an rclone async job; progress and new errors are polled and written to
//! a per-run log file. Finished runs are kept in `jobs.json`, up to `HISTORY_LIMIT` per job.

use super::store;
use super::{validate, Job, JobInput, JobView, Progress, RunOutcome, RunRecord, HISTORY_LIMIT};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::files::endpoint_fs;
use crate::paths::AppPaths;
use crate::rclone::Supervisor;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Longest log text returned to the UI. Older lines are dropped from the front.
const LOG_VIEW_LIMIT: usize = 200 * 1024;

pub type ChangeHook = Arc<dyn Fn() + Send + Sync>;

struct Running {
    rc_job_id: Option<u64>,
    progress: Progress,
    cancel_requested: bool,
}

struct State {
    jobs: Vec<Job>,
    running: HashMap<String, Running>,
}

pub struct Runner {
    supervisor: Supervisor,
    jobs_file: PathBuf,
    logs_dir: PathBuf,
    state: Mutex<State>,
    on_change: ChangeHook,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Stats {
    bytes: u64,
    total_bytes: u64,
    speed: f64,
    eta: Option<i64>,
    transfers: u64,
    errors: u64,
    last_error: String,
    transferring: Vec<Transferring>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Transferring {
    name: String,
}

#[derive(Deserialize)]
#[serde(default)]
struct JobStatus {
    finished: bool,
    success: bool,
    error: String,
}

impl Default for JobStatus {
    fn default() -> Self {
        Self { finished: false, success: false, error: String::new() }
    }
}

#[derive(Deserialize)]
struct JobReply {
    jobid: u64,
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn new_id(prefix: &str) -> String {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("{prefix}{millis:x}{:04x}", rand::random::<u16>())
}

impl Runner {
    /// Loads saved jobs. Runs that were still going when Orbit last closed are marked as failed.
    pub fn start(supervisor: Supervisor, paths: &AppPaths, on_change: impl Fn() + Send + Sync + 'static) -> AppResult<Arc<Self>> {
        let mut jobs = store::load(&paths.jobs())?;
        recover_interrupted(&mut jobs);
        let runner = Arc::new(Self {
            supervisor,
            jobs_file: paths.jobs(),
            logs_dir: paths.logs_dir(),
            state: Mutex::new(State { jobs, running: HashMap::new() }),
            on_change: Arc::new(on_change),
        });
        runner.persist()?;
        Ok(runner)
    }

    pub fn list(&self) -> Vec<JobView> {
        let state = self.state.lock().expect("jobs lock");
        state
            .jobs
            .iter()
            .map(|job| JobView {
                job: job.clone(),
                progress: state.running.get(&job.id).map(|running| running.progress.clone()),
            })
            .collect()
    }

    /// Number of jobs running right now. Used for the tray's "syncing" state.
    pub fn active(&self) -> usize {
        self.state.lock().expect("jobs lock").running.len()
    }

    pub fn create(&self, input: JobInput) -> AppResult<JobView> {
        let name = validate(&input)?;
        let job = Job {
            id: new_id("job"),
            name,
            kind: input.kind,
            source: input.source,
            destination: input.destination,
            enabled: true,
            runs: Vec::new(),
        };
        {
            let mut state = self.state.lock().expect("jobs lock");
            state.jobs.push(job.clone());
        }
        self.persist()?;
        self.notify();
        Ok(JobView { job, progress: None })
    }

    pub fn update(&self, id: &str, input: JobInput) -> AppResult<JobView> {
        let name = validate(&input)?;
        let view = {
            let mut state = self.state.lock().expect("jobs lock");
            if state.running.contains_key(id) {
                return Err(AppError::invalid("Stop this job before changing it."));
            }
            let job = find_mut(&mut state.jobs, id)?;
            job.name = name;
            job.kind = input.kind;
            job.source = input.source;
            job.destination = input.destination;
            JobView { job: job.clone(), progress: None }
        };
        self.persist()?;
        self.notify();
        Ok(view)
    }

    pub fn set_enabled(&self, id: &str, enabled: bool) -> AppResult<()> {
        {
            let mut state = self.state.lock().expect("jobs lock");
            find_mut(&mut state.jobs, id)?.enabled = enabled;
        }
        self.persist()?;
        self.notify();
        Ok(())
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        {
            let mut state = self.state.lock().expect("jobs lock");
            if state.running.contains_key(id) {
                return Err(AppError::invalid("Stop this job before deleting it."));
            }
            let before = state.jobs.len();
            state.jobs.retain(|job| job.id != id);
            if state.jobs.len() == before {
                return Err(AppError::invalid("That job no longer exists."));
            }
        }
        self.persist()?;
        self.notify();
        Ok(())
    }

    /// Starts a run in the background and returns its id.
    pub fn run(self: &Arc<Self>, id: &str) -> AppResult<String> {
        let run_id = new_id("run");
        let job = {
            let mut state = self.state.lock().expect("jobs lock");
            if state.running.contains_key(id) {
                return Err(AppError::invalid("This job is already running."));
            }
            let job = state.jobs.iter().find(|job| job.id == id).cloned().ok_or_else(|| AppError::invalid("That job no longer exists."))?;
            state.running.insert(
                id.to_owned(),
                Running {
                    rc_job_id: None,
                    progress: Progress { run_id: run_id.clone(), ..Progress::default() },
                    cancel_requested: false,
                },
            );
            job
        };
        self.notify();
        let runner = self.clone();
        let run_for_task = run_id.clone();
        tauri::async_runtime::spawn(async move {
            runner.execute(job, run_for_task).await;
        });
        Ok(run_id)
    }

    pub async fn cancel(&self, id: &str) -> AppResult<()> {
        let rc_job = {
            let mut state = self.state.lock().expect("jobs lock");
            let running = state.running.get_mut(id).ok_or_else(|| AppError::invalid("This job is not running."))?;
            running.cancel_requested = true;
            running.rc_job_id
        };
        if let Some(rc_job) = rc_job {
            self.supervisor.client()?.call_value("job/stop", &json!({ "jobid": rc_job })).await?;
        }
        Ok(())
    }

    /// Returns the end of a run's log, so the UI can show recent lines without loading huge files.
    pub fn read_log(&self, job_id: &str, run_id: &str) -> AppResult<String> {
        let file_name = {
            let state = self.state.lock().expect("jobs lock");
            let job = state.jobs.iter().find(|job| job.id == job_id).ok_or_else(|| AppError::invalid("That job no longer exists."))?;
            job.runs
                .iter()
                .find(|run| run.id == run_id)
                .map(|run| run.log_file.clone())
                .ok_or_else(|| AppError::invalid("That run is no longer in the history."))?
        };
        let text = fs::read_to_string(self.logs_dir.join(&file_name))
            .map_err(|err| AppError::new(ErrorKind::Io, format!("Could not read the log: {err}")))?;
        if text.len() <= LOG_VIEW_LIMIT {
            return Ok(text);
        }
        let mut start = text.len() - LOG_VIEW_LIMIT;
        while !text.is_char_boundary(start) {
            start += 1;
        }
        Ok(format!("…earlier lines are in the log file…\n{}", &text[start..]))
    }

    async fn execute(self: Arc<Self>, job: Job, run_id: String) {
        let started_at = now_secs();
        let started = Instant::now();
        let log_file = format!("job-{}-{}.log", job.id, run_id);
        let log_path = self.logs_dir.join(&log_file);
        let mut log = RunLog::open(log_path, started);
        log.line(&format!(
            "{} job \"{}\" from {} to {}",
            match job.kind {
                super::JobKind::Sync => "Sync",
                super::JobKind::Copy => "Copy",
            },
            job.name,
            describe(&job.source),
            describe(&job.destination),
        ));

        let outcome = self.drive(&job, &run_id, &mut log).await;
        let (outcome, files, bytes, errors) = match outcome {
            Ok(totals) => totals,
            Err(err) => (RunOutcome::Failed { message: err.message }, 0, 0, 0),
        };
        let outcome = {
            let state = self.state.lock().expect("jobs lock");
            match (state.running.get(&job.id).map(|r| r.cancel_requested), outcome) {
                (Some(true), _) => RunOutcome::Cancelled,
                (_, outcome) => outcome,
            }
        };
        log.line(&match &outcome {
            RunOutcome::Succeeded => "Finished. Everything was transferred.".to_owned(),
            RunOutcome::Cancelled => "Stopped by you.".to_owned(),
            RunOutcome::Failed { message } => format!("Finished with a problem: {message}"),
            RunOutcome::Running => "Finished.".to_owned(),
        });

        let record = RunRecord {
            id: run_id,
            started_at,
            finished_at: Some(now_secs()),
            outcome,
            files,
            bytes,
            errors,
            log_file,
        };
        self.finish(&job.id, record);
    }

    /// Runs the rc job and follows its progress until it ends. Returns (outcome, files, bytes, errors).
    async fn drive(&self, job: &Job, run_id: &str, log: &mut RunLog) -> AppResult<(RunOutcome, u64, u64, u64)> {
        let client = self.supervisor.client()?;
        let (endpoint, mut body) = match job.kind {
            super::JobKind::Sync => ("sync/sync", json!({})),
            super::JobKind::Copy => ("sync/copy", json!({})),
        };
        let group = format!("job-{run_id}");
        body["srcFs"] = json!(endpoint_fs(&job.source));
        body["dstFs"] = json!(endpoint_fs(&job.destination));
        body["_async"] = json!(true);
        body["_group"] = json!(group);

        let reply: JobReply = client.call(endpoint, &body).await?;
        self.update_running(&job.id, |running| running.rc_job_id = Some(reply.jobid));

        let mut seen_errors: Vec<String> = Vec::new();
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            let status: JobStatus = client.call("job/status", &json!({ "jobid": reply.jobid })).await?;
            let stats: Stats = client.call("core/stats", &json!({ "group": group })).await.unwrap_or_default();

            if !stats.last_error.is_empty() && !seen_errors.contains(&stats.last_error) {
                log.line(&format!("ERROR: {}", stats.last_error));
                seen_errors.push(stats.last_error.clone());
            }
            let current = stats.transferring.first().map(|t| t.name.clone());
            self.update_running(&job.id, |running| {
                running.progress.bytes = stats.bytes;
                running.progress.total_bytes = stats.total_bytes;
                running.progress.files = stats.transfers;
                running.progress.errors = stats.errors;
                running.progress.speed = stats.speed;
                running.progress.eta_seconds = stats.eta.and_then(|eta| u64::try_from(eta).ok());
                running.progress.current_file = current.clone();
            });
            if status.finished {
                let outcome = if status.success {
                    RunOutcome::Succeeded
                } else {
                    let message = if status.error.is_empty() { "rclone reported a problem.".to_owned() } else { status.error };
                    RunOutcome::Failed { message }
                };
                return Ok((outcome, stats.transfers, stats.bytes, stats.errors));
            }
        }
    }

    fn update_running(&self, id: &str, change: impl FnOnce(&mut Running)) {
        {
            let mut state = self.state.lock().expect("jobs lock");
            if let Some(running) = state.running.get_mut(id) {
                change(running);
            }
        }
        self.notify();
    }

    fn finish(&self, job_id: &str, record: RunRecord) {
        let removed_logs = {
            let mut state = self.state.lock().expect("jobs lock");
            state.running.remove(job_id);
            match state.jobs.iter_mut().find(|job| job.id == job_id) {
                Some(job) => {
                    job.runs.insert(0, record);
                    trim_history(&mut job.runs)
                }
                None => Vec::new(),
            }
        };
        for file in removed_logs {
            let _ = fs::remove_file(self.logs_dir.join(file));
        }
        if let Err(err) = self.persist() {
            eprintln!("Orbit: could not save job history: {}", err.message);
        }
        self.notify();
    }

    fn persist(&self) -> AppResult<()> {
        let jobs = self.state.lock().expect("jobs lock").jobs.clone();
        store::save(&self.jobs_file, &jobs)
    }

    fn notify(&self) {
        (self.on_change)();
    }
}

/// Keeps the newest `HISTORY_LIMIT` runs and returns the log files of the ones dropped.
pub fn trim_history(runs: &mut Vec<RunRecord>) -> Vec<String> {
    if runs.len() <= HISTORY_LIMIT {
        return Vec::new();
    }
    runs.drain(HISTORY_LIMIT..).map(|old| old.log_file).collect()
}

/// A run still marked running when Orbit starts again was cut short by a crash or a forced quit.
pub fn recover_interrupted(jobs: &mut [Job]) {
    for run in jobs.iter_mut().flat_map(|job| job.runs.iter_mut()) {
        if run.outcome == RunOutcome::Running {
            run.outcome = RunOutcome::Failed { message: "Orbit closed before this run finished.".into() };
            run.finished_at = Some(run.started_at);
        }
    }
}

fn find_mut<'a>(jobs: &'a mut [Job], id: &str) -> AppResult<&'a mut Job> {
    jobs.iter_mut()
        .find(|job| job.id == id)
        .ok_or_else(|| AppError::invalid("That job no longer exists."))
}

fn describe(endpoint: &crate::files::Endpoint) -> String {
    match endpoint {
        crate::files::Endpoint::Remote { remote, path } if path.is_empty() => format!("{remote}:"),
        crate::files::Endpoint::Remote { remote, path } => format!("{remote}:{path}"),
        crate::files::Endpoint::Local { path } => path.clone(),
    }
}

/// Appends timestamped lines to a run's log. Failures to write are ignored: the run itself matters more.
struct RunLog {
    file: Option<fs::File>,
    started: Instant,
}

impl RunLog {
    fn open(path: PathBuf, started: Instant) -> Self {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let file = OpenOptions::new().create(true).append(true).open(path).ok();
        Self { file, started }
    }

    fn line(&mut self, text: &str) {
        if let Some(file) = self.file.as_mut() {
            let _ = writeln!(file, "[+{:>5}s] {text}", self.started.elapsed().as_secs());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Endpoint;
    use crate::jobs::{RunOutcome, RunRecord};

    #[test]
    fn describes_endpoints_for_the_log() {
        assert_eq!(describe(&Endpoint::Remote { remote: "w".into(), path: String::new() }), "w:");
        assert_eq!(describe(&Endpoint::Remote { remote: "w".into(), path: "Photos".into() }), "w:Photos");
        assert_eq!(describe(&Endpoint::Local { path: "/home/ana".into() }), "/home/ana");
    }

    fn run(id: &str, outcome: RunOutcome) -> RunRecord {
        RunRecord {
            id: id.into(),
            started_at: 1,
            finished_at: None,
            outcome,
            files: 0,
            bytes: 0,
            errors: 0,
            log_file: format!("job-x-{id}.log"),
        }
    }

    #[test]
    fn history_keeps_the_newest_runs_only() {
        let mut runs: Vec<RunRecord> = (0..HISTORY_LIMIT + 2)
            .map(|i| run(&i.to_string(), RunOutcome::Succeeded))
            .collect();
        let dropped = trim_history(&mut runs);
        assert_eq!(runs.len(), HISTORY_LIMIT);
        assert_eq!(dropped, vec![format!("job-x-{}.log", HISTORY_LIMIT), format!("job-x-{}.log", HISTORY_LIMIT + 1)]);
    }

    #[test]
    fn interrupted_runs_are_marked_failed_on_restart() {
        let mut jobs = vec![Job {
            id: "j".into(),
            name: "n".into(),
            kind: super::super::JobKind::Copy,
            source: Endpoint::Local { path: "/a".into() },
            destination: Endpoint::Local { path: "/b".into() },
            enabled: true,
            runs: vec![run("1", RunOutcome::Running), run("2", RunOutcome::Succeeded)],
        }];
        recover_interrupted(&mut jobs);
        assert!(matches!(jobs[0].runs[0].outcome, RunOutcome::Failed { .. }));
        assert_eq!(jobs[0].runs[1].outcome, RunOutcome::Succeeded);
    }

    #[test]
    fn ids_are_unique_enough_and_prefixed() {
        let a = new_id("job");
        let b = new_id("job");
        assert!(a.starts_with("job"));
        assert_ne!(a, b);
    }
}
