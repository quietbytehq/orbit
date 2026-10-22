//! Mounting remotes as drives or folders, through rclone's mount API.
//!
//! Linux mounts to an empty folder and needs FUSE 3. Windows mounts to a free drive letter and
//! needs WinFsp. Mounts end when Orbit quits.

use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::{RcClient, Supervisor};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tauri::State;

const MOUNT_TIMEOUT: Duration = Duration::from_secs(60);

/// Remembers which remote each mount point was made from. rclone reports the resolved
/// filesystem instead of the remote name, so this keeps the names the user chose.
#[derive(Default)]
pub struct MountRegistry(Mutex<HashMap<String, String>>);

impl MountRegistry {
    fn remember(&self, mount_point: &str, remote: &str) {
        self.0
            .lock()
            .expect("mount lock")
            .insert(mount_point.to_owned(), remote.to_owned());
    }

    fn forget(&self, mount_point: &str) {
        self.0.lock().expect("mount lock").remove(mount_point);
    }

    fn remote_for(&self, mount_point: &str) -> Option<String> {
        self.0.lock().expect("mount lock").get(mount_point).cloned()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MountInfo {
    pub remote: String,
    pub mount_point: String,
    pub mounted_on: Option<String>,
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
        "linux" => (
            false,
            Some("Install FUSE 3 to mount remotes. On Ubuntu: sudo apt install fuse3."),
        ),
        "windows" if winfsp == WinFspStatus::Installed => (true, None),
        "windows" => (
            false,
            Some("Install WinFsp to mount remotes. It is free and takes a minute."),
        ),
        _ => (false, Some("Mounting is not available on this system.")),
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
        return first_free_drive_letter().ok_or_else(|| {
            AppError::new(
                ErrorKind::Unsupported,
                "No drive letter is free for this mount.",
            )
        });
    }
    let home = std::env::var_os("HOME")
        .ok_or_else(|| AppError::internal("Could not find your home folder."))?;
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
        support
            .requirement
            .unwrap_or("Mounting is not available on this system."),
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
    registry: State<'_, MountRegistry>,
) -> AppResult<Vec<MountInfo>> {
    let mut mounts = list(&supervisor.client()?).await?;
    for mount in &mut mounts {
        if let Some(remote) = registry.remote_for(&mount.mount_point) {
            mount.remote = remote;
        }
    }
    Ok(mounts)
}

#[tauri::command]
pub async fn mount_remote(
    remote: String,
    mount_point: String,
    supervisor: State<'_, Supervisor>,
    registry: State<'_, MountRegistry>,
) -> AppResult<()> {
    ensure_available()?;
    let mount_point = mount_point.trim();
    if mount_point.is_empty() {
        return Err(AppError::invalid("Choose where the remote should appear."));
    }
    mount(&supervisor.client()?, &remote, mount_point).await?;
    registry.remember(mount_point, &remote);
    Ok(())
}

#[tauri::command]
pub async fn unmount_remote(
    mount_point: String,
    supervisor: State<'_, Supervisor>,
    registry: State<'_, MountRegistry>,
) -> AppResult<()> {
    unmount(&supervisor.client()?, &mount_point).await?;
    registry.forget(&mount_point);
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
}
