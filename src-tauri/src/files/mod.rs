//! File browsing and transfers.
//!
//! Remote paths are relative to the remote's root and use "/" separators, with "" for the root.
//! Local paths are absolute paths on this computer. Both are passed to rclone as on-the-fly
//! backend strings: `remote:path` for remotes and `:local:/path` for this computer's files.

pub mod commands;
pub mod local;
pub mod ops;
pub mod transfers;

use serde::{Deserialize, Serialize};

/// One file or folder inside a remote, as shown in the browser.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    /// Path relative to the remote root.
    pub path: String,
    pub is_dir: bool,
    pub size: i64,
    pub mod_time: Option<String>,
    pub mime_type: Option<String>,
}

/// Where a transfer reads from or writes to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Endpoint {
    Remote { remote: String, path: String },
    Local { path: String },
}

/// Remote path helpers. Paths never start or end with a slash.
pub fn normalize(path: &str) -> String {
    path.split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/")
}

pub fn join(parent: &str, name: &str) -> String {
    let parent = normalize(parent);
    let name = normalize(name);
    match (parent.is_empty(), name.is_empty()) {
        (true, _) => name,
        (false, true) => parent,
        (false, false) => format!("{parent}/{name}"),
    }
}

/// Splits "a/b/c" into ("a/b", "c"). A top-level name gives ("", name).
pub fn split(path: &str) -> (String, String) {
    let path = normalize(path);
    match path.rfind('/') {
        Some(index) => (path[..index].to_owned(), path[index + 1..].to_owned()),
        None => (String::new(), path),
    }
}

/// rclone fs string for a remote folder.
pub fn remote_fs(remote: &str, path: &str) -> String {
    format!("{remote}:{}", normalize(path))
}

/// rclone fs string for a folder on this computer. `:local:` stops colons in paths being read as remotes.
pub fn local_fs(path: &str) -> String {
    format!(":local:{path}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_and_dots() {
        assert_eq!(normalize("/a//b/./c/"), "a/b/c");
        assert_eq!(normalize("/"), "");
    }

    #[test]
    fn joins_and_splits() {
        assert_eq!(join("", "Photos"), "Photos");
        assert_eq!(join("Photos", "2024"), "Photos/2024");
        assert_eq!(split("Photos/2024/cat.jpg"), ("Photos/2024".into(), "cat.jpg".into()));
        assert_eq!(split("cat.jpg"), (String::new(), "cat.jpg".into()));
    }

    #[test]
    fn fs_strings() {
        assert_eq!(remote_fs("work", "/Photos/"), "work:Photos");
        assert_eq!(remote_fs("work", ""), "work:");
        assert_eq!(local_fs("/home/ana/Pictures"), ":local:/home/ana/Pictures");
    }

    #[test]
    fn endpoint_serializes_with_kind() {
        let json = serde_json::to_value(Endpoint::Local { path: "/x".into() }).unwrap();
        assert_eq!(json["kind"], "local");
        assert_eq!(json["path"], "/x");
    }
}
