use crate::error::{AppError, AppResult, ErrorKind};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::time::Duration;

/// Default time limit for an RC call. Long-running work uses `_async` and polls `job/status`.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Connection details for a running `rclone rcd`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub port: u16,
    pub user: String,
    pub pass: String,
}

/// Talks to `rclone rcd` over its JSON RC API.
#[derive(Clone)]
pub struct RcClient {
    http: reqwest::Client,
    base_url: String,
    user: String,
    pass: String,
}

// Hand-written so the RC password never ends up in logs or debug output.
impl std::fmt::Debug for RcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RcClient")
            .field("base_url", &self.base_url)
            .field("user", &self.user)
            .finish_non_exhaustive()
    }
}

impl RcClient {
    pub fn new(endpoint: &Endpoint) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(DEFAULT_TIMEOUT)
                .build()
                .expect("reqwest client with default settings builds"),
            base_url: format!("http://127.0.0.1:{}", endpoint.port),
            user: endpoint.user.clone(),
            pass: endpoint.pass.clone(),
        }
    }

    /// Calls an RC endpoint and returns the raw JSON response.
    pub async fn call_value(&self, endpoint: &str, params: &Value) -> AppResult<Value> {
        self.call_value_with_timeout(endpoint, params, DEFAULT_TIMEOUT)
            .await
    }

    pub async fn call_value_with_timeout(
        &self,
        endpoint: &str,
        params: &Value,
        timeout: Duration,
    ) -> AppResult<Value> {
        let url = format!("{}/{}", self.base_url, endpoint.trim_start_matches('/'));
        let response = self
            .http
            .post(&url)
            .basic_auth(&self.user, Some(&self.pass))
            .timeout(timeout)
            .json(params)
            .send()
            .await
            .map_err(transport_error)?;

        let status = response.status();
        let body = response.text().await.map_err(transport_error)?;
        let parsed: Value = if body.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&body).map_err(|_| {
                AppError::new(
                    ErrorKind::Rclone,
                    format!(
                        "rclone returned an unreadable answer to {endpoint} (HTTP {})",
                        status.as_u16()
                    ),
                )
            })?
        };

        if status.is_success() {
            Ok(parsed)
        } else {
            Err(AppError::new(
                ErrorKind::Rclone,
                parsed
                    .get("error")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        format!("rclone failed on {endpoint} (HTTP {})", status.as_u16())
                    }),
            ))
        }
    }

    /// Calls an RC endpoint and deserializes the response into `T`.
    pub async fn call<T: DeserializeOwned>(&self, endpoint: &str, params: &Value) -> AppResult<T> {
        let value = self.call_value(endpoint, params).await?;
        Ok(serde_json::from_value(value)?)
    }
}

fn transport_error(err: reqwest::Error) -> AppError {
    if err.is_connect() || err.is_timeout() {
        AppError::new(
            ErrorKind::NotRunning,
            "Orbit cannot reach rclone right now.",
        )
    } else {
        AppError::new(ErrorKind::Rclone, format!("rclone request failed: {err}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_hides_password() {
        let client = RcClient::new(&Endpoint {
            port: 5572,
            user: "orbit".into(),
            pass: "super-secret".into(),
        });
        let text = format!("{client:?}");
        assert!(!text.contains("super-secret"));
        assert!(text.contains("127.0.0.1:5572"));
    }

    #[tokio::test]
    async fn unreachable_port_reports_not_running() {
        // Bind and drop to get a port that is almost certainly closed.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let client = RcClient::new(&Endpoint {
            port,
            user: "orbit".into(),
            pass: "x".into(),
        });
        let err = client
            .call_value("core/version", &json!({}))
            .await
            .unwrap_err();
        assert_eq!(err.kind, ErrorKind::NotRunning);
    }
}
