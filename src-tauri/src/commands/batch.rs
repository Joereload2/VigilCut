use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::batch::{BatchJob, BatchStatus};
use crate::state::AppState;

#[tauri::command]
pub fn queue_batch_job(
    media_paths: Vec<String>,
    preset_id: String,
    output_dir: String,
    state: State<'_, AppState>,
) -> AppResult<BatchJob> {
    if media_paths.is_empty() {
        return Err(AppError::Invalid("No media files for batch".into()));
    }
    std::fs::create_dir_all(&output_dir)?;

    let job = BatchJob::new(media_paths, preset_id, output_dir);
    state
        .batch_jobs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .insert(job.id.clone(), job.clone());

    // MVP: job is queued; a worker task can be spawned in a later PR.
    // Status remains Queued so the UI can poll.
    tracing::info!("Batch job {} queued ({} files)", job.id, job.media_paths.len());
    Ok(job)
}

#[tauri::command]
pub fn get_batch_status(id: String, state: State<'_, AppState>) -> AppResult<BatchJob> {
    let jobs = state
        .batch_jobs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    jobs.get(&id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("Batch job {id}")))
}

/// Helper for future worker (not exposed as command yet).
#[allow(dead_code)]
pub fn mark_batch_running(job: &mut BatchJob) {
    job.status = BatchStatus::Running;
    job.updated_at = chrono::Utc::now();
}
