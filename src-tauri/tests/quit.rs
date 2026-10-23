//! Integration test for the clean stop behind "Quit Orbit": running work is cancelled, the engine
//! is stopped, and nothing is left listening.

mod common;

use common::spec;
use orbit_lib::files::Endpoint;
use orbit_lib::files::transfers::{Mode, TransferQueue, TransferRequest};
use orbit_lib::jobs::runner::Runner;
use orbit_lib::paths::AppPaths;
use orbit_lib::quit;
use orbit_lib::rclone::{RcdState, Supervisor};
use orbit_lib::remotes::store as remote_store;
use std::collections::BTreeMap;
use std::fs;
use std::net::TcpStream;
use std::time::{Duration, Instant};

#[tokio::test]
async fn quit_cancels_work_stops_the_engine_and_frees_its_port() {
    let Some(binary) = common::find_rclone() else {
        eprintln!("skipping: no rclone found (set ORBIT_TEST_RCLONE)");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let paths = AppPaths::new(root.join("app"));
    let supervisor = Supervisor::start(spec(binary, &root.join("config")), |_| {});
    let deadline = Instant::now() + Duration::from_secs(30);
    while !matches!(supervisor.state(), RcdState::Healthy { .. }) {
        assert!(Instant::now() < deadline, "rclone never became ready");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    let client = supervisor.client().unwrap();
    let port = client.port();
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_ok(),
        "engine listens while running"
    );

    // A remote over a temp folder and a big file, so a transfer is still running when we quit.
    let remote_dir = root.join("remote");
    fs::create_dir_all(&remote_dir).unwrap();
    let params: BTreeMap<String, String> = [(
        "remote".to_owned(),
        remote_dir.to_string_lossy().into_owned(),
    )]
    .into();
    remote_store::create(&client, "orbit-dst", "alias", &params)
        .await
        .unwrap();
    let source = root.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("big.bin"), vec![7u8; 96 * 1024 * 1024]).unwrap();

    let queue = TransferQueue::start(supervisor.clone(), 2, |_| {});
    queue
        .enqueue(vec![TransferRequest {
            source: Endpoint::Local {
                path: source.to_string_lossy().into_owned(),
            },
            destination: Endpoint::Remote {
                remote: "orbit-dst".into(),
                path: String::new(),
            },
            mode: Mode::Copy,
            name: "big.bin".into(),
            is_dir: false,
        }])
        .unwrap();
    let runner = Runner::start(supervisor.clone(), &paths, || {}).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    while queue.active() == 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        quit::active_work(&queue, &runner) >= 1,
        "something is running before quit"
    );

    let started = Instant::now();
    let report = quit::stop_everything(&queue, &runner, &supervisor).await;
    assert!(report.cancelled_transfers >= 1, "{report:?}");
    assert!(report.engine_stopped, "{report:?}");
    assert_eq!(supervisor.state(), RcdState::Stopped);
    assert!(
        started.elapsed() < quit::WATCHDOG,
        "quit must finish well inside the watchdog, took {:?}",
        started.elapsed()
    );
    assert!(
        TcpStream::connect(("127.0.0.1", port)).is_err(),
        "nothing listens on the engine's port after quit"
    );
    assert_eq!(
        quit::active_work(&queue, &runner),
        0,
        "nothing is left running"
    );
}
