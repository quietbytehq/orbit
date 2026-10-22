use super::health::{HealthStore, RemoteHealth};
use super::providers::{self, AuthKind, ProviderDef};
use super::{ConnectionTest, RemoteForm, RemoteSummary, oauth, store};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::rclone::Supervisor;
use serde::Deserialize;
use std::collections::BTreeMap;
use tauri::{AppHandle, Emitter, State};

/// Emitted with the sign-in URL while a browser sign-in is waiting.
pub const SIGN_IN_URL_EVENT: &str = "wizard://sign-in-url";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRemote {
    pub provider_id: String,
    pub name: String,
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteUpdate {
    #[serde(default)]
    pub values: BTreeMap<String, String>,
}

fn summary(provider: &ProviderDef, name: &str) -> RemoteSummary {
    RemoteSummary {
        name: name.to_owned(),
        rclone_type: provider.rclone_type.to_owned(),
        provider_id: Some(provider.id.to_owned()),
        provider_title: provider.title.to_owned(),
        health: RemoteHealth::Unknown,
    }
}

fn credentials_provider(id: &str) -> AppResult<&'static ProviderDef> {
    let provider = providers::find(id).ok_or_else(|| AppError::invalid("Unknown service."))?;
    if provider.auth != AuthKind::Credentials {
        return Err(AppError::invalid(
            "This service signs in through the browser.",
        ));
    }
    Ok(provider)
}

async fn ensure_name_free(client: &crate::rclone::RcClient, name: &str) -> AppResult<()> {
    if store::exists(client, name).await? {
        return Err(AppError::invalid(format!(
            "A connection named {name} already exists."
        )));
    }
    Ok(())
}

#[tauri::command]
pub fn list_providers() -> &'static [ProviderDef] {
    providers::all()
}

#[tauri::command]
pub async fn list_remotes(
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<Vec<RemoteSummary>> {
    let client = supervisor.client()?;
    let mut remotes = store::list(&client).await?;
    for remote in &mut remotes {
        remote.health = health.get(&remote.name);
    }
    Ok(remotes)
}

#[tauri::command]
pub async fn get_remote_form(
    name: String,
    supervisor: State<'_, Supervisor>,
) -> AppResult<RemoteForm> {
    store::form(&supervisor.client()?, &name).await
}

#[tauri::command]
pub async fn create_remote(
    request: NewRemote,
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<RemoteSummary> {
    let provider = credentials_provider(&request.provider_id)?;
    let name = providers::validate_remote_name(&request.name)?;
    let params = providers::build_parameters(provider, &request.values, None)?;
    let client = supervisor.client()?;
    ensure_name_free(&client, &name).await?;
    store::create(&client, &name, provider.rclone_type, &params).await?;
    health.forget(&name);
    Ok(summary(provider, &name))
}

/// Runs rclone's browser sign-in for a Google Drive, OneDrive or Dropbox remote.
/// Returns when sign-in completes. Emits `wizard://sign-in-url` with the URL to open.
#[tauri::command]
pub async fn start_browser_sign_in(
    request: NewRemote,
    app: AppHandle,
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<RemoteSummary> {
    let provider = providers::find(&request.provider_id)
        .ok_or_else(|| AppError::invalid("Unknown service."))?;
    if provider.auth != AuthKind::Browser {
        return Err(AppError::invalid(
            "This service uses keys or a password, not a browser sign-in.",
        ));
    }
    let name = providers::validate_remote_name(&request.name)?;
    let params = providers::build_parameters(provider, &request.values, None)?;
    let client = supervisor.client()?;
    ensure_name_free(&client, &name).await?;
    oauth::create_with_browser(&client, &name, provider.rclone_type, &params, |url| {
        let _ = app.emit(SIGN_IN_URL_EVENT, url);
    })
    .await?;
    health.forget(&name);
    Ok(summary(provider, &name))
}

#[tauri::command]
pub async fn cancel_browser_sign_in(supervisor: State<'_, Supervisor>) -> AppResult<()> {
    oauth::cancel(&supervisor.client()?).await
}

#[tauri::command]
pub async fn update_remote(
    name: String,
    update: RemoteUpdate,
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<()> {
    let client = supervisor.client()?;
    let form = store::form(&client, &name).await?;
    let provider = providers::find(&form.provider_id)
        .ok_or_else(|| AppError::new(ErrorKind::Unsupported, "Unknown service."))?;
    let params = providers::build_parameters(provider, &update.values, Some(&form.saved_secrets))?;
    store::update(&client, &name, &params).await?;
    health.forget(&name);
    Ok(())
}

#[tauri::command]
pub async fn rename_remote(
    name: String,
    new_name: String,
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<String> {
    let new_name = providers::validate_remote_name(&new_name)?;
    if new_name == name {
        return Ok(name);
    }
    let client = supervisor.client()?;
    ensure_name_free(&client, &new_name).await?;
    store::rename(&client, &name, &new_name).await?;
    health.rename(&name, &new_name);
    Ok(new_name)
}

#[tauri::command]
pub async fn delete_remote(
    name: String,
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<()> {
    store::delete(&supervisor.client()?, &name).await?;
    health.forget(&name);
    Ok(())
}

#[tauri::command]
pub async fn test_remote(
    name: String,
    supervisor: State<'_, Supervisor>,
    health: State<'_, HealthStore>,
) -> AppResult<ConnectionTest> {
    let result = store::test(&supervisor.client()?, &name).await?;
    health.set(
        &name,
        if result.ok {
            RemoteHealth::Ok
        } else {
            RemoteHealth::Failed {
                message: result.message.clone(),
            }
        },
    );
    Ok(result)
}
