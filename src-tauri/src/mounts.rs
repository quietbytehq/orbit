//! Mounting remotes as drives or folders, through rclone's mount API.
//!
//! Linux mounts to an empty folder and needs FUSE 3. Windows mounts to a free drive letter and
//! needs WinFsp. Mounts end when Orbit quits.

use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::{RcClient, Supervisor};
use crate::text;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;

const MOUNT_TIMEOUT: Duration = Duration::from_secs(60);

/// A drive or folder the person set up, kept in `mounts.json` so it can be offered again and,
/// when `auto_connect` is on, connected on startup once the engine is healthy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedMount {
    pub remote: String,
    pub mount_point: String,
    #[serde(default)]
    pub auto_connect: bool,
}

/// Remembered mounts plus which remote each live mount point was made from (rclone reports the
/// resolved filesystem instead of the name the user chose).
pub struct MountStore {
    file: PathBuf,
    saved: Mutex<Vec<SavedMount>>,
    live: Mutex<HashMap<String, String>>,
}

impl MountStore {
    /// Loads `mounts.json`; a missing or empty file means nothing saved.
    pub fn load(file: PathBuf) -> Self {
        let saved = match std::fs::read_to_string(&file) {
            Ok(text) if !text.trim().is_empty() => {
                serde_json::from_str(&text).unwrap_or_else(|err| {
                    eprintln!("Orbit: could not read mounts.json, starting empty: {err}");
                    Vec::new()
                })
            }
            _ => Vec::new(),
        };
        Self {
            file,
            saved: Mutex::new(saved),
            live: Mutex::new(HashMap::new()),
        }
    }

    pub fn saved(&self) -> Vec<SavedMount> {
        self.saved.lock().expect("mounts lock").clone()
    }

    /// Records a mount the person set up (one entry per remote).
    pub fn remember(&self, remote: &str, mount_point: &str, auto_connect: bool) {
        {
            let mut saved = self.saved.lock().expect("mounts lock");
            saved.retain(|m| m.remote != remote);
            saved.push(SavedMount {
                remote: remote.to_owned(),
                mount_point: mount_point.to_owned(),
                auto_connect,
            });
        }
        self.live
            .lock()
            .expect("mounts lock")
            .insert(mount_point.to_owned(), remote.to_owned());
        self.persist();
    }

    pub fn set_auto_connect(&self, remote: &str, auto_connect: bool) -> AppResult<()> {
        {
            let mut saved = self.saved.lock().expect("mounts lock");
            let entry = saved
                .iter_mut()
                .find(|m| m.remote == remote)
                .ok_or_else(|| AppError::invalid(text::remote_missing(remote)))?;
            entry.auto_connect = auto_connect;
        }
        self.persist();
        Ok(())
    }

    /// Drops the remembered entry. The live mount, if any, is left alone.
    pub fn forget(&self, remote: &str) {
        self.saved
            .lock()
            .expect("mounts lock")
            .retain(|m| m.remote != remote);
        self.persist();
    }

    fn live_remote_for(&self, mount_point: &str) -> Option<String> {
        self.live
            .lock()
            .expect("mounts lock")
            .get(mount_point)
            .cloned()
    }

    fn live_forget(&self, mount_point: &str) {
        self.live.lock().expect("mounts lock").remove(mount_point);
    }

    fn persist(&self) {
        let saved = self.saved.lock().expect("mounts lock").clone();
        if let Err(err) = save_file(&self.file, &saved) {
            eprintln!("Orbit: could not save mounts.json: {}", err.message);
        }
    }
}

fn save_file(path: &Path, saved: &[SavedMount]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(saved)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// The saved mounts to connect at startup: those marked automatic.
pub fn to_restore(saved: &[SavedMount]) -> Vec<&SavedMount> {
    saved.iter().filter(|m| m.auto_connect).collect()
}

/// Connects every automatic mount. Called once, when the engine first becomes healthy. Failures
/// are logged; the Drives page shows what is actually connected.
pub async fn restore(client: &RcClient, store: &MountStore) {
    if !support().available {
        return;
    }
    for entry in to_restore(&store.saved()) {
        match mount(client, &entry.remote, &entry.mount_point).await {
            Ok(()) => {
                store
                    .live
                    .lock()
                    .expect("mounts lock")
                    .insert(entry.mount_point.clone(), entry.remote.clone());
            }
            Err(err) => eprintln!(
                "Orbit: could not reconnect {} at {}: {}",
                entry.remote, entry.mount_point, err.message
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountInfo {
    pub remote: String,
    pub mount_point: String,
    pub mounted_on: Option<String>,
    /// Whether this mount is remembered and connected again at startup.
    pub auto_connect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WinFspStatus {
    NotApplicable,
    Installed,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountSupport {
    pub platform: &'static str,
    pub available: bool,
    pub winfsp: WinFspStatus,
    /// What to install if mounting is not available yet.
    pub requirement: Option<&'static str>,
}

#[derive(Deserialize)]
struct ListReply {
    #[serde(default, rename = "mountPoints")]
    mount_points: Vec<RawMount>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawMount {
    fs: String,
    mount_point: String,
    #[serde(default)]
    mounted_on: Option<String>,
}

pub fn platform() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    }
}

/// Tells the UI whether mounting works here and, if not, what to install.
pub fn support() -> MountSupport {
    let winfsp = winfsp_status();
    let (available, requirement) = match platform() {
        "linux" if fuse_available() => (true, None),
        "linux" => (false, Some(text::MOUNT_NEEDS_FUSE)),
        "windows" if winfsp == WinFspStatus::Installed => (true, None),
        "windows" => (false, Some(text::MOUNT_NEEDS_WINFSP)),
        _ => (false, Some(text::MOUNT_UNSUPPORTED)),
    };
    MountSupport {
        platform: platform(),
        available,
        winfsp,
        requirement,
    }
}

#[cfg(target_os = "linux")]
fn fuse_available() -> bool {
    ["fusermount3", "fusermount"].iter().any(|tool| {
        std::env::var_os("PATH")
            .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(tool).is_file()))
            .unwrap_or(false)
    })
}

#[cfg(not(target_os = "linux"))]
fn fuse_available() -> bool {
    false
}

#[cfg(windows)]
pub fn winfsp_status() -> WinFspStatus {
    use std::path::Path;
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let install_dir = hklm
        .open_subkey("SOFTWARE\\WOW6432Node\\WinFsp")
        .or_else(|_| hklm.open_subkey("SOFTWARE\\WinFsp"))
        .and_then(|key| key.get_value::<String, _>("InstallDir"));
    match install_dir {
        Ok(dir) if Path::new(&dir).exists() => WinFspStatus::Installed,
        _ => WinFspStatus::Missing,
    }
}

#[cfg(not(windows))]
pub fn winfsp_status() -> WinFspStatus {
    WinFspStatus::NotApplicable
}

/// Where a new mount for `remote` goes: a folder under the home directory on Linux,
/// or the first free drive letter from Z: down on Windows.
pub fn suggest_mount_point(remote: &str) -> AppResult<String> {
    if cfg!(windows) {
        return first_free_drive_letter()
            .ok_or_else(|| AppError::new(ErrorKind::Unsupported, text::MOUNT_NO_LETTER));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| AppError::internal(text::MOUNT_NO_HOME))?;
    let safe: String = remote
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    Ok(std::path::Path::new(&home)
        .join("Orbit")
        .join(safe)
        .to_string_lossy()
        .into_owned())
}

#[cfg(windows)]
fn first_free_drive_letter() -> Option<String> {
    ('D'..='Z')
        .rev()
        .map(|letter| format!("{letter}:"))
        .find(|drive| !std::path::Path::new(&format!("{drive}\\")).exists())
}

#[cfg(not(windows))]
fn first_free_drive_letter() -> Option<String> {
    None
}

pub async fn list(client: &RcClient) -> AppResult<Vec<MountInfo>> {
    let reply: ListReply = client.call("mount/listmounts", &json!({})).await?;
    Ok(reply
        .mount_points
        .into_iter()
        .map(|mount| MountInfo {
            remote: mount.fs.trim_end_matches(':').to_owned(),
            mount_point: mount.mount_point,
            mounted_on: mount.mounted_on,
            auto_connect: false,
        })
        .collect())
}

pub async fn mount(client: &RcClient, remote: &str, mount_point: &str) -> AppResult<()> {
    if cfg!(target_os = "linux") {
        std::fs::create_dir_all(mount_point)?;
    }
    client
        .call_value_with_timeout(
            "mount/mount",
            &json!({
                "fs": format!("{remote}:"),
                "mountPoint": mount_point,
                "vfsOpt": { "CacheMode": "writes" },
            }),
            MOUNT_TIMEOUT,
        )
        .await
        .map(|_| ())
}

pub async fn unmount(client: &RcClient, mount_point: &str) -> AppResult<()> {
    client
        .call_value("mount/unmount", &json!({ "mountPoint": mount_point }))
        .await
        .map(|_| ())
}

/// Used on quit so no mount outlives Orbit. Errors are ignored: rcd is stopping anyway.
pub async fn unmount_all(client: &RcClient) {
    let _ = client.call_value("mount/unmountall", &json!({})).await;
}

fn ensure_available() -> AppResult<()> {
    let support = support();
    if support.available {
        return Ok(());
    }
    Err(AppError::new(
        ErrorKind::Unsupported,
        support.requirement.unwrap_or(text::MOUNT_UNSUPPORTED),
    ))
}

#[tauri::command]
pub fn mount_support() -> MountSupport {
    support()
}

#[tauri::command]
pub fn suggest_mount_location(remote: String) -> AppResult<String> {
    suggest_mount_point(&remote)
}

#[tauri::command]
pub async fn list_mounts(
    supervisor: State<'_, Supervisor>,
    store: State<'_, MountStore>,
) -> AppResult<Vec<MountInfo>> {
    let mut mounts = list(&supervisor.client()?).await?;
    let saved = store.saved();
    for mount in &mut mounts {
        if let Some(remote) = store.live_remote_for(&mount.mount_point) {
            mount.remote = remote;
        }
        mount.auto_connect = saved
            .iter()
            .any(|m| m.remote == mount.remote && m.auto_connect);
    }
    Ok(mounts)
}

/// Remembered drives and folders, connected or not.
#[tauri::command]
pub fn list_saved_mounts(store: State<'_, MountStore>) -> Vec<SavedMount> {
    store.saved()
}

#[tauri::command]
pub fn set_mount_auto_connect(
    remote: String,
    auto_connect: bool,
    store: State<'_, MountStore>,
) -> AppResult<()> {
    store.set_auto_connect(&remote, auto_connect)
}

#[tauri::command]
pub fn forget_saved_mount(remote: String, store: State<'_, MountStore>) {
    store.forget(&remote);
}

#[tauri::command]
pub async fn mount_remote(
    remote: String,
    mount_point: String,
    auto_connect: bool,
    supervisor: State<'_, Supervisor>,
    store: State<'_, MountStore>,
) -> AppResult<()> {
    ensure_available()?;
    let mount_point = mount_point.trim();
    if mount_point.is_empty() {
        return Err(AppError::invalid(text::MOUNT_WHERE));
    }
    mount(&supervisor.client()?, &remote, mount_point).await?;
    store.remember(&remote, mount_point, auto_connect);
    Ok(())
}

/// Disconnects a drive or folder. The remembered entry stays, so it can be offered again.
#[tauri::command]
pub async fn unmount_remote(
    mount_point: String,
    supervisor: State<'_, Supervisor>,
    store: State<'_, MountStore>,
) -> AppResult<()> {
    unmount(&supervisor.client()?, &mount_point).await?;
    store.live_forget(&mount_point);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mount_list() {
        let reply: ListReply = serde_json::from_str(
            r#"{"mountPoints":[{"Fs":"work:","MountPoint":"/home/ana/Orbit/work","MountedOn":"2026-10-22T10:00:00Z","MountConfig":{}}]}"#,
        )
        .unwrap();
        assert_eq!(reply.mount_points[0].fs, "work:");
        assert_eq!(reply.mount_points[0].mount_point, "/home/ana/Orbit/work");
    }

    #[test]
    fn empty_mount_list_is_fine() {
        let reply: ListReply = serde_json::from_str("{}").unwrap();
        assert!(reply.mount_points.is_empty());
    }

    #[test]
    fn suggested_folder_is_safe_for_the_remote_name() {
        if cfg!(windows) {
            return;
        }
        let path = suggest_mount_point("my remote/x").unwrap();
        assert!(path.ends_with("Orbit/my_remote_x"), "{path}");
    }

    #[test]
    fn support_reports_a_requirement_when_unavailable() {
        let support = support();
        assert_eq!(support.available, support.requirement.is_none());
    }

    #[test]
    fn saved_mounts_round_trip_and_keep_one_entry_per_remote() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mounts.json");
        let store = MountStore::load(file.clone());
        assert!(store.saved().is_empty());
        store.remember("work", "/mnt/work", false);
        store.remember("photos", "Z:", true);
        store.remember("work", "/mnt/work2", true);
        let again = MountStore::load(file.clone());
        let saved = again.saved();
        assert_eq!(saved.len(), 2);
        let work = saved.iter().find(|m| m.remote == "work").unwrap();
        assert_eq!(work.mount_point, "/mnt/work2");
        assert!(work.auto_connect);
        again.set_auto_connect("work", false).unwrap();
        assert!(again.set_auto_connect("nope", true).is_err());
        again.forget("photos");
        let saved = MountStore::load(file).saved();
        assert_eq!(saved.len(), 1);
        assert!(!saved[0].auto_connect);
    }

    #[test]
    fn only_automatic_mounts_are_restored() {
        let saved = vec![
            SavedMount {
                remote: "a".into(),
                mount_point: "X:".into(),
                auto_connect: true,
            },
            SavedMount {
                remote: "b".into(),
                mount_point: "Y:".into(),
                auto_connect: false,
            },
        ];
        let names: Vec<_> = to_restore(&saved)
            .iter()
            .map(|m| m.remote.as_str())
            .collect();
        assert_eq!(names, vec!["a"]);
    }

    #[test]
    fn a_broken_mounts_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("mounts.json");
        std::fs::write(&file, "not json").unwrap();
        assert!(MountStore::load(file).saved().is_empty());
    }
}
