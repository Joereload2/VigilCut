use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, AppResult};
use crate::models::batch::BatchJob;
use crate::models::edl::PolicyConfig;
use crate::models::preset::{ColorOptions, ExportOptions};
use crate::pipeline::batch_worker::{list_videos_in_dir, process_one_file};
use crate::state::AppState;

#[derive(Default)]
pub struct InboxWatchState {
    pub running: AtomicBool,
    pub processed: Mutex<HashSet<String>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub running: bool,
    pub inbox: String,
    pub outbox: String,
    pub processed_count: usize,
}

#[tauri::command]
pub fn get_inbox_watch_status(state: State<'_, InboxWatchState>) -> AppResult<WatchStatus> {
    let paths_root = AppState::app_data_dir()?;
    let processed = state
        .processed
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .len();
    Ok(WatchStatus {
        running: state.running.load(Ordering::SeqCst),
        inbox: paths_root.join("inbox").to_string_lossy().into_owned(),
        outbox: paths_root.join("outbox").to_string_lossy().into_owned(),
        processed_count: processed,
    })
}

#[tauri::command]
pub fn stop_inbox_watch(state: State<'_, InboxWatchState>) -> AppResult<()> {
    state.running.store(false, Ordering::SeqCst);
    tracing::info!("Inbox watch stop requested");
    Ok(())
}

/// Poll inbox every few seconds; process new videos into outbox (factory auto).
#[tauri::command]
pub fn start_inbox_watch(
    app: AppHandle,
    state: State<'_, InboxWatchState>,
) -> AppResult<WatchStatus> {
    if state.running.swap(true, Ordering::SeqCst) {
        return get_inbox_watch_status(state);
    }

    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        tracing::info!("Inbox watch started");
        let policy = PolicyConfig::default();
        let export_opts = ExportOptions::default();
        let color = ColorOptions::default();

        while app_handle
            .state::<InboxWatchState>()
            .running
            .load(Ordering::SeqCst)
        {
            let inbox = match AppState::app_data_dir() {
                Ok(r) => r.join("inbox"),
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            let outbox = match AppState::app_data_dir() {
                Ok(r) => r.join("outbox"),
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
            let _ = std::fs::create_dir_all(&inbox);
            let _ = std::fs::create_dir_all(&outbox);

            let videos = list_videos_in_dir(&inbox).unwrap_or_default();
            for path in videos {
                let key = path.to_string_lossy().into_owned();
                let already = {
                    let watch = app_handle.state::<InboxWatchState>();
                    let set = watch.processed.lock();
                    set.map(|s| s.contains(&key)).unwrap_or(true)
                };
                if already {
                    continue;
                }

                tracing::info!("Inbox watch processing {key}");
                let _ = app_handle.emit("watch://processing", serde_json::json!({ "path": key }));

                // Inbox watch defaults to Safe — never force-cut exceptions silently.
                let result = process_one_file(
                    &path,
                    &outbox,
                    &policy,
                    crate::models::exception_mode::ExceptionHandlingMode::Safe,
                    &export_opts,
                    &color,
                )
                .await;

                {
                    let watch = app_handle.state::<InboxWatchState>();
                    let mut set = watch.processed.lock().ok();
                    if let Some(ref mut s) = set {
                        s.insert(path.to_string_lossy().into_owned());
                    }
                }

                let _ = app_handle.emit("watch://done", &result);
                if result.ok {
                    // Move source to inbox/done to avoid reprocess if mtime changes
                    let done_dir = inbox.join("done");
                    let _ = std::fs::create_dir_all(&done_dir);
                    if let Some(name) = path.file_name() {
                        let dest = done_dir.join(name);
                        let _ = std::fs::rename(&path, &dest);
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(8)).await;
        }
        tracing::info!("Inbox watch stopped");
    });

    get_inbox_watch_status(state)
}

/// One-shot process of app inbox → outbox (same as CLI batch on factory dirs).
#[tauri::command]
pub async fn process_factory_inbox_now(app: AppHandle) -> AppResult<BatchJob> {
    let root = AppState::app_data_dir()?;
    let inbox = root.join("inbox");
    let outbox = root.join("outbox");
    std::fs::create_dir_all(&inbox)?;
    std::fs::create_dir_all(&outbox)?;
    let videos = list_videos_in_dir(&inbox)?;
    if videos.is_empty() {
        return Err(AppError::Invalid(
            "Inbox vacío. Copia vídeos a la carpeta inbox.".into(),
        ));
    }
    let paths: Vec<String> = videos
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();

    // Reuse queue_batch_job path via constructing job manually and running worker
    let job = BatchJob::new(
        paths,
        "factory-inbox".into(),
        outbox.to_string_lossy().into_owned(),
        crate::models::exception_mode::ExceptionHandlingMode::Safe,
    );
    let job_id = job.id.clone();
    {
        let state = app.state::<AppState>();
        state
            .batch_jobs
            .lock()
            .map_err(|e| AppError::Message(e.to_string()))?
            .insert(job.id.clone(), job.clone());
    }

    let handle = app.clone();
    let job_id_log = job_id.clone();
    tauri::async_runtime::spawn(async move {
        let policy = PolicyConfig::default();
        let done = crate::pipeline::batch_worker::run_batch_job(job, policy).await;
        if let Some(state) = handle.try_state::<AppState>() {
            if let Ok(mut map) = state.batch_jobs.lock() {
                map.insert(done.id.clone(), done.clone());
            }
        }
        let _ = handle.emit("batch://done", &done);
        tracing::info!("process_factory_inbox_now finished {job_id_log}");
    });

    let state = app.state::<AppState>();
    let jobs = state
        .batch_jobs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    jobs.get(&job_id).cloned().ok_or(AppError::NotFound(job_id))
}
