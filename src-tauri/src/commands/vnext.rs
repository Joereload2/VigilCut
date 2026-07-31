//! Tauri API for VigilCut vNext (typed use-cases). No FFmpeg details exposed.

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::clipping::{ClipCandidate, ClipFraming, ClipReviewStatus};
use crate::vnext::application::{
    apply_framing_decision, apply_span_decision, apply_status_decision, cancel_job,
    create_plan_for_candidate_id, ensure_project_for_media, enqueue_vertical_render,
    execute_vertical_render_job, list_review_history, load_run, persist_clipping_run,
    project_next_action, retry_job,
};
use crate::vnext::domain::{
    ContentProjectRecord, JobKind, ReviewDecisionRecord, SubtitleCueV1, VerticalRenderPlanV1,
};
use crate::vnext::persistence::{
    get_job, get_project, list_artifacts_for_project, list_candidates_for_project,
    list_jobs_for_project, list_projects, list_recent_jobs, list_run_ids_for_project, ArtifactRow,
    JobRow,
};

// ── DTOs (camelCase for TS) ────────────────────────────────────────────────

/// Reserved for typed error responses in later API hardening.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationErrorDto {
    pub contract_version: &'static str,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

/// Reserved for progress events carrying job_id (JobProgressV1).
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressDto {
    pub contract_version: &'static str,
    pub job_id: String,
    pub content_project_id: String,
    pub status: String,
    pub stage: String,
    pub message: String,
    pub percent: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentProjectDto {
    pub contract_version: &'static str,
    pub id: String,
    pub title: String,
    pub client_label: Option<String>,
    pub privacy_mode: String,
    pub status: String,
    pub source_media_path: String,
    pub work_dir: String,
    pub next_action: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub contract_version: &'static str,
    pub id: String,
    pub content_project_id: String,
    pub kind: String,
    pub status: String,
    pub idempotency_key: String,
    pub attempt: i64,
    pub max_attempts: i64,
    pub stage: String,
    pub progress_pct: f64,
    pub result_artifact_id: Option<String>,
    pub render_plan_id: Option<String>,
    pub error_json: Option<String>,
    pub cancel_requested: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDto {
    pub contract_version: &'static str,
    pub id: String,
    pub content_project_id: String,
    pub kind: String,
    pub role: String,
    pub path: String,
    pub sha256: String,
    pub byte_size: i64,
    pub mime_type: String,
    pub created_by_job_id: String,
    pub validation_status: String,
    pub validation_notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub title: Option<String>,
    pub source_media_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReviewDecisionRequest {
    pub run_id: String,
    pub candidate_id: String,
    /// approve | reject | modify_span | modify_framing
    pub decision: String,
    pub reason: Option<String>,
    pub start_s: Option<f64>,
    pub end_s: Option<f64>,
    pub framing: Option<ClipFraming>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRenderPlanRequest {
    pub run_id: String,
    pub candidate_id: String,
    pub burn_in: Option<bool>,
    pub cues: Option<Vec<SubtitleCueDto>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleCueDto {
    pub id: Option<String>,
    pub start_s: f64,
    pub end_s: f64,
    pub text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    pub content_project_id: String,
    pub kind: String,
    pub idempotency_key: String,
    pub input_json: Option<String>,
}

fn map_project(p: ContentProjectRecord, next: Option<String>) -> ContentProjectDto {
    ContentProjectDto {
        contract_version: "v1",
        id: p.id,
        title: p.title,
        client_label: p.client_label,
        privacy_mode: p.privacy_mode,
        status: p.status,
        source_media_path: p.source_media_path,
        work_dir: p.work_dir,
        next_action: next,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

fn map_job(j: JobRow) -> JobDto {
    JobDto {
        contract_version: "v1",
        id: j.id,
        content_project_id: j.content_project_id,
        kind: j.kind,
        status: j.status,
        idempotency_key: j.idempotency_key,
        attempt: j.attempt,
        max_attempts: j.max_attempts,
        stage: j.stage,
        progress_pct: j.progress_pct,
        result_artifact_id: j.result_artifact_id,
        render_plan_id: j.render_plan_id,
        error_json: j.error_json,
        cancel_requested: j.cancel_requested,
        created_at: j.created_at,
        updated_at: j.updated_at,
    }
}

fn map_artifact(a: ArtifactRow) -> ArtifactDto {
    ArtifactDto {
        contract_version: "v1",
        id: a.id,
        content_project_id: a.content_project_id,
        kind: a.kind,
        role: a.role,
        path: a.path,
        sha256: a.sha256,
        byte_size: a.byte_size,
        mime_type: a.mime_type,
        created_by_job_id: a.created_by_job_id,
        validation_status: a.validation_status,
        validation_notes: a.validation_notes,
        created_at: a.created_at,
    }
}

fn parse_status_decision(s: &str) -> AppResult<ClipReviewStatus> {
    Ok(match s {
        "approve" | "approved" => ClipReviewStatus::Approved,
        "reject" | "rejected" => ClipReviewStatus::Rejected,
        "discarded" => ClipReviewStatus::Discarded,
        "modified" => ClipReviewStatus::Modified,
        other => {
            return Err(AppError::Invalid(format!("decision desconocida: {other}")));
        }
    })
}

// ── Commands ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn vnext_create_content_project(req: CreateProjectRequest) -> AppResult<ContentProjectDto> {
    let path = req.source_media_path.trim();
    if path.is_empty() {
        return Err(AppError::Invalid("sourceMediaPath requerido".into()));
    }
    let mut p = ensure_project_for_media(path)?;
    if let Some(t) = req.title {
        if !t.trim().is_empty() && t != p.title {
            // lightweight title update via re-insert not available — store as-is for MVP
            let _ = t;
        }
    }
    let next = project_next_action(&p.id)
        .ok()
        .map(|a| a.as_str().to_string());
    // refresh
    if let Some(fresh) = get_project(&p.id)? {
        p = fresh;
    }
    Ok(map_project(p, next))
}

#[tauri::command]
pub fn vnext_list_content_projects(limit: Option<u32>) -> AppResult<Vec<ContentProjectDto>> {
    let list = list_projects(limit.unwrap_or(50) as usize)?;
    let mut out = Vec::new();
    for p in list {
        let next = project_next_action(&p.id)
            .ok()
            .map(|a| a.as_str().to_string());
        out.push(map_project(p, next));
    }
    Ok(out)
}

#[tauri::command]
pub fn vnext_get_content_project(id: String) -> AppResult<ContentProjectDto> {
    let p = get_project(&id)?.ok_or_else(|| AppError::NotFound(id.clone()))?;
    let next = project_next_action(&p.id)
        .ok()
        .map(|a| a.as_str().to_string());
    Ok(map_project(p, next))
}

#[tauri::command]
pub fn vnext_create_job(req: CreateJobRequest) -> AppResult<JobDto> {
    let kind = JobKind::parse(&req.kind).map_err(|e| AppError::Invalid(e.to_string()))?;
    let job = crate::vnext::application::enqueue(
        &req.content_project_id,
        kind,
        &req.idempotency_key,
        req.input_json.as_deref().unwrap_or("{}"),
        2,
    )?;
    Ok(map_job(job))
}

#[tauri::command]
pub fn vnext_get_job(id: String) -> AppResult<JobDto> {
    let j = get_job(&id)?.ok_or(AppError::NotFound(id))?;
    Ok(map_job(j))
}

#[tauri::command]
pub fn vnext_list_project_jobs(
    content_project_id: String,
    limit: Option<u32>,
) -> AppResult<Vec<JobDto>> {
    Ok(list_jobs_for_project(&content_project_id, limit.unwrap_or(100) as usize)?
        .into_iter()
        .map(map_job)
        .collect())
}

#[tauri::command]
pub fn vnext_list_queue_jobs(limit: Option<u32>) -> AppResult<Vec<JobDto>> {
    Ok(list_recent_jobs(limit.unwrap_or(100) as usize)?
        .into_iter()
        .map(map_job)
        .collect())
}

#[tauri::command]
pub fn vnext_retry_job(id: String) -> AppResult<JobDto> {
    Ok(map_job(retry_job(&id)?))
}

#[tauri::command]
pub fn vnext_cancel_job(id: String) -> AppResult<JobDto> {
    Ok(map_job(cancel_job(&id)?))
}

#[tauri::command]
pub fn vnext_list_short_candidates(content_project_id: String) -> AppResult<Vec<ClipCandidate>> {
    list_candidates_for_project(&content_project_id)
}

#[tauri::command]
pub fn vnext_list_clipping_runs(content_project_id: String) -> AppResult<Vec<String>> {
    list_run_ids_for_project(&content_project_id)
}

#[tauri::command]
pub fn vnext_get_clipping_run(run_id: String) -> AppResult<crate::models::clipping::ClippingRun> {
    load_run(&run_id)?.ok_or(AppError::NotFound(run_id))
}

#[tauri::command]
pub fn vnext_save_review_decision(req: SaveReviewDecisionRequest) -> AppResult<ClipCandidate> {
    match req.decision.as_str() {
        "modify_span" => {
            let start = req
                .start_s
                .ok_or_else(|| AppError::Invalid("startS requerido".into()))?;
            let end = req
                .end_s
                .ok_or_else(|| AppError::Invalid("endS requerido".into()))?;
            apply_span_decision(&req.run_id, &req.candidate_id, start, end)
        }
        "modify_framing" => {
            let framing = req
                .framing
                .ok_or_else(|| AppError::Invalid("framing requerido".into()))?;
            apply_framing_decision(&req.run_id, &req.candidate_id, framing)
        }
        other => {
            let st = parse_status_decision(other)?;
            apply_status_decision(&req.run_id, &req.candidate_id, st, req.reason)
        }
    }
}

#[tauri::command]
pub fn vnext_list_review_decisions(candidate_id: String) -> AppResult<Vec<ReviewDecisionRecord>> {
    list_review_history(&candidate_id)
}

#[tauri::command]
pub fn vnext_create_vertical_render_plan(
    req: CreateRenderPlanRequest,
) -> AppResult<VerticalRenderPlanV1> {
    let cues: Vec<SubtitleCueV1> = req
        .cues
        .unwrap_or_default()
        .into_iter()
        .map(|c| SubtitleCueV1 {
            id: c.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            start_s: c.start_s,
            end_s: c.end_s,
            text: c.text,
        })
        .collect();
    create_plan_for_candidate_id(
        &req.run_id,
        &req.candidate_id,
        cues,
        req.burn_in.unwrap_or(true),
    )
}

#[tauri::command]
pub async fn vnext_start_vertical_render(plan_id: String) -> AppResult<JobDto> {
    let plan = crate::vnext::persistence::get_render_plan(&plan_id)?
        .ok_or_else(|| AppError::NotFound(plan_id.clone()))?;
    let job_id = enqueue_vertical_render(&plan.id, &plan.content_project_id)?;
    // Execute in-process — surface FFmpeg/path errors to the UI (do not swallow)
    if let Err(e) = execute_vertical_render_job(&job_id).await {
        // Job row already marked failed; still return a useful error to the client
        return Err(e);
    }
    let j = get_job(&job_id)?.ok_or(AppError::NotFound(job_id))?;
    Ok(map_job(j))
}

#[tauri::command]
pub fn vnext_list_project_artifacts(
    content_project_id: String,
    limit: Option<u32>,
) -> AppResult<Vec<ArtifactDto>> {
    Ok(
        list_artifacts_for_project(&content_project_id, limit.unwrap_or(100) as usize)?
            .into_iter()
            .map(map_artifact)
            .collect(),
    )
}

#[tauri::command]
pub fn vnext_project_next_action(content_project_id: String) -> AppResult<String> {
    Ok(project_next_action(&content_project_id)?.as_str().into())
}

/// True if media file on disk looks unchanged since a prior analysis timestamp (RFC3339).
fn media_looks_unchanged(media_path: &str, analyzed_at_rfc3339: &str) -> bool {
    use std::time::SystemTime;
    let path = std::path::Path::new(media_path);
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    let Ok(run_dt) = chrono::DateTime::parse_from_rfc3339(analyzed_at_rfc3339) else {
        return false;
    };
    let run_sys = SystemTime::UNIX_EPOCH
        + std::time::Duration::from_secs(run_dt.timestamp().max(0) as u64);
    // File not newer than analysis (2s skew for FS precision)
    modified
        <= run_sys
            .checked_add(std::time::Duration::from_secs(2))
            .unwrap_or(run_sys)
}

/// Try load last clipping run with candidates for this project if media unchanged.
fn try_reuse_cached_clipping_run(
    content_project_id: &str,
    media_path: &str,
) -> AppResult<Option<crate::models::clipping::ClippingRun>> {
    let run_ids = list_run_ids_for_project(content_project_id)?;
    for id in run_ids {
        let Some(run) = load_run(&id)? else {
            continue;
        };
        if run.candidates.is_empty() {
            continue;
        }
        if !media_looks_unchanged(media_path, &run.created_at) {
            continue;
        }
        // Prefer path match (same file identity by path for this project)
        if !run.media_path.is_empty()
            && !media_path.is_empty()
            && std::path::Path::new(&run.media_path) != std::path::Path::new(media_path)
        {
            // Still ok if same project source — project owns the media path
        }
        return Ok(Some(run));
    }
    // Fallback: candidates exist without loadable run meta — reconstruct not needed;
    // empty means full analysis.
    Ok(None)
}

/// Run legacy clipping analysis and persist into vNext (bridge for UI).
///
/// `force` = true always re-analyzes. Default false: reuses last run for this
/// project when the media file was not modified (keeps transcript + moment statuses).
#[tauri::command]
pub async fn vnext_run_clipping_for_project(
    app: tauri::AppHandle,
    content_project_id: String,
    force: Option<bool>,
) -> AppResult<crate::models::clipping::ClippingRun> {
    let p = get_project(&content_project_id)?
        .ok_or_else(|| AppError::NotFound(content_project_id.clone()))?;
    let force = force.unwrap_or(false);

    if !force {
        if let Some(cached) = try_reuse_cached_clipping_run(&content_project_id, &p.source_media_path)?
        {
            crate::models::progress::emit(
                &app,
                "clipping",
                "cache",
                "Reutilizando análisis y momentos guardados…",
                90.0,
            );
            crate::models::progress::emit(
                &app,
                "clipping",
                "done",
                "Análisis en caché listo",
                100.0,
            );
            return Ok(cached);
        }
    }

    let mut on_prog = |stage: &str, message: &str, percent: f64| {
        crate::models::progress::emit(&app, "clipping", stage, message, percent);
    };
    let run = crate::pipeline::clipping::run_clipping_analysis_with_progress(
        std::path::Path::new(&p.source_media_path),
        crate::models::clipping::ClippingOptions::default(),
        None,
        &mut on_prog,
    )
    .await?;
    persist_clipping_run(&run, &content_project_id)?;
    crate::models::progress::emit(&app, "clipping", "done", "Clips listos", 100.0);
    Ok(run)
}
