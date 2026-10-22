//! Remote management: what connections exist, how they are created and checked.
//!
//! Orbit keeps no copy of credentials. Everything is written through rclone's RC config
//! endpoints, into rclone's own config file.

pub mod commands;
pub mod health;
pub mod oauth;
pub mod providers;
pub mod store;

use health::RemoteHealth;
use serde::Serialize;

/// One connection, as shown in the sidebar and the remotes list.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSummary {
    pub name: String,
    pub rclone_type: String,
    /// Wizard that created this remote, if it is one Orbit knows.
    pub provider_id: Option<String>,
    pub provider_title: String,
    pub health: RemoteHealth,
}

/// A remote's non-secret settings, for the edit form.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteForm {
    pub name: String,
    pub provider_id: String,
    pub values: std::collections::BTreeMap<String, String>,
    /// Secret fields that are already saved. The UI shows "saved" and never the value.
    pub saved_secrets: Vec<String>,
}

/// Result of a connection test.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTest {
    pub ok: bool,
    pub message: String,
}
