//! Looks at files on this computer so uploads know which items are folders.

use crate::error::{AppError, AppResult, ErrorKind};
use serde::Serialize;
use std::path::Path;

/// A local file or folder chosen for upload. Its parent folder is the transfer source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalEntry {
    pub parent: String,
    pub name: String,
    pub is_dir: bool,
}

pub fn inspect(paths: &[String]) -> AppResult<Vec<LocalEntry>> {
    paths
        .iter()
        .map(|raw| {
            let path = Path::new(raw);
            let metadata = std::fs::metadata(path).map_err(|err| {
                AppError::new(ErrorKind::Io, format!("Could not read {raw}: {err}"))
            })?;
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .ok_or_else(|| AppError::invalid(format!("{raw} has no file name.")))?;
            let parent = path
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .ok_or_else(|| AppError::invalid(format!("{raw} has no folder above it.")))?;
            Ok(LocalEntry {
                parent,
                name,
                is_dir: metadata.is_dir(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn tells_files_from_folders() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        fs::write(&file, b"x").unwrap();
        let folder = dir.path().join("docs");
        fs::create_dir(&folder).unwrap();

        let entries = inspect(&[file.to_string_lossy().into_owned(), folder.to_string_lossy().into_owned()]).unwrap();
        assert_eq!(entries[0].name, "a.txt");
        assert!(!entries[0].is_dir);
        assert_eq!(entries[1].name, "docs");
        assert!(entries[1].is_dir);
        assert_eq!(entries[0].parent, dir.path().to_string_lossy());
    }

    #[test]
    fn missing_paths_are_reported() {
        let err = inspect(&["/definitely/not/here".to_owned()]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::Io);
    }
}
