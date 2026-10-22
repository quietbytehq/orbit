use super::FileEntry;
use super::local::{self, LocalEntry};
use super::ops;
use super::transfers::{Item, TransferQueue, TransferRequest};
use crate::error::AppResult;
use crate::rclone::Supervisor;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemRef {
    pub path: String,
    pub is_dir: bool,
}

#[tauri::command]
pub fn inspect_local_paths(paths: Vec<String>) -> AppResult<Vec<LocalEntry>> {
    local::inspect(&paths)
}

#[tauri::command]
pub async fn list_files(
    remote: String,
    path: String,
    supervisor: State<'_, Supervisor>,
) -> AppResult<Vec<FileEntry>> {
    ops::list(&supervisor.client()?, &remote, &path).await
}

#[tauri::command]
pub async fn create_folder(
    remote: String,
    path: String,
    supervisor: State<'_, Supervisor>,
) -> AppResult<()> {
    ops::create_folder(&supervisor.client()?, &remote, &path).await
}

/// Deletes each item in turn and stops at the first failure, so the user sees what went wrong.
#[tauri::command]
pub async fn delete_items(
    remote: String,
    items: Vec<ItemRef>,
    supervisor: State<'_, Supervisor>,
) -> AppResult<()> {
    let client = supervisor.client()?;
    for item in items {
        ops::delete(&client, &remote, &item.path, item.is_dir).await?;
    }
    Ok(())
}

#[tauri::command]
pub async fn rename_item(
    remote: String,
    path: String,
    is_dir: bool,
    new_name: String,
    supervisor: State<'_, Supervisor>,
) -> AppResult<String> {
    ops::rename(
        &supervisor.client()?,
        &remote,
        &path,
        is_dir,
        new_name.trim(),
    )
    .await
}

#[tauri::command]
pub fn enqueue_transfers(
    requests: Vec<TransferRequest>,
    queue: State<'_, Arc<TransferQueue>>,
) -> AppResult<Vec<u64>> {
    queue.enqueue(requests)
}

#[tauri::command]
pub fn list_transfers(queue: State<'_, Arc<TransferQueue>>) -> Vec<Item> {
    queue.snapshot()
}

#[tauri::command]
pub async fn cancel_transfer(id: u64, queue: State<'_, Arc<TransferQueue>>) -> AppResult<()> {
    queue.cancel(id).await
}

#[tauri::command]
pub fn clear_finished_transfers(queue: State<'_, Arc<TransferQueue>>) {
    queue.clear_finished();
}
