//! Browse and change files inside remotes, using rclone's operations API.

use super::{join, normalize, remote_fs, split, FileEntry};
use crate::error::AppResult;
use crate::rclone::RcClient;
use serde::Deserialize;
use serde_json::json;
use std::cmp::Ordering;

#[derive(Deserialize)]
struct ListReply {
    #[serde(default)]
    list: Vec<RawEntry>,
}

/// rclone's listing format (PascalCase keys).
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct RawEntry {
    name: String,
    #[serde(default)]
    size: i64,
    #[serde(default)]
    mod_time: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    is_dir: bool,
}

/// Lists a folder. Folders come first, then names in alphabetical order.
pub async fn list(client: &RcClient, remote: &str, path: &str) -> AppResult<Vec<FileEntry>> {
    let reply: ListReply = client
        .call(
            "operations/list",
            &json!({ "fs": remote_fs(remote, ""), "remote": normalize(path) }),
        )
        .await?;
    let mut entries: Vec<FileEntry> = reply
        .list
        .into_iter()
        .map(|raw| FileEntry {
            path: join(path, &raw.name),
            name: raw.name,
            is_dir: raw.is_dir,
            size: raw.size,
            mod_time: raw.mod_time,
            mime_type: raw.mime_type,
        })
        .collect();
    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(entries)
}

pub async fn create_folder(client: &RcClient, remote: &str, path: &str) -> AppResult<()> {
    client
        .call_value("operations/mkdir", &json!({ "fs": remote_fs(remote, ""), "remote": normalize(path) }))
        .await
        .map(|_| ())
}

/// Permanently deletes a file, or a folder and everything in it.
pub async fn delete(client: &RcClient, remote: &str, path: &str, is_dir: bool) -> AppResult<()> {
    let endpoint = if is_dir { "operations/purge" } else { "operations/deletefile" };
    client
        .call_value(endpoint, &json!({ "fs": remote_fs(remote, ""), "remote": normalize(path) }))
        .await
        .map(|_| ())
}

/// Renames inside the same remote. `new_name` is a single path segment.
pub async fn rename(client: &RcClient, remote: &str, path: &str, is_dir: bool, new_name: &str) -> AppResult<String> {
    let (parent, _) = split(path);
    let target = join(&parent, new_name);
    move_within(client, remote, path, &target, is_dir).await?;
    Ok(target)
}

async fn move_within(client: &RcClient, remote: &str, from: &str, to: &str, is_dir: bool) -> AppResult<()> {
    let body = if is_dir {
        json!({
            "srcFs": remote_fs(remote, from),
            "dstFs": remote_fs(remote, to),
            "deleteEmptySrcDirs": true,
        })
    } else {
        let (src_parent, src_name) = split(from);
        let (dst_parent, dst_name) = split(to);
        json!({
            "srcFs": remote_fs(remote, &src_parent),
            "srcRemote": src_name,
            "dstFs": remote_fs(remote, &dst_parent),
            "dstRemote": dst_name,
        })
    };
    let endpoint = if is_dir { "sync/move" } else { "operations/movefile" };
    client.call_value(endpoint, &body).await.map(|_| ())
}

#[cfg(test)]
mod tests {
    // The RC calls are covered by tests/files.rs against a real rclone. This module keeps the
    // path rules honest.
    use crate::files::{join, split};

    #[test]
    fn rename_target_keeps_parent() {
        let (parent, _) = split("Photos/2024/cat.jpg");
        assert_eq!(join(&parent, "dog.jpg"), "Photos/2024/dog.jpg");
    }
}
