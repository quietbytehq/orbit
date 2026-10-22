//! Transfer queue. Each transfer runs as an async rclone job, and progress is read from the
//! job's stats group. Up to `limit` transfers run at once; the rest wait in order.

use super::{join, local_fs, remote_fs, Endpoint};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::Supervisor;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

const POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    Copy,
    Move,
}

/// What the user asked for. The item is `name`, found in the source folder, and it lands in the
/// destination folder under the same name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRequest {
    pub source: Endpoint,
    pub destination: Endpoint,
    pub mode: Mode,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Upload,
    Download,
    Copy,
    Move,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum State {
    Queued,
    Running,
    Done,
    Failed { message: String },
    Cancelled,
}

/// A transfer as the UI sees it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: u64,
    pub name: String,
    pub direction: Direction,
    pub is_dir: bool,
    pub from: String,
    pub to: String,
    pub state: State,
    pub bytes: u64,
    pub total_bytes: u64,
    pub speed: f64,
    pub eta_seconds: Option<u64>,
}

/// One rc call that performs a transfer, before `_async` and `_group` are added.
#[derive(Debug, Clone, PartialEq)]
pub struct RcCall {
    pub endpoint: &'static str,
    pub body: Value,
}

pub fn direction(request: &TransferRequest) -> AppResult<Direction> {
    match (&request.source, &request.destination, request.mode) {
        (Endpoint::Local { .. }, Endpoint::Remote { .. }, Mode::Copy) => Ok(Direction::Upload),
        (Endpoint::Remote { .. }, Endpoint::Local { .. }, Mode::Copy) => Ok(Direction::Download),
        (Endpoint::Remote { .. }, Endpoint::Remote { .. }, Mode::Copy) => Ok(Direction::Copy),
        (Endpoint::Remote { .. }, Endpoint::Remote { .. }, Mode::Move) => Ok(Direction::Move),
        (Endpoint::Local { .. }, _, Mode::Move) => Err(AppError::invalid("Files on this computer are uploaded, not moved.")),
        (Endpoint::Remote { .. }, Endpoint::Local { .. }, Mode::Move) => {
            Err(AppError::invalid("Downloads copy files. Delete the remote copy yourself if you want to move it."))
        }
        (Endpoint::Local { .. }, Endpoint::Local { .. }, _) => {
            Err(AppError::invalid("Orbit copies between this computer and a remote, not within this computer."))
        }
    }
}

/// Filesystem string for a folder, or for an item inside it when `child` is given.
fn fs_of(endpoint: &Endpoint, child: Option<&str>) -> String {
    match endpoint {
        Endpoint::Remote { remote, path } => match child {
            Some(child) => remote_fs(remote, &join(path, child)),
            None => remote_fs(remote, path),
        },
        Endpoint::Local { path } => match child {
            Some(child) => local_fs(&Path::new(path).join(child).to_string_lossy()),
            None => local_fs(path),
        },
    }
}

/// Builds the rc call for a request. Files use copyfile or movefile; folders use sync/copy or sync/move.
pub fn build_call(request: &TransferRequest) -> AppResult<RcCall> {
    direction(request)?;
    let is_move = request.mode == Mode::Move;
    if request.name.trim().is_empty() || request.name.contains('/') || request.name.contains('\\') {
        return Err(AppError::invalid("Choose a single file or folder to transfer."));
    }
    if request.is_dir {
        let endpoint = if is_move { "sync/move" } else { "sync/copy" };
        let mut body = json!({
            "srcFs": fs_of(&request.source, Some(&request.name)),
            "dstFs": fs_of(&request.destination, Some(&request.name)),
        });
        if is_move {
            body["deleteEmptySrcDirs"] = json!(true);
        }
        return Ok(RcCall { endpoint, body });
    }
    let endpoint = if is_move { "operations/movefile" } else { "operations/copyfile" };
    Ok(RcCall {
        endpoint,
        body: json!({
            "srcFs": fs_of(&request.source, None),
            "srcRemote": request.name,
            "dstFs": fs_of(&request.destination, None),
            "dstRemote": request.name,
        }),
    })
}

fn label(endpoint: &Endpoint) -> String {
    match endpoint {
        Endpoint::Remote { remote, path } => format!("{remote}:/{path}").trim_end_matches('/').to_owned(),
        Endpoint::Local { path } => path.clone(),
    }
}

#[derive(Deserialize)]
struct JobReply {
    jobid: u64,
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

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Stats {
    bytes: u64,
    total_bytes: u64,
    speed: f64,
    eta: Option<i64>,
}

struct Entry {
    item: Item,
    request: TransferRequest,
    job_id: Option<u64>,
    cancel_requested: bool,
}

#[derive(Default)]
struct Inner {
    next_id: u64,
    limit: usize,
    running: usize,
    entries: Vec<Entry>,
}

pub type ChangeHook = Arc<dyn Fn(&[Item]) + Send + Sync>;

pub struct TransferQueue {
    supervisor: Supervisor,
    inner: Mutex<Inner>,
    wake: Notify,
    on_change: ChangeHook,
}

impl TransferQueue {
    /// Starts the dispatcher on Tauri's async runtime.
    pub fn start(supervisor: Supervisor, limit: usize, on_change: impl Fn(&[Item]) + Send + Sync + 'static) -> Arc<Self> {
        let queue = Arc::new(Self {
            supervisor,
            inner: Mutex::new(Inner { next_id: 1, limit: limit.max(1), ..Inner::default() }),
            wake: Notify::new(),
            on_change: Arc::new(on_change),
        });
        tauri::async_runtime::spawn(dispatch(queue.clone()));
        queue
    }

    /// Validates every request first. If one is invalid, nothing is queued.
    pub fn enqueue(&self, requests: Vec<TransferRequest>) -> AppResult<Vec<u64>> {
        for request in &requests {
            build_call(request)?;
        }
        let ids = {
            let mut inner = self.inner.lock().expect("queue lock");
            let mut ids = Vec::with_capacity(requests.len());
            for request in requests {
                let id = inner.next_id;
                inner.next_id += 1;
                let direction = direction(&request).expect("validated above");
                inner.entries.push(Entry {
                    item: Item {
                        id,
                        name: request.name.clone(),
                        direction,
                        is_dir: request.is_dir,
                        from: label(&request.source),
                        to: label(&request.destination),
                        state: State::Queued,
                        bytes: 0,
                        total_bytes: 0,
                        speed: 0.0,
                        eta_seconds: None,
                    },
                    request,
                    job_id: None,
                    cancel_requested: false,
                });
                ids.push(id);
            }
            ids
        };
        self.wake.notify_one();
        self.emit();
        Ok(ids)
    }

    pub fn snapshot(&self) -> Vec<Item> {
        self.inner.lock().expect("queue lock").entries.iter().map(|e| e.item.clone()).collect()
    }

    /// Transfers that are running now. Used for the tray's "syncing" state.
    pub fn active(&self) -> usize {
        self.inner.lock().expect("queue lock").running
    }

    pub fn set_limit(&self, limit: usize) {
        self.inner.lock().expect("queue lock").limit = limit.max(1);
        self.wake.notify_one();
    }

    pub async fn cancel(&self, id: u64) -> AppResult<()> {
        let job_id = {
            let mut inner = self.inner.lock().expect("queue lock");
            let Some(entry) = inner.entries.iter_mut().find(|e| e.item.id == id) else {
                return Err(AppError::new(ErrorKind::InvalidInput, "That transfer is no longer in the queue."));
            };
            match entry.item.state {
                State::Queued => {
                    entry.item.state = State::Cancelled;
                    None
                }
                State::Running => {
                    entry.cancel_requested = true;
                    entry.job_id
                }
                _ => None,
            }
        };
        self.emit();
        if let Some(job_id) = job_id {
            self.supervisor.client()?.call_value("job/stop", &json!({ "jobid": job_id })).await?;
        }
        Ok(())
    }

    /// Removes finished, failed and cancelled transfers from the list.
    pub fn clear_finished(&self) {
        self.inner
            .lock()
            .expect("queue lock")
            .entries
            .retain(|e| matches!(e.item.state, State::Queued | State::Running));
        self.emit();
    }

    fn emit(&self) {
        let snapshot = self.snapshot();
        (self.on_change)(&snapshot);
    }

    /// Marks the first queued transfer as running, if a slot is free.
    fn claim_next(&self) -> Option<(u64, TransferRequest)> {
        let mut inner = self.inner.lock().expect("queue lock");
        if inner.running >= inner.limit {
            return None;
        }
        let entry = inner.entries.iter_mut().find(|e| e.item.state == State::Queued)?;
        entry.item.state = State::Running;
        let claimed = (entry.item.id, entry.request.clone());
        inner.running += 1;
        Some(claimed)
    }

    async fn run(self: Arc<Self>, id: u64, request: TransferRequest) {
        let outcome = self.execute(id, &request).await;
        {
            let mut inner = self.inner.lock().expect("queue lock");
            inner.running -= 1;
            if let Some(entry) = inner.entries.iter_mut().find(|e| e.item.id == id) {
                entry.job_id = None;
                entry.item.state = match outcome {
                    Ok(()) => {
                        entry.item.bytes = entry.item.total_bytes.max(entry.item.bytes);
                        State::Done
                    }
                    Err(_) if entry.cancel_requested => State::Cancelled,
                    Err(err) => State::Failed { message: err.message },
                };
                entry.item.speed = 0.0;
                entry.item.eta_seconds = None;
            }
        }
        self.wake.notify_one();
        self.emit();
    }

    async fn execute(&self, id: u64, request: &TransferRequest) -> AppResult<()> {
        let client = self.supervisor.client()?;
        let call = build_call(request)?;
        let group = format!("transfer-{id}");
        let mut body = call.body;
        body["_async"] = json!(true);
        body["_group"] = json!(group);

        let reply: JobReply = client.call(call.endpoint, &body).await?;
        self.update(id, |entry| entry.job_id = Some(reply.jobid));

        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            let status: JobStatus = client.call("job/status", &json!({ "jobid": reply.jobid })).await?;
            let stats: Stats = client.call("core/stats", &json!({ "group": group })).await.unwrap_or_default();
            self.update(id, |entry| {
                entry.item.bytes = stats.bytes;
                entry.item.total_bytes = stats.total_bytes;
                entry.item.speed = stats.speed;
                entry.item.eta_seconds = stats.eta.and_then(|eta| u64::try_from(eta).ok());
            });
            self.emit();
            if status.finished {
                return if status.success {
                    Ok(())
                } else {
                    let message = if status.error.is_empty() { "The transfer failed.".to_owned() } else { status.error };
                    Err(AppError::new(ErrorKind::Rclone, message))
                };
            }
        }
    }

    fn update(&self, id: u64, change: impl FnOnce(&mut Entry)) {
        let mut inner = self.inner.lock().expect("queue lock");
        if let Some(entry) = inner.entries.iter_mut().find(|e| e.item.id == id) {
            change(entry);
        }
    }
}

async fn dispatch(queue: Arc<TransferQueue>) {
    loop {
        queue.wake.notified().await;
        while let Some((id, request)) = queue.claim_next() {
            tauri::async_runtime::spawn(queue.clone().run(id, request));
        }
        queue.emit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(name: &str, path: &str) -> Endpoint {
        Endpoint::Remote { remote: name.into(), path: path.into() }
    }

    fn local(path: &str) -> Endpoint {
        Endpoint::Local { path: path.into() }
    }

    fn request(source: Endpoint, destination: Endpoint, mode: Mode, name: &str, is_dir: bool) -> TransferRequest {
        TransferRequest { source, destination, mode, name: name.into(), is_dir }
    }

    #[test]
    fn directions_follow_endpoints_and_mode() {
        let up = request(local("/home/ana"), remote("work", "Photos"), Mode::Copy, "a.jpg", false);
        assert_eq!(direction(&up).unwrap(), Direction::Upload);
        let down = request(remote("work", "Photos"), local("/home/ana"), Mode::Copy, "a.jpg", false);
        assert_eq!(direction(&down).unwrap(), Direction::Download);
        let copy = request(remote("a", ""), remote("b", ""), Mode::Copy, "x", false);
        assert_eq!(direction(&copy).unwrap(), Direction::Copy);
        let mv = request(remote("a", ""), remote("b", ""), Mode::Move, "x", false);
        assert_eq!(direction(&mv).unwrap(), Direction::Move);
    }

    #[test]
    fn unsupported_combinations_are_rejected() {
        let local_move = request(local("/x"), remote("a", ""), Mode::Move, "f", false);
        assert!(build_call(&local_move).is_err());
        let download_move = request(remote("a", ""), local("/x"), Mode::Move, "f", false);
        assert!(build_call(&download_move).is_err());
        let local_local = request(local("/x"), local("/y"), Mode::Copy, "f", false);
        assert!(build_call(&local_local).is_err());
    }

    #[test]
    fn file_upload_uses_copyfile_with_parent_folders() {
        let call = build_call(&request(local("/home/ana/Pictures"), remote("work", "Photos/2024"), Mode::Copy, "cat.jpg", false)).unwrap();
        assert_eq!(call.endpoint, "operations/copyfile");
        assert_eq!(call.body["srcFs"], ":local:/home/ana/Pictures");
        assert_eq!(call.body["srcRemote"], "cat.jpg");
        assert_eq!(call.body["dstFs"], "work:Photos/2024");
        assert_eq!(call.body["dstRemote"], "cat.jpg");
    }

    #[test]
    fn folder_download_uses_sync_copy_on_full_paths() {
        let call = build_call(&request(remote("work", "Photos"), local("/home/ana"), Mode::Copy, "2024", true)).unwrap();
        assert_eq!(call.endpoint, "sync/copy");
        assert_eq!(call.body["srcFs"], "work:Photos/2024");
        assert_eq!(call.body["dstFs"], ":local:/home/ana/2024");
        assert!(call.body.get("deleteEmptySrcDirs").is_none());
    }

    #[test]
    fn folder_move_removes_empty_source_folders() {
        let call = build_call(&request(remote("a", "x"), remote("b", "y"), Mode::Move, "z", true)).unwrap();
        assert_eq!(call.endpoint, "sync/move");
        assert_eq!(call.body["srcFs"], "a:x/z");
        assert_eq!(call.body["dstFs"], "b:y/z");
        assert_eq!(call.body["deleteEmptySrcDirs"], true);
    }

    #[test]
    fn names_must_be_one_segment() {
        let bad = request(remote("a", ""), remote("b", ""), Mode::Copy, "x/y", false);
        assert!(build_call(&bad).is_err());
        let empty = request(remote("a", ""), remote("b", ""), Mode::Copy, " ", false);
        assert!(build_call(&empty).is_err());
    }

    #[test]
    fn labels_show_where_a_transfer_goes() {
        assert_eq!(label(&remote("work", "Photos")), "work:/Photos");
        assert_eq!(label(&remote("work", "")), "work:");
        assert_eq!(label(&local("/home/ana")), "/home/ana");
    }
}
