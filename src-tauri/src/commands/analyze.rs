use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::error::{AppError, AppResult};
use crate::models::analysis::{AnalysisRun, ResolveExceptionRequest};
use crate::models::edl::PolicyConfig;
use crate::models::progress;
use crate::models::segment::SilenceDetectionOptions;
use crate::pipeline::engine::run_silence_analysis_with_progress;
use crate::pipeline::{
    accept_all_exceptions, policy_from_silence_options, reject_all_exceptions, resolve_exception,
};
use crate::state::AppState;

/// In-memory last runs (job cache for exception resolve).
#[derive(Default)]
pub struct AnalysisCache {
    pub runs: Mutex<std::collections::HashMap<String, AnalysisRun>>,
}

fn load_run_from_disk(run_id: &str) -> Option<AnalysisRun> {
    let file = AppState::cache_dir()
        .ok()?
        .join("runs")
        .join(format!("{run_id}.json"));
    let data = std::fs::read_to_string(file).ok()?;
    serde_json::from_str(&data).ok()
}

fn persist_run(run: &AnalysisRun) {
    if let Ok(dir) = AppState::cache_dir() {
        let runs_dir = dir.join("runs");
        let _ = std::fs::create_dir_all(&runs_dir);
        let file = runs_dir.join(format!("{}.json", run.id));
        if let Ok(json) = serde_json::to_string_pretty(run) {
            let _ = std::fs::write(file, json);
        }
    }
}

fn take_run(cache: &AnalysisCache, run_id: &str) -> AppResult<AnalysisRun> {
    let mut map = cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    if let Some(run) = map.remove(run_id) {
        return Ok(run);
    }
    drop(map);
    load_run_from_disk(run_id).ok_or_else(|| AppError::NotFound(format!("Analysis run {run_id}")))
}

fn put_run(cache: &AnalysisCache, run: AnalysisRun) -> AppResult<AnalysisRun> {
    persist_run(&run);
    cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .insert(run.id.clone(), run.clone());
    Ok(run)
}

#[tauri::command]
pub async fn run_analysis(
    app: AppHandle,
    path: String,
    options: Option<SilenceDetectionOptions>,
    policy: Option<PolicyConfig>,
    cache: State<'_, AnalysisCache>,
) -> AppResult<AnalysisRun> {
    let pol = policy.unwrap_or_else(|| {
        options
            .as_ref()
            .map(policy_from_silence_options)
            .unwrap_or_default()
    });

    let mut on_prog = |stage: &str, message: &str, percent: f64| {
        progress::emit(&app, "analysis", stage, message, percent);
    };
    let run = run_silence_analysis_with_progress(
        PathBuf::from(&path).as_path(),
        &pol,
        &mut on_prog,
    )
    .await?;
    put_run(&cache, run)
}

#[tauri::command]
pub fn get_analysis_run(run_id: String, cache: State<'_, AnalysisCache>) -> AppResult<AnalysisRun> {
    let map = cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    if let Some(run) = map.get(&run_id) {
        return Ok(run.clone());
    }
    drop(map);
    load_run_from_disk(&run_id).ok_or_else(|| AppError::NotFound(format!("Analysis run {run_id}")))
}

#[tauri::command]
pub fn resolve_analysis_exception(
    run_id: String,
    request: ResolveExceptionRequest,
    cache: State<'_, AnalysisCache>,
) -> AppResult<AnalysisRun> {
    let run = take_run(&cache, &run_id)?;
    let accept = matches!(
        request.resolution.as_str(),
        "accepted" | "accept" | "cut" | "yes"
    );
    let updated = resolve_exception(run, &request.exception_id, accept);
    put_run(&cache, updated)
}

#[tauri::command]
pub fn resolve_all_exceptions(
    run_id: String,
    accept: bool,
    cache: State<'_, AnalysisCache>,
) -> AppResult<AnalysisRun> {
    let run = take_run(&cache, &run_id)?;
    let updated = if accept {
        accept_all_exceptions(run)
    } else {
        reject_all_exceptions(run)
    };
    put_run(&cache, updated)
}
