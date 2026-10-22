use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;

/// What Orbit last learned about a remote. In-memory only; it is re-checked on each launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum RemoteHealth {
    Unknown,
    Ok,
    Failed { message: String },
}

#[derive(Default)]
pub struct HealthStore {
    entries: Mutex<HashMap<String, RemoteHealth>>,
}

impl HealthStore {
    pub fn get(&self, name: &str) -> RemoteHealth {
        self.entries
            .lock()
            .expect("health lock")
            .get(name)
            .cloned()
            .unwrap_or(RemoteHealth::Unknown)
    }

    pub fn set(&self, name: &str, health: RemoteHealth) {
        self.entries.lock().expect("health lock").insert(name.to_owned(), health);
    }

    pub fn forget(&self, name: &str) {
        self.entries.lock().expect("health lock").remove(name);
    }

    pub fn rename(&self, from: &str, to: &str) {
        let mut entries = self.entries.lock().expect("health lock");
        if let Some(health) = entries.remove(from) {
            entries.insert(to.to_owned(), health);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_until_set() {
        let store = HealthStore::default();
        assert_eq!(store.get("a"), RemoteHealth::Unknown);
        store.set("a", RemoteHealth::Ok);
        assert_eq!(store.get("a"), RemoteHealth::Ok);
    }

    #[test]
    fn rename_moves_entry() {
        let store = HealthStore::default();
        store.set("old", RemoteHealth::Failed { message: "x".into() });
        store.rename("old", "new");
        assert_eq!(store.get("old"), RemoteHealth::Unknown);
        assert!(matches!(store.get("new"), RemoteHealth::Failed { .. }));
    }
}
