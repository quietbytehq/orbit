//! Integration tests for sync and copy jobs, and for mounting, against a real `rclone rcd`.
//!
//! Remotes are `alias` remotes over temp folders. Mount tests need FUSE and skip without it.

mod common;

use common::spec;
use orbit_lib::files::Endpoint;
use orbit_lib::jobs::runner::Runner;
use orbit_lib::jobs::store;
use orbit_lib::jobs::{Job, JobInput, JobKind, JobView, RunOutcome, Schedule};
use orbit_lib::mounts;
use orbit_lib::paths::AppPaths;
use orbit_lib::rclone::{RcdState, Supervisor};
use orbit_lib::remotes::store as remote_store;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    paths: AppPaths,
    supervisor: Supervisor,
}

async fn fixture() -> Option<Fixture> {
    let binary = common::find_rclone().or_else(|| {
        eprintln!("skipping: no rclone found (set ORBIT_TEST_RCLONE)");
        None
    })?;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let paths = AppPaths::new(root.join("app"));
    let supervisor = Supervisor::start(spec(binary, &root.join("config")), |_| {});
    let deadline = Instant::now() + Duration::from_secs(30);
    while !matches!(supervisor.state(), RcdState::Healthy { .. }) {
        assert!(
            Instant::now() < deadline,
            "rclone never became ready: {:?}",
            supervisor.state()
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Some(Fixture {
        _dir: dir,
        root,
        paths,
        supervisor,
    })
}

async fn alias(fx: &Fixture, name: &str, folder: &Path) {
    fs::create_dir_all(folder).unwrap();
    let client = fx.supervisor.client().unwrap();
    let params: BTreeMap<String, String> =
        [("remote".to_owned(), folder.to_string_lossy().into_owned())].into();
    remote_store::create(&client, name, "alias", &params)
        .await
        .unwrap();
}

fn remote(name: &str, path: &str) -> Endpoint {
    Endpoint::Remote {
        remote: name.into(),
        path: path.into(),
    }
}

fn local(path: &Path) -> Endpoint {
    Endpoint::Local {
        path: path.to_string_lossy().into_owned(),
    }
}

fn input(name: &str, kind: JobKind, source: Endpoint, destination: Endpoint) -> JobInput {
    JobInput {
        name: name.into(),
        kind,
        source,
        destination,
        schedule: Schedule::Manual,
    }
}

/// Polls until the job has no run in progress, then returns its newest finished run.
async fn wait_for_run(runner: &Runner, job_id: &str) -> (RunOutcome, u64, u64) {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let view = runner
            .list()
            .into_iter()
            .find(|v| v.job.id == job_id)
            .expect("job exists");
        if view.progress.is_none() {
            let run = view.job.runs.first().expect("a run was recorded");
            return (run.outcome.clone(), run.files, run.errors);
        }
        assert!(Instant::now() < deadline, "run did not finish");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn copy_adds_files_and_keeps_extras_while_sync_removes_them() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-dst", &fx.root.join("remote")).await;
    let source = fx.root.join("source");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("one.txt"), b"1").unwrap();
    fs::write(source.join("nested/two.txt"), b"2").unwrap();
    fs::create_dir_all(fx.root.join("remote/backup")).unwrap();
    fs::write(fx.root.join("remote/backup/extra.txt"), b"keep me?").unwrap();

    let runner = Runner::start(fx.supervisor.clone(), &fx.paths, || {}).unwrap();

    let copy = runner
        .create(input(
            "Copy photos",
            JobKind::Copy,
            local(&source),
            remote("orbit-dst", "backup"),
        ))
        .unwrap();
    runner.run(&copy.job.id).unwrap();
    let (outcome, _, errors) = wait_for_run(&runner, &copy.job.id).await;
    assert_eq!(outcome, RunOutcome::Succeeded);
    assert_eq!(errors, 0);
    assert_eq!(
        fs::read(fx.root.join("remote/backup/one.txt")).unwrap(),
        b"1"
    );
    assert_eq!(
        fs::read(fx.root.join("remote/backup/nested/two.txt")).unwrap(),
        b"2"
    );
    assert!(
        fx.root.join("remote/backup/extra.txt").exists(),
        "copy must not delete"
    );

    let log = runner
        .read_log(&copy.job.id, &runner.list()[0].job.runs[0].id)
        .unwrap();
    assert!(log.contains("Copy job \"Copy photos\""), "{log}");
    assert!(
        log.contains("Finished. Everything was transferred."),
        "{log}"
    );

    let sync = runner
        .create(input(
            "Mirror photos",
            JobKind::Sync,
            local(&source),
            remote("orbit-dst", "backup"),
        ))
        .unwrap();
    runner.run(&sync.job.id).unwrap();
    let (outcome, _, _) = wait_for_run(&runner, &sync.job.id).await;
    assert_eq!(outcome, RunOutcome::Succeeded);
    assert!(
        !fx.root.join("remote/backup/extra.txt").exists(),
        "sync must remove extras"
    );

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn missing_source_fails_with_a_message_and_locks_are_enforced() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-src", &fx.root.join("remote")).await;
    let runner = Runner::start(fx.supervisor.clone(), &fx.paths, || {}).unwrap();

    let job = runner
        .create(input(
            "Missing",
            JobKind::Copy,
            remote("orbit-src", "does-not-exist"),
            local(&fx.root.join("out")),
        ))
        .unwrap();
    runner.run(&job.job.id).unwrap();
    let (outcome, _, _) = wait_for_run(&runner, &job.job.id).await;
    match outcome {
        RunOutcome::Failed { message, .. } => assert!(!message.is_empty()),
        other => panic!("expected Failed, got {other:?}"),
    }

    // A job that is not running can be edited and deleted.
    let renamed = runner
        .update(
            &job.job.id,
            input(
                "Renamed",
                JobKind::Copy,
                remote("orbit-src", "does-not-exist"),
                local(&fx.root.join("out")),
            ),
        )
        .unwrap();
    assert_eq!(renamed.job.name, "Renamed");

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn cancel_stops_a_running_job_and_running_jobs_are_locked() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-dst", &fx.root.join("remote")).await;
    let source = fx.root.join("source");
    fs::create_dir_all(&source).unwrap();
    // Large enough that the run is still going when we cancel it.
    fs::write(source.join("big.bin"), vec![3u8; 128 * 1024 * 1024]).unwrap();

    let runner = Runner::start(fx.supervisor.clone(), &fx.paths, || {}).unwrap();
    let job = runner
        .create(input(
            "Big copy",
            JobKind::Copy,
            local(&source),
            remote("orbit-dst", ""),
        ))
        .unwrap();
    runner.run(&job.job.id).unwrap();

    // Wait until rclone has the job, then try to change it while it runs.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let view = runner
            .list()
            .into_iter()
            .find(|v| v.job.id == job.job.id)
            .unwrap();
        if view.progress.is_some() && runner.active() == 1 {
            break;
        }
        assert!(Instant::now() < deadline);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        runner
            .update(
                &job.job.id,
                input(
                    "Changed",
                    JobKind::Copy,
                    local(&source),
                    remote("orbit-dst", "")
                )
            )
            .is_err()
    );
    assert!(runner.delete(&job.job.id).is_err());
    assert!(
        runner.run(&job.job.id).is_err(),
        "a running job cannot start twice"
    );

    runner.cancel(&job.job.id).await.unwrap();
    let (outcome, _, _) = wait_for_run(&runner, &job.job.id).await;
    assert_eq!(outcome, RunOutcome::Cancelled);

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn a_run_cut_short_by_a_crash_is_marked_failed_on_next_start() {
    let Some(fx) = fixture().await else { return };
    let job = Job {
        id: "job-crashed".into(),
        name: "Interrupted".into(),
        kind: JobKind::Copy,
        source: local(&fx.root),
        destination: remote("nowhere", ""),
        enabled: true,
        schedule: Schedule::Manual,
        runs: vec![orbit_lib::jobs::RunRecord {
            id: "run-1".into(),
            started_at: 1,
            finished_at: None,
            outcome: RunOutcome::Running,
            files: 0,
            bytes: 0,
            errors: 0,
            log_file: "job-job-crashed-run-1.log".into(),
            scheduled: false,
        }],
    };
    store::save(&fx.paths.jobs(), &[job]).unwrap();

    let runner = Runner::start(fx.supervisor.clone(), &fx.paths, || {}).unwrap();
    let recovered: JobView = runner.list().remove(0);
    assert!(matches!(
        recovered.job.runs[0].outcome,
        RunOutcome::Failed { .. }
    ));
    assert!(recovered.progress.is_none());

    fx.supervisor.shutdown().await;
}

#[tokio::test]
async fn scheduled_jobs_start_when_due_and_only_while_idle_enabled_and_connected() {
    let Some(fx) = fixture().await else { return };
    alias(&fx, "orbit-dst", &fx.root.join("remote")).await;
    let source = fx.root.join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("one.txt"), b"1").unwrap();
    let runner = Runner::start(fx.supervisor.clone(), &fx.paths, || {}).unwrap();

    let mut every = input(
        "Every minute",
        JobKind::Copy,
        local(&source),
        remote("orbit-dst", "auto"),
    );
    every.schedule = Schedule::Every { minutes: 1 };
    let armed = orbit_lib::jobs::runner::now_secs();
    let job = runner.create(every).unwrap();
    let next = job.next_run_at.expect("a scheduled job has a next run");
    assert!(
        (armed + 60..=armed + 62).contains(&next),
        "next run {next} vs armed {armed}"
    );

    // Not due yet, then due exactly once: a second look while it runs starts nothing.
    assert!(runner.run_due(armed + 30).is_empty());
    assert_eq!(runner.run_due(armed + 61), vec![job.job.id.clone()]);
    assert!(runner.run_due(armed + 61).is_empty());
    let (outcome, _, _) = wait_for_run(&runner, &job.job.id).await;
    assert_eq!(outcome, RunOutcome::Succeeded);
    assert_eq!(fs::read(fx.root.join("remote/auto/one.txt")).unwrap(), b"1");

    let view = runner
        .list()
        .into_iter()
        .find(|v| v.job.id == job.job.id)
        .unwrap();
    assert!(view.job.runs[0].scheduled, "the run is marked as scheduled");
    let log = runner.read_log(&job.job.id, &view.job.runs[0].id).unwrap();
    assert!(log.contains("started by the schedule"), "{log}");
    // The next run counts from the run that just happened.
    let started = view.job.runs[0].started_at;
    assert_eq!(view.next_run_at, Some(started + 60));

    // Disabled jobs never run on their own, and have no next run.
    runner.set_enabled(&job.job.id, false).unwrap();
    assert!(runner.run_due(armed + 10_000).is_empty());
    assert!(runner.list()[0].next_run_at.is_none());

    // Turning a job back on re-arms it instead of running it at once.
    let before_enable = orbit_lib::jobs::runner::now_secs();
    runner.set_enabled(&job.job.id, true).unwrap();
    assert!(runner.run_due(before_enable + 30).is_empty());
    assert!(runner.list()[0].next_run_at.unwrap() >= before_enable + 60);

    // Manual jobs never run on their own.
    let manual = runner
        .create(input(
            "By hand",
            JobKind::Copy,
            local(&source),
            remote("orbit-dst", "manual"),
        ))
        .unwrap();
    assert!(manual.next_run_at.is_none());
    let started = runner.run_due(armed + 10_000);
    assert_eq!(
        started,
        vec![job.job.id.clone()],
        "only the scheduled job starts"
    );
    let (outcome, _, _) = wait_for_run(&runner, &job.job.id).await;
    assert_eq!(outcome, RunOutcome::Succeeded);
    assert!(!fx.root.join("remote/manual").exists());

    // Nothing starts while rclone is down.
    fx.supervisor.shutdown().await;
    assert!(runner.run_due(armed + 100_000).is_empty());
}

#[tokio::test]
async fn mount_makes_remote_files_visible_and_unmount_removes_them() {
    if !orbit_lib::mounts::support().available {
        eprintln!("skipping: mounting is not available here");
        return;
    }
    let Some(fx) = fixture().await else { return };
    let remote_dir = fx.root.join("remote");
    alias(&fx, "orbit-mount", &remote_dir).await;
    fs::write(remote_dir.join("hello.txt"), b"from the remote").unwrap();
    let mount_point = fx.root.join("mnt");
    fs::create_dir_all(&mount_point).unwrap();

    let client = fx.supervisor.client().unwrap();
    mounts::mount(&client, "orbit-mount", &mount_point.to_string_lossy())
        .await
        .unwrap();

    let mounted = mounts::list(&client).await.unwrap();
    assert_eq!(mounted.len(), 1);
    assert_eq!(Path::new(&mounted[0].mount_point), mount_point.as_path());

    let text =
        fs::read_to_string(mount_point.join("hello.txt")).expect("file visible through the mount");
    assert_eq!(text, "from the remote");

    mounts::unmount(&client, &mount_point.to_string_lossy())
        .await
        .unwrap();
    assert!(mounts::list(&client).await.unwrap().is_empty());
    assert!(!mount_point.join("hello.txt").exists());

    fx.supervisor.shutdown().await;
}
