use super::runner::Runner;
use super::{JobInput, JobView};
use crate::error::AppResult;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn list_jobs(runner: State<'_, Arc<Runner>>) -> Vec<JobView> {
    runner.list()
}

#[tauri::command]
pub fn create_job(input: JobInput, runner: State<'_, Arc<Runner>>) -> AppResult<JobView> {
    runner.create(input)
}

#[tauri::command]
pub fn update_job(
    id: String,
    input: JobInput,
    runner: State<'_, Arc<Runner>>,
) -> AppResult<JobView> {
    runner.update(&id, input)
}

#[tauri::command]
pub fn set_job_enabled(id: String, enabled: bool, runner: State<'_, Arc<Runner>>) -> AppResult<()> {
    runner.set_enabled(&id, enabled)
}

#[tauri::command]
pub fn delete_job(id: String, runner: State<'_, Arc<Runner>>) -> AppResult<()> {
    runner.delete(&id)
}

#[tauri::command]
pub fn run_job(id: String, runner: State<'_, Arc<Runner>>) -> AppResult<String> {
    runner.run(&id)
}

#[tauri::command]
pub async fn cancel_job(id: String, runner: State<'_, Arc<Runner>>) -> AppResult<()> {
    runner.cancel(&id).await
}

#[tauri::command]
pub fn read_job_log(
    job_id: String,
    run_id: String,
    runner: State<'_, Arc<Runner>>,
) -> AppResult<String> {
    runner.read_log(&job_id, &run_id)
}
