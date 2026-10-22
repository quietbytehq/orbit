//! Integration tests for browsing and transfers against a real `rclone rcd`.
//!
//! Remotes are `alias` remotes pointing at folders in a temp directory, so no cloud account is
//! needed and the results are easy to inspect on disk.

mod common;

use common::spec;
use orbit_lib::error::ErrorKind;
use orbit_lib::files::ops;
use orbit_lib::files::transfers::{Direction, Mode, State, TransferQueue, TransferRequest};
use orbit_lib::files::Endpoint;
use orbit_lib::remotes::store;
use orbit_lib::rclone::{RcdState, Supervisor};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    supervisor: Supervisor,
}

async fn fixture() -> Option<Fixture> {
    let binary = common::find_rclone().or_else(|| {
        eprintln!("skipping: no rclone found (set ORBIT_TEST_RCLONE)");
        None
    })?;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let supervisor = Supervisor::start(spec(binary, &root.join("config")), |_| {});
    wait_until(&supervisor, Duration::from_secs(30), |s| matches!(s, RcdState::Healthy { .. })).await;
    Some(Fixture { _dir: dir, root, supervisor })
}

async fn wait_until<F: Fn(&RcdState) -> bool>(supervisor: &Supervisor, timeout: Duration, pred: F) {
    let deadline = Instant::now() + timeout;
    while !pred(&supervisor.state()) {
        assert!(Instant::now() < deadline, "rclone never became ready: {:?}", supervisor.state());
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Creates an alias remote named `name` that points at `folder`.
async fn alias(fx: &Fixture, name: &str, folder: &Path) {
    fs::create_dir_all(folder).unwrap();
    let client = fx.supervisor.client().unwrap();
    let params: BTreeMap<String, String> = [("remote".to_owned(), folder.to_string_lossy().into_owned())].into();
    store::create(&client, name, "alias", &params).await.unwrap();
}

async fn wait_for_states(queue: &TransferQueue, ids: &[u64], timeout: Duration) -> Vec<State> {
    let deadline = Instant::now() + timeout;
    loop {
        let snapshot = queue.snapshot();
        let states: Vec<State> = ids
            .iter()
            .map(|id| snapshot.iter().find(|item| item.id == *id).expect("queued item").state.clone())
            .collect();
        let settled = states.iter().all(|s| matches!(s, State::Done | State::Failed { .. } | State::Cancelled));
        if settled || Instant::now() >= deadline {
            return states;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

fn request(source: Endpoint, destination: Endpoint, mode: Mode, name: &str, is_dir: bool) -> TransferRequest {
    TransferRequest { source, destination, mode, name: name.into(), is_dir }
}

fn remote(name: &str, path: &str) -> Endpoint {
    Endpoint::Remote { remote: name.into(), path: path.into() }
}

fn local(path: &Path) -> Endpoint {
    Endpoint::Local { path: path.to_string_lossy().into_owned() }
}

#[tokio::test]
async fn browse_create_rename_and_delete() {
    let Some(fx) = fixture().await else { return };
    let client = fx.supervisor.client().unwrap();
    alias(&fx, "orbit-a", &fx.root.join("remote-a")).await;

    assert!(ops::list(&client, "orbit-a", "").await.unwrap().is_empty());

    ops::create_folder(&client, "orbit-a", "Photos").await.unwrap();
    fs::write(fx.root.join("remote-a/notes.txt"), b"hi").unwrap();
    let entries = ops::list(&client, "orbit-a", "").await.unwrap();
    assert_eq!(entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), vec!["Photos", "notes.txt"]);
    assert!(entries[0].is_dir, "folders sort first");
    assert_eq!(entries[1].size, 2);
    assert_eq!(entries[1].path, "notes.txt");

    let renamed = ops::rename(&client, "orbit-a", "Photos", true, "Pictures").await.unwrap();
    assert_eq!(renamed, "Pictures");
    assert!(fx.root.join("remote-a/Pictures").is_dir());
    assert!(!fx.root.join("remote-a/Photos").exists());

    ops::delete(&client, "orbit-a", "notes.txt", false).await.unwrap();
    ops::delete(&client, "orbit-a", "Pictures", true).await.unwrap();
    assert!(ops::list(&client, "orbit-a", "").await.unwrap().is_empty());

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn transfers_upload_download_copy_and_move() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-a", &fx.root.join("remote-a")).await;
    alias(&fx, "orbit-b", &fx.root.join("remote-b")).await;
    let client = fx.supervisor.client().unwrap();
    ops::create_folder(&client, "orbit-a", "Photos").await.unwrap();

    let source = fx.root.join("source");
    fs::create_dir_all(source.join("sub")).unwrap();
    fs::write(source.join("a.txt"), b"alpha").unwrap();
    fs::write(source.join("sub/b.txt"), b"beta").unwrap();

    let queue = TransferQueue::start(fx.supervisor.clone(), 2, |_| {});

    // Upload a file into a folder on the remote.
    let ids = queue
        .enqueue(vec![request(local(&source), remote("orbit-a", "Photos"), Mode::Copy, "a.txt", false)])
        .unwrap();
    assert_eq!(wait_for_states(&queue, &ids, Duration::from_secs(60)).await, vec![State::Done]);
    assert_eq!(fs::read(fx.root.join("remote-a/Photos/a.txt")).unwrap(), b"alpha");
    let item = queue.snapshot().into_iter().find(|i| i.id == ids[0]).unwrap();
    assert_eq!(item.direction, Direction::Upload);
    assert_eq!(item.bytes, 5);

    // Upload a folder.
    let ids = queue
        .enqueue(vec![request(local(&source), remote("orbit-b", ""), Mode::Copy, "sub", true)])
        .unwrap();
    assert_eq!(wait_for_states(&queue, &ids, Duration::from_secs(60)).await, vec![State::Done]);
    assert_eq!(fs::read(fx.root.join("remote-b/sub/b.txt")).unwrap(), b"beta");

    // Download a file to a folder on this computer.
    let downloads = fx.root.join("downloads");
    fs::create_dir_all(&downloads).unwrap();
    let ids = queue
        .enqueue(vec![request(remote("orbit-a", "Photos"), local(&downloads), Mode::Copy, "a.txt", false)])
        .unwrap();
    assert_eq!(wait_for_states(&queue, &ids, Duration::from_secs(60)).await, vec![State::Done]);
    assert_eq!(fs::read(downloads.join("a.txt")).unwrap(), b"alpha");

    // Copy between two remotes, then move between two remotes.
    let ids = queue
        .enqueue(vec![request(remote("orbit-a", "Photos"), remote("orbit-b", "copies"), Mode::Copy, "a.txt", false)])
        .unwrap();
    assert_eq!(wait_for_states(&queue, &ids, Duration::from_secs(60)).await, vec![State::Done]);
    assert!(fx.root.join("remote-a/Photos/a.txt").exists());
    assert_eq!(fs::read(fx.root.join("remote-b/copies/a.txt")).unwrap(), b"alpha");

    let ids = queue
        .enqueue(vec![request(remote("orbit-b", "copies"), remote("orbit-a", ""), Mode::Move, "a.txt", false)])
        .unwrap();
    assert_eq!(wait_for_states(&queue, &ids, Duration::from_secs(60)).await, vec![State::Done]);
    assert!(fx.root.join("remote-a/a.txt").exists());
    assert!(!fx.root.join("remote-b/copies/a.txt").exists());

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn failed_transfer_reports_a_message() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-a", &fx.root.join("remote-a")).await;
    let downloads = fx.root.join("downloads");
    fs::create_dir_all(&downloads).unwrap();

    let queue = TransferQueue::start(fx.supervisor.clone(), 1, |_| {});
    let ids = queue
        .enqueue(vec![request(remote("orbit-a", "missing"), local(&downloads), Mode::Copy, "nope.txt", false)])
        .unwrap();
    let states = wait_for_states(&queue, &ids, Duration::from_secs(60)).await;
    match &states[0] {
        State::Failed { message } => assert!(!message.is_empty()),
        other => panic!("expected Failed, got {other:?}"),
    }

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn queued_transfer_can_be_cancelled_and_invalid_requests_queue_nothing() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-a", &fx.root.join("remote-a")).await;

    // A 64 MB file keeps the single slot busy long enough to cancel the one behind it.
    let source = fx.root.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("big.bin"), vec![7u8; 64 * 1024 * 1024]).unwrap();
    fs::write(source.join("small.txt"), b"small").unwrap();

    let queue = TransferQueue::start(fx.supervisor.clone(), 1, |_| {});
    let ids = queue
        .enqueue(vec![
            request(local(&source), remote("orbit-a", ""), Mode::Copy, "big.bin", false),
            request(local(&source), remote("orbit-a", ""), Mode::Copy, "small.txt", false),
        ])
        .unwrap();
    queue.cancel(ids[1]).await.unwrap();
    let states = wait_for_states(&queue, &ids, Duration::from_secs(120)).await;
    assert_eq!(states[0], State::Done);
    assert_eq!(states[1], State::Cancelled);
    assert!(!fx.root.join("remote-a/small.txt").exists(), "cancelled transfer must not run");

    // Invalid request: nothing from the batch is queued.
    let before = queue.snapshot().len();
    let err = queue
        .enqueue(vec![
            request(local(&source), remote("orbit-a", ""), Mode::Copy, "small.txt", false),
            request(local(&source), local(&source), Mode::Copy, "small.txt", false),
        ])
        .unwrap_err();
    assert_eq!(err.kind, ErrorKind::InvalidInput);
    assert_eq!(queue.snapshot().len(), before);

    queue.clear_finished();
    assert!(queue.snapshot().is_empty());
    fx.supervisor.shutdown().await;
}
