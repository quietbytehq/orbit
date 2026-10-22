//! Reads and writes `jobs.json`. Writes go to a temp file first, then replace the old file.

use super::Job;
use crate::error::AppResult;
use std::fs;
use std::path::Path;

pub fn load(path: &Path) -> AppResult<Vec<Job>> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim().is_empty() => Ok(Vec::new()),
        Ok(text) => Ok(serde_json::from_str(&text)?),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(err) => Err(err.into()),
    }
}

pub fn save(path: &Path, jobs: &[Job]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(jobs)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::Endpoint;
    use crate::jobs::JobKind;

    #[test]
    fn round_trips_and_treats_missing_as_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.json");
        assert!(load(&path).unwrap().is_empty());

        let jobs = vec![Job {
            id: "j1".into(),
            name: "Backup".into(),
            kind: JobKind::Sync,
            source: Endpoint::Local { path: "/a".into() },
            destination: Endpoint::Remote { remote: "w".into(), path: "b".into() },
            enabled: false,
            runs: vec![],
        }];
        save(&path, &jobs).unwrap();
        assert_eq!(load(&path).unwrap(), jobs);
        assert!(!path.with_extension("json.tmp").exists());
    }
}
