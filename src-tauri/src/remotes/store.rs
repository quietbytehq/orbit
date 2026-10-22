//! Reads and writes rclone's remote config through the RC API.

use super::providers;
use super::{ConnectionTest, RemoteForm, RemoteSummary};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::RcClient;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::time::Duration;

/// How long a connection test may take before we call it a failure.
pub const TEST_TIMEOUT: Duration = Duration::from_secs(25);

#[derive(Deserialize)]
struct RemoteNames {
    #[serde(default)]
    remotes: Vec<String>,
}

/// Lists every remote in rclone's config, with the wizard it belongs to.
pub async fn list(client: &RcClient) -> AppResult<Vec<RemoteSummary>> {
    let names: RemoteNames = client.call("config/listremotes", &json!({})).await?;
    let mut summaries = Vec::with_capacity(names.remotes.len());
    for name in names.remotes {
        let raw = get_raw(client, &name).await?;
        let rclone_type = raw.get("type").cloned().unwrap_or_default();
        let provider = providers::for_rclone_type(&rclone_type);
        summaries.push(RemoteSummary {
            provider_id: provider.map(|p| p.id.to_owned()),
            provider_title: provider.map(|p| p.title.to_owned()).unwrap_or_else(|| rclone_type.clone()),
            rclone_type,
            health: super::health::RemoteHealth::Unknown,
            name,
        });
    }
    summaries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(summaries)
}

pub async fn exists(client: &RcClient, name: &str) -> AppResult<bool> {
    let names: RemoteNames = client.call("config/listremotes", &json!({})).await?;
    Ok(names.remotes.iter().any(|existing| existing == name))
}

/// Returns every key of a remote's section, secrets included. Internal use only.
async fn get_raw(client: &RcClient, name: &str) -> AppResult<BTreeMap<String, String>> {
    let value: Value = client.call("config/get", &json!({ "name": name })).await?;
    let object = value.as_object().cloned().unwrap_or_default();
    Ok(object
        .into_iter()
        .filter_map(|(key, value)| value.as_str().map(|text| (key, text.to_owned())))
        .collect())
}

/// Returns a remote's settings for the edit form. Secret values are withheld.
pub async fn form(client: &RcClient, name: &str) -> AppResult<RemoteForm> {
    let raw = get_raw(client, name).await?;
    let rclone_type = raw
        .get("type")
        .cloned()
        .ok_or_else(|| AppError::new(ErrorKind::NotRunning, format!("No remote named {name} was found.")))?;
    let provider = providers::for_rclone_type(&rclone_type).ok_or_else(|| {
        AppError::new(ErrorKind::Unsupported, format!("Orbit cannot edit {rclone_type} remotes yet."))
    })?;

    let secret_keys: Vec<&str> = providers::secret_keys(provider).collect();
    let mut values = BTreeMap::new();
    let mut saved_secrets = Vec::new();
    for (key, value) in raw.iter().filter(|(key, _)| key.as_str() != "type") {
        if secret_keys.contains(&key.as_str()) {
            saved_secrets.push(key.clone());
        } else {
            values.insert(key.clone(), value.clone());
        }
    }
    Ok(RemoteForm {
        name: name.to_owned(),
        provider_id: provider.id.to_owned(),
        values,
        saved_secrets,
    })
}

/// Creates a remote that is configured entirely by parameters (no browser sign-in).
pub async fn create(
    client: &RcClient,
    name: &str,
    rclone_type: &str,
    params: &BTreeMap<String, String>,
) -> AppResult<()> {
    client
        .call_value(
            "config/create",
            &json!({
                "name": name,
                "type": rclone_type,
                "parameters": params,
                "opt": { "nonInteractive": true, "obscure": true },
            }),
        )
        .await
        .map(|_| ())
}

/// Changes the given parameters on an existing remote. Keys not listed keep their values.
pub async fn update(client: &RcClient, name: &str, params: &BTreeMap<String, String>) -> AppResult<()> {
    client
        .call_value(
            "config/update",
            &json!({
                "name": name,
                "parameters": params,
                "opt": { "nonInteractive": true, "obscure": true },
            }),
        )
        .await
        .map(|_| ())
}

pub async fn delete(client: &RcClient, name: &str) -> AppResult<()> {
    client.call_value("config/delete", &json!({ "name": name })).await.map(|_| ())
}

/// Renames by copying the section under the new name and then deleting the old one.
/// Values are copied as stored (already obscured), so they are not obscured twice.
pub async fn rename(client: &RcClient, from: &str, to: &str) -> AppResult<()> {
    let mut raw = get_raw(client, from).await?;
    let rclone_type = raw
        .remove("type")
        .ok_or_else(|| AppError::invalid(format!("No remote named {from} was found.")))?;
    let params: Map<String, Value> = raw.into_iter().map(|(k, v)| (k, Value::String(v))).collect();
    client
        .call_value(
            "config/create",
            &json!({
                "name": to,
                "type": rclone_type,
                "parameters": params,
                "opt": { "nonInteractive": true, "noObscure": true },
            }),
        )
        .await?;
    delete(client, from).await
}

/// Checks that a remote can be listed. Uses the remote root, which is cheap on every backend.
pub async fn test(client: &RcClient, name: &str) -> AppResult<ConnectionTest> {
    let result = client
        .call_value_with_timeout(
            "operations/list",
            &json!({ "fs": format!("{name}:"), "remote": "" }),
            TEST_TIMEOUT,
        )
        .await;
    Ok(match result {
        Ok(_) => ConnectionTest {
            ok: true,
            message: "Connected.".to_owned(),
        },
        Err(err) if err.kind == ErrorKind::NotRunning => return Err(err),
        Err(err) => ConnectionTest {
            ok: false,
            message: friendly_connection_error(&err.message),
        },
    })
}

/// Turns rclone's technical errors into a sentence someone can act on.
pub fn friendly_connection_error(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let contains = |needles: &[&str]| needles.iter().any(|needle| lower.contains(needle));
    if contains(&["no such host", "lookup "]) {
        "Orbit could not find that server. Check the address.".to_owned()
    } else if contains(&["connection refused"]) {
        "The server refused the connection. Check the port, and that the service is running.".to_owned()
    } else if contains(&["timeout", "deadline exceeded", "timed out"]) {
        "The server did not answer in time. Check your network connection and the address.".to_owned()
    } else if contains(&["x509", "certificate"]) {
        "The server's security certificate could not be verified.".to_owned()
    } else if contains(&[
        "unauthorized",
        "401",
        "403",
        "accessdenied",
        "invalidaccesskeyid",
        "signaturedoesnotmatch",
        "permission denied",
        "unable to authenticate",
        "no supported methods remain",
        "invalid_grant",
    ]) {
        "The sign-in details were not accepted. Check the password, key or access keys.".to_owned()
    } else {
        let mut message = "Orbit could not reach this connection.".to_owned();
        let detail: String = raw.chars().take(240).collect();
        if !detail.is_empty() {
            message.push_str(&format!(" ({detail})"));
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_messages_cover_common_failures() {
        assert!(friendly_connection_error("dial tcp: lookup nope.example: no such host").contains("find that server"));
        assert!(friendly_connection_error("dial tcp 1.2.3.4:22: connect: connection refused").contains("refused"));
        assert!(friendly_connection_error("context deadline exceeded").contains("in time"));
        assert!(friendly_connection_error("ssh: unable to authenticate").contains("sign-in details"));
        assert!(friendly_connection_error("SignatureDoesNotMatch").contains("sign-in details"));
    }

    #[test]
    fn unknown_failures_keep_the_detail() {
        let message = friendly_connection_error("something odd happened");
        assert!(message.contains("something odd happened"));
    }
}
