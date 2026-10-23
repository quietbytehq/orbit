//! Integration tests for remote management against a real `rclone rcd`.
//!
//! The local backend stands in for cloud accounts. The sign-in tests drive rclone's browser flow
//! up to the callback, using a bogus code, since no real account is available here.

mod common;

use common::{spec, wait_for_version};
use orbit_lib::error::ErrorKind;
use orbit_lib::rclone::process::RcdProcess;
use orbit_lib::remotes::health::RemoteHealth;
use orbit_lib::remotes::oauth;
use orbit_lib::remotes::store;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Starts rcd for a test, or returns None (after a note) when rclone is not available.
async fn start(dir: &std::path::Path) -> Option<RcdProcess> {
    let Some(binary) = common::find_rclone() else {
        eprintln!("skipping: no rclone found (set ORBIT_TEST_RCLONE)");
        return None;
    };
    let process = RcdProcess::spawn(&spec(binary, dir)).unwrap();
    wait_for_version(&process).await;
    Some(process)
}

fn params(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[tokio::test]
async fn local_remote_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let Some(process) = start(dir.path()).await else {
        return;
    };
    let client = process.client();

    store::create(&client, "orbit-local", "local", &BTreeMap::new())
        .await
        .unwrap();
    assert!(store::exists(&client, "orbit-local").await.unwrap());

    let listed = store::list(&client).await.unwrap();
    let remote = listed
        .iter()
        .find(|r| r.name == "orbit-local")
        .expect("listed");
    assert_eq!(remote.rclone_type, "local");
    assert!(remote.provider_id.is_none());

    let test = store::test(&client, "orbit-local").await.unwrap();
    assert!(test.ok, "local remote should connect: {}", test.message);

    // Update keeps untouched keys and changes the one we pass.
    store::update(&client, "orbit-local", &params(&[("nounc", "true")]))
        .await
        .unwrap();
    let form = store::form(&client, "orbit-local").await.unwrap_err();
    assert_eq!(
        form.kind,
        ErrorKind::Unsupported,
        "local is not a wizard type, so no edit form"
    );

    store::rename(&client, "orbit-local", "orbit-renamed")
        .await
        .unwrap();
    assert!(!store::exists(&client, "orbit-local").await.unwrap());
    assert!(store::exists(&client, "orbit-renamed").await.unwrap());

    store::delete(&client, "orbit-renamed").await.unwrap();
    assert!(!store::exists(&client, "orbit-renamed").await.unwrap());

    process.stop().await;
}

#[tokio::test]
async fn sftp_form_hides_saved_password() {
    let dir = tempfile::tempdir().unwrap();
    let Some(process) = start(dir.path()).await else {
        return;
    };
    let client = process.client();

    // Port 1 on loopback is essentially never open, so the connection test fails quickly.
    let sftp = params(&[
        ("host", "127.0.0.1"),
        ("port", "1"),
        ("user", "ana"),
        ("pass", "hunter2-secret"),
    ]);
    store::create(&client, "orbit-sftp", "sftp", &sftp)
        .await
        .unwrap();

    let form = store::form(&client, "orbit-sftp").await.unwrap();
    assert_eq!(form.provider_id, "sftp");
    assert_eq!(form.saved_secrets, vec!["pass".to_owned()]);
    assert_eq!(form.values["host"], "127.0.0.1");
    assert!(!form.values.contains_key("pass"));
    let serialized = serde_json::to_string(&form).unwrap();
    assert!(
        !serialized.contains("hunter2"),
        "the stored password must not reach the UI"
    );

    let result = store::test(&client, "orbit-sftp").await.unwrap();
    assert!(!result.ok);
    assert!(
        result.message.contains("refused") || result.message.contains("reach"),
        "{}",
        result.message
    );

    process.stop().await;
}

#[tokio::test]
async fn browser_sign_in_reports_url_and_can_be_cancelled() {
    let dir = tempfile::tempdir().unwrap();
    let Some(process) = start(dir.path()).await else {
        return;
    };
    let client = process.client();

    // rclone's callback server listens on a fixed port. Skip if something else holds it.
    if std::net::TcpListener::bind("127.0.0.1:53682").is_err() {
        eprintln!("skipping: 127.0.0.1:53682 is in use");
        process.stop().await;
        return;
    }

    // Bogus code: rclone reports the sign-in as failed instead of waiting forever.
    let urls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = urls.clone();
    let body = params(&[("scope", "drive")]);
    let started = Instant::now();
    let sign_in = tokio::spawn({
        let client = client.clone();
        async move {
            oauth::create_with_browser(&client, "orbit-drive", "drive", &body, move |url| {
                sink.lock().unwrap().push(url.to_owned());
            })
            .await
        }
    });

    let url = wait_for_url(&urls).await;
    assert!(
        url.starts_with("http://127.0.0.1:53682/auth?state="),
        "{url}"
    );
    let state = url.split("state=").nth(1).unwrap();
    // On Windows rclone can close this connection without a reply once it has the code; what
    // matters is that the sign-in below ends with an error, so a dropped reply is not a failure.
    match reqwest::get(format!("http://127.0.0.1:53682/?state={state}&code=bogus")).await {
        Ok(callback) => assert!(callback.status().is_success()),
        Err(err) => eprintln!("callback ended without a reply (tolerated): {err}"),
    }

    let outcome = tokio::time::timeout(Duration::from_secs(90), sign_in)
        .await
        .expect("sign-in should finish")
        .unwrap();
    assert!(outcome.is_err(), "a bogus code must not succeed");
    assert!(started.elapsed() < Duration::from_secs(90));
    assert!(
        !store::exists(&client, "orbit-drive").await.unwrap(),
        "failed sign-in must not leave a remote"
    );

    // Second run: cancel while waiting.
    let urls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = urls.clone();
    let sign_in = tokio::spawn({
        let client = client.clone();
        async move {
            oauth::create_with_browser(
                &client,
                "orbit-drive",
                "drive",
                &BTreeMap::new(),
                move |url| {
                    sink.lock().unwrap().push(url.to_owned());
                },
            )
            .await
        }
    });
    wait_for_url(&urls).await;
    oauth::cancel(&client).await.unwrap();
    let outcome = tokio::time::timeout(Duration::from_secs(30), sign_in)
        .await
        .expect("cancel should end sign-in")
        .unwrap();
    assert!(outcome.is_err());

    process.stop().await;
}

async fn wait_for_url(urls: &Arc<Mutex<Vec<String>>>) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(url) = urls.lock().unwrap().first().cloned() {
            return url;
        }
        assert!(Instant::now() < deadline, "no sign-in URL was reported");
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test]
async fn smb_share_is_stored_and_used_for_the_check() {
    let dir = tempfile::tempdir().unwrap();
    let Some(process) = start(dir.path()).await else {
        return;
    };
    let client = process.client();
    let closed_port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let smb = orbit_lib::remotes::providers::find("smb").unwrap();
    let params = orbit_lib::remotes::providers::build_parameters(
        smb,
        &params(&[
            ("host", "127.0.0.1"),
            ("share", "docs"),
            ("user", "ana"),
            ("pass", "hunter2"),
            ("port", &closed_port.to_string()),
        ]),
        None,
    )
    .unwrap();
    store::create(&client, "orbit-smb", "smb", &params)
        .await
        .unwrap();

    let summary = store::list(&client)
        .await
        .unwrap()
        .into_iter()
        .find(|r| r.name == "orbit-smb")
        .unwrap();
    assert_eq!(summary.provider_id.as_deref(), Some("smb"));
    assert_eq!(summary.start_path, "docs", "the share is the start path");

    let form = store::form(&client, "orbit-smb").await.unwrap();
    assert_eq!(form.values.get("share").map(String::as_str), Some("docs"));
    assert_eq!(
        form.values.get("host").map(String::as_str),
        Some("127.0.0.1")
    );
    assert!(
        !form.values.contains_key("pass"),
        "the password never comes back"
    );
    assert!(form.saved_secrets.contains(&"pass".to_owned()));

    // Nothing listens on that port, so the check fails with a plain message and raw details.
    let result = store::test(&client, "orbit-smb").await.unwrap();
    assert!(!result.ok);
    assert!(
        !result.message.is_empty() && !result.message.to_lowercase().contains("rclone"),
        "{}",
        result.message
    );
    assert!(result.details.as_deref().is_some_and(|d| !d.is_empty()));

    store::delete(&client, "orbit-smb").await.unwrap();
    process.stop().await;
}

#[test]
fn health_defaults_are_unknown() {
    let health = orbit_lib::remotes::health::HealthStore::default();
    assert_eq!(health.get("anything"), RemoteHealth::Unknown);
}
