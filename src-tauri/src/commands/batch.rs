use std::path::PathBuf;

use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, AppResult};
use crate::models::batch::{BatchJob, BatchStatus};
use crate::models::edl::PolicyConfig;
use crate::pipeline::batch_worker::{list_videos_in_dir, run_batch_job};
use crate::pipeline::engine::policy_from_silence_options;
use crate::models::segment::SilenceDetectionOptions;
use crate::state::AppState;

fn update_job(app: &AppHandle, job: &BatchJob) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut map) = state.batch_jobs.lock() {
            map.insert(job.id.clone(), job.clone());
        }
    }
    let _ = app.emit("batch://progress", job.clone());
}

#[tauri::command]
pub fn queue_batch_job(
    media_paths: Vec<String>,
    output_dir: String,
    preset_id: Option<String>,
    auto_accept_exceptions: Option<bool>,
    options: Option<SilenceDetectionOptions>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<BatchJob> {
    if media_paths.is_empty() {
        return Err(AppError::Invalid("No media files for batch".into()));
    }
    std::fs::create_dir_all(&output_dir)?;

    let job = BatchJob::new(
        media_paths,
        preset_id.unwrap_or_else(|| "default".into()),
        output_dir,
        auto_accept_exceptions.unwrap_or(true),
    );

    state
        .batch_jobs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .insert(job.id.clone(), job.clone());

    let policy = options
        .as_ref()
        .map(policy_from_silence_options)
        .unwrap_or_default();

    let job_id = job.id.clone();
    let app_handle = app.clone();

    tauri::async_runtime::spawn(async move {
        tracing::info!("Batch worker started for {job_id}");
        // Reload job from state
        let initial = {
            let state = app_handle.state::<AppState>();
            state
                .batch_jobs
                .lock()
                .ok()
                .and_then(|m| m.get(&job_id).cloned())
        };
        let Some(job) = initial else {
            return;
        };

        // Process with intermediate updates
        let mut working = job;
        working.status = BatchStatus::Running;
        working.touch();
        update_job(&app_handle, &working);

        let total = working.media_paths.len().max(1);
        let export_policy = policy.clone();
        let out_dir = PathBuf::from(&working.output_dir);
        let auto_accept = working.auto_accept_exceptions;
        let export_opts = crate::models::preset::ExportOptions::default();
        let color = crate::models::preset::ColorOptions::default();

        for (i, path_str) in working.media_paths.clone().iter().enumerate() {
            working.current_file = Some(path_str.clone());
            working.progress = i as f64 / total as f64;
            working.touch();
            update_job(&app_handle, &working);

            let result = crate::pipeline::batch_worker::process_one_file(
                PathBuf::from(path_str).as_path(),
                &out_dir,
                &export_policy,
                auto_accept,
                &export_opts,
                &color,
            )
            .await;

            if result.ok {
                working.completed += 1;
            } else {
                working.failed += 1;
                if let Some(err) = &result.error {
                    working.errors.push(format!("{path_str}: {err}"));
                }
            }
            working.results.push(result);
            working.progress = (i + 1) as f64 / total as f64;
            working.touch();
            update_job(&app_handle, &working);
        }

        working.current_file = None;
        working.status = if working.completed == 0 {
            BatchStatus::Failed
        } else {
            BatchStatus::Completed
        };
        working.touch();
        update_job(&app_handle, &working);
        tracing::info!(
            "Batch {job_id} done: {} ok, {} failed",
            working.completed,
            working.failed
        );
        let _ = app_handle.emit("batch://done", &working);
    });

    Ok(job)
}

#[tauri::command]
pub fn get_batch_status(id: String, state: State<'_, AppState>) -> AppResult<BatchJob> {
    state
        .batch_jobs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .get(&id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("Batch job {id}")))
}

#[tauri::command]
pub fn list_batch_jobs(state: State<'_, AppState>) -> AppResult<Vec<BatchJob>> {
    let map = state
        .batch_jobs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut jobs: Vec<_> = map.values().cloned().collect();
    jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(jobs)
}

/// Scan a folder for videos and queue a factory batch → output_dir.
#[tauri::command]
pub fn queue_inbox_batch(
    inbox_dir: String,
    output_dir: Option<String>,
    auto_accept_exceptions: Option<bool>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<BatchJob> {
    let inbox = PathBuf::from(&inbox_dir);
    let videos = list_videos_in_dir(&inbox)?;
    if videos.is_empty() {
        return Err(AppError::Invalid(format!(
            "No videos found in {inbox_dir}"
        )));
    }
    let out = output_dir.unwrap_or_else(|| {
        inbox
            .parent()
            .map(|p| p.join("outbox").to_string_lossy().into_owned())
            .unwrap_or_else(|| {
                AppState::app_data_dir()
                    .map(|d| d.join("exports").to_string_lossy().into_owned())
                    .unwrap_or_else(|_| "outbox".into())
            })
    });
    let paths: Vec<String> = videos
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    queue_batch_job(
        paths,
        out,
        Some("factory".into()),
        auto_accept_exceptions,
        None,
        app,
        state,
    )
}

/// Synchronous batch for tests / internal use.
#[allow(dead_code)]
pub async fn run_batch_sync(job: BatchJob, policy: PolicyConfig) -> BatchJob {
    run_batch_job(job, policy).await
}
