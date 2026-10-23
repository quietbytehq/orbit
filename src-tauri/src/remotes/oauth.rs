//! Browser sign-in through rclone's own flow.
//!
//! `config/create` without `nonInteractive` makes rclone start its callback server on
//! 127.0.0.1:53682 and wait for the browser to return. While it waits, `config/oauthstatus`
//! reports the sign-in URL. The call returns when sign-in finishes, fails, or is stopped.

use super::store;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::RcClient;
use crate::text;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use std::time::Duration;

/// Generous limit for a person to find their account and approve access.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const URL_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OauthStatus {
    status: String,
    auth_url: Option<String>,
}

/// Creates a remote that signs in through the browser.
///
/// `on_url` is called once with the sign-in URL, as soon as rclone reports it.
pub async fn create_with_browser(
    client: &RcClient,
    name: &str,
    rclone_type: &str,
    params: &BTreeMap<String, String>,
    on_url: impl Fn(&str),
) -> AppResult<()> {
    let body = json!({
        "name": name,
        "type": rclone_type,
        "parameters": params,
        "opt": { "obscure": true },
    });
    let create = client.call_value_with_timeout("config/create", &body, SIGN_IN_TIMEOUT);
    tokio::pin!(create);

    let mut ticker = tokio::time::interval(URL_POLL_INTERVAL);
    let mut announced = false;
    loop {
        tokio::select! {
            result = &mut create => {
                let Err(err) = result else {
                    return Ok(());
                };
                // rclone writes the section before the token exchange, so a failed sign-in
                // leaves an empty remote behind. The name was free before we started, so remove it.
                discard_stub(client, name).await;
                return Err(if err.kind == ErrorKind::Rclone {
                    AppError::new(ErrorKind::Rclone, text::SIGN_IN_FAILED)
                        .with_details(err.details.unwrap_or(err.message))
                } else {
                    err
                });
            }
            _ = ticker.tick(), if !announced => {
                if let Some(url) = sign_in_url(client).await {
                    on_url(&url);
                    announced = true;
                }
            }
        }
    }
}

async fn sign_in_url(client: &RcClient) -> Option<String> {
    let status: OauthStatus = client.call("config/oauthstatus", &json!({})).await.ok()?;
    if status.status == "running" {
        status.auth_url
    } else {
        None
    }
}

async fn discard_stub(client: &RcClient, name: &str) {
    if store::exists(client, name).await.unwrap_or(false) {
        let _ = store::delete(client, name).await;
    }
}

/// Stops a sign-in that is waiting for the browser. The pending create call then fails.
pub async fn cancel(client: &RcClient) -> AppResult<()> {
    client
        .call_value("config/oauthstop", &json!({}))
        .await
        .map(|_| ())
}
