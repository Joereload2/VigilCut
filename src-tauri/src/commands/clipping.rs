//! Tauri commands for intelligent clipping.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::commands::analyze::AnalysisCache;
use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::job_control::JobControl;
use crate::models::clipping::{
    ClipCandidate, ClipExportResult, ClipFraming, ClipReviewStatus, ClippingOptions, ClippingRun,
};
use crate::models::progress;
use crate::pipeline::clipping::{
    export_approved_clips, export_one_clip, run_clipping_analysis_with_progress,
};

#[derive(Default)]
pub struct ClippingCache {
    pub runs: Mutex<HashMap<String, ClippingRun>>,
}

fn put_run(cache: &ClippingCache, run: ClippingRun) -> AppResult<ClippingRun> {
    let mut map = cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    map.insert(run.id.clone(), run.clone());
    Ok(run)
}

fn get_run(cache: &ClippingCache, id: &str) -> AppResult<ClippingRun> {
    let map = cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    map.get(id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("Clipping run {id}")))
}

fn take_mut<F, R>(cache: &ClippingCache, id: &str, f: F) -> AppResult<R>
where
    F: FnOnce(&mut ClippingRun) -> AppResult<R>,
{
    let mut map = cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    let run = map
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound(format!("Clipping run {id}")))?;
    f(run)
}

#[tauri::command]
pub async fn run_clipping(
    app: AppHandle,
    jobs: State<'_, JobControl>,
    media_path: String,
    options: Option<ClippingOptions>,
    analysis_run_id: Option<String>,
    cache: State<'_, ClippingCache>,
    analysis_cache: State<'_, AnalysisCache>,
) -> AppResult<ClippingRun> {
    jobs.begin();
    let opts = options.unwrap_or_default();
    // Sprint A1: reuse by run id (memory or disk) or by media path index.
    let reused = analysis_run_id
        .as_ref()
        .and_then(|id| {
            analysis_cache
                .runs
                .lock()
                .ok()
                .and_then(|m| m.get(id).cloned())
                .or_else(|| crate::commands::analyze::load_run_from_disk(id))
        })
        .or_else(|| {
            crate::commands::analyze::find_analysis_for_media(&analysis_cache, &media_path)
        });
    let mut on_prog = |stage: &str, message: &str, percent: f64| {
        progress::emit(&app, "clipping", stage, message, percent);
    };
    let run = run_clipping_analysis_with_progress(
        PathBuf::from(&media_path).as_path(),
        opts,
        reused.as_ref(),
        &mut on_prog,
    )
    .await;
    if jobs.is_cancelled() {
        return Err(AppError::Cancelled);
    }
    let run = run?;
    // Phase 3: durable vNext store (cache is no longer the source of truth).
    let project = crate::vnext::application::ensure_project_for_media(&run.media_path)?;
    crate::vnext::application::persist_clipping_run(&run, &project.id)?;
    put_run(&cache, run)
}

#[tauri::command]
pub fn get_clipping_run(run_id: String, cache: State<'_, ClippingCache>) -> AppResult<ClippingRun> {
    if let Ok(run) = get_run(&cache, &run_id) {
        return Ok(run);
    }
    // Reconstruct from SQLite after restart / cache miss.
    if let Some(run) = crate::vnext::application::load_run(&run_id)? {
        return put_run(&cache, run);
    }
    Err(AppError::NotFound(format!("Clipping run {run_id}")))
}

#[tauri::command]
pub fn update_clip_status(
    run_id: String,
    candidate_id: String,
    status: String,
    cache: State<'_, ClippingCache>,
) -> AppResult<ClipCandidate> {
    let st = parse_status(&status)?;
    // Persist decision first; then refresh cache from DB.
    let updated =
        crate::vnext::application::apply_status_decision(&run_id, &candidate_id, st, None)?;
    let _ = sync_run_cache_from_db(&cache, &run_id);
    // Also patch cache if present for snappy UI
    let _ = take_mut(&cache, &run_id, |run| {
        if let Some(c) = run.candidates.iter_mut().find(|c| c.id == candidate_id) {
            *c = updated.clone();
        }
        Ok(())
    });
    Ok(updated)
}

#[tauri::command]
pub fn update_clip_span(
    run_id: String,
    candidate_id: String,
    start: f64,
    end: f64,
    cache: State<'_, ClippingCache>,
) -> AppResult<ClipCandidate> {
    let updated =
        crate::vnext::application::apply_span_decision(&run_id, &candidate_id, start, end)?;
    let _ = take_mut(&cache, &run_id, |run| {
        if let Some(c) = run.candidates.iter_mut().find(|c| c.id == candidate_id) {
            *c = updated.clone();
        }
        Ok(())
    });
    Ok(updated)
}

#[tauri::command]
pub fn update_clip_framing(
    run_id: String,
    candidate_id: String,
    framing: ClipFraming,
    cache: State<'_, ClippingCache>,
) -> AppResult<ClipCandidate> {
    let updated =
        crate::vnext::application::apply_framing_decision(&run_id, &candidate_id, framing)?;
    let _ = take_mut(&cache, &run_id, |run| {
        if let Some(c) = run.candidates.iter_mut().find(|c| c.id == candidate_id) {
            *c = updated.clone();
        }
        Ok(())
    });
    Ok(updated)
}

fn sync_run_cache_from_db(cache: &ClippingCache, run_id: &str) -> AppResult<()> {
    if let Some(run) = crate::vnext::application::load_run(run_id)? {
        put_run(cache, run)?;
    }
    Ok(())
}

#[tauri::command]
pub fn bulk_clip_status(
    run_id: String,
    status: String,
    only_high_confidence: bool,
    cache: State<'_, ClippingCache>,
) -> AppResult<ClippingRun> {
    let st = parse_status(&status)?;
    let run = take_mut(&cache, &run_id, |run| {
        for c in run.candidates.iter_mut() {
            if !c.is_primary_variant {
                continue;
            }
            if matches!(
                c.status,
                ClipReviewStatus::Discarded | ClipReviewStatus::Exported
            ) {
                continue;
            }
            if only_high_confidence && (c.score < 72.0 || c.confidence < 0.55) {
                continue;
            }
            if matches!(
                c.status,
                ClipReviewStatus::Preselected
                    | ClipReviewStatus::Suggested
                    | ClipReviewStatus::Modified
            ) {
                c.status = st;
            }
        }
        Ok(run.clone())
    })?;
    // Durable snapshot of bulk status changes
    if let Ok(project) = crate::vnext::application::ensure_project_for_media(&run.media_path) {
        let _ = crate::vnext::application::persist_clipping_run(&run, &project.id);
    }
    Ok(run)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportClipsResponse {
    pub results: Vec<ClipExportResult>,
    pub output_dir: String,
    pub run: ClippingRun,
}

#[tauri::command]
pub async fn export_clips(
    run_id: String,
    output_dir: String,
    candidate_ids: Option<Vec<String>>,
    framing_override: Option<ClipFraming>,
    cache: State<'_, ClippingCache>,
) -> AppResult<ExportClipsResponse> {
    let mut run = get_run(&cache, &run_id)?;
    let media = PathBuf::from(&run.media_path);
    let out = PathBuf::from(&output_dir);
    std::fs::create_dir_all(&out)?;

    let ffmpeg = Ffmpeg::new()?;
    let info = ffmpeg.probe(&media).await?;
    let ids = candidate_ids.unwrap_or_default();

    let results = export_approved_clips(
        &media,
        &mut run.candidates,
        &ids,
        &out,
        framing_override.as_ref(),
        info.width,
        info.height,
    )
    .await?;

    for c in &run.candidates {
        let _ = crate::vnext::application::sync_candidate_state(c, &run_id);
    }
    let run = put_run(&cache, run)?;
    Ok(ExportClipsResponse {
        results,
        output_dir,
        run,
    })
}

/// Promote a secondary variant to primary within its group.
#[tauri::command]
pub fn promote_clip_variant(
    run_id: String,
    candidate_id: String,
    cache: State<'_, ClippingCache>,
) -> AppResult<ClippingRun> {
    let run = take_mut(&cache, &run_id, |run| {
        let gid = run
            .candidates
            .iter()
            .find(|c| c.id == candidate_id)
            .map(|c| c.variant_group_id.clone())
            .ok_or_else(|| AppError::NotFound(candidate_id.clone()))?;
        for c in run.candidates.iter_mut() {
            if c.variant_group_id == gid {
                c.is_primary_variant = c.id == candidate_id;
            }
        }
        Ok(run.clone())
    })?;
    if let Ok(project) = crate::vnext::application::ensure_project_for_media(&run.media_path) {
        let _ = crate::vnext::application::persist_clipping_run(&run, &project.id);
    }
    Ok(run)
}

#[tauri::command]
pub async fn export_single_clip(
    run_id: String,
    candidate_id: String,
    output_path: String,
    cache: State<'_, ClippingCache>,
) -> AppResult<ClipExportResult> {
    let mut run = get_run(&cache, &run_id)?;
    let media = PathBuf::from(&run.media_path);
    let out = PathBuf::from(&output_path);
    let ffmpeg = Ffmpeg::new()?;
    let info = ffmpeg.probe(&media).await?;

    let c = run
        .candidates
        .iter_mut()
        .find(|c| c.id == candidate_id)
        .ok_or_else(|| AppError::NotFound(candidate_id.clone()))?;

    match export_one_clip(&media, c, &out, &c.framing, info.width, info.height).await {
        Ok(p) => {
            c.status = ClipReviewStatus::Exported;
            c.export_path = Some(p.to_string_lossy().into_owned());
            let res = ClipExportResult {
                candidate_id: c.id.clone(),
                ok: true,
                output_path: Some(p.to_string_lossy().into_owned()),
                error: None,
            };
            let _ = crate::vnext::application::sync_candidate_state(c, &run_id);
            put_run(&cache, run)?;
            Ok(res)
        }
        Err(e) => {
            c.status = ClipReviewStatus::Error;
            c.error = Some(e.to_string());
            let _ = crate::vnext::application::sync_candidate_state(c, &run_id);
            put_run(&cache, run)?;
            Ok(ClipExportResult {
                candidate_id,
                ok: false,
                output_path: None,
                error: Some(e.to_string()),
            })
        }
    }
}

fn parse_status(s: &str) -> AppResult<ClipReviewStatus> {
    Ok(match s {
        "suggested" => ClipReviewStatus::Suggested,
        "preselected" => ClipReviewStatus::Preselected,
        "approved" => ClipReviewStatus::Approved,
        "rejected" => ClipReviewStatus::Rejected,
        "modified" => ClipReviewStatus::Modified,
        "discarded" => ClipReviewStatus::Discarded,
        "exported" => ClipReviewStatus::Exported,
        other => {
            return Err(AppError::Invalid(format!("Unknown clip status: {other}")));
        }
    })
}
