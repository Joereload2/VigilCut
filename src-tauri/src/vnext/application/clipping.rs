//! Persist clipping runs / candidates and append-only review decisions.

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::models::clipping::{ClipCandidate, ClipFraming, ClipReviewStatus, ClippingRun};
use crate::vnext::domain::{
    derive_next_action, ContentProjectRecord, NextAction, NextActionInput, ReviewDecisionRecord,
};
use crate::vnext::persistence::{
    content_project_id_for_run, count_candidates_for_project, count_decisions_for_target,
    find_project_by_media_path, get_candidate, insert_clipping_run, insert_decision,
    insert_project, list_decisions_for_target, load_clipping_run, update_candidate_workflow,
    upsert_candidate_from_clip,
};

/// Ensure a ContentProject exists for this media path (creates one if needed).
pub fn ensure_project_for_media(media_path: &str) -> AppResult<ContentProjectRecord> {
    if let Some(p) = find_project_by_media_path(media_path)? {
        return Ok(p);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let title = Path::new(media_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Proyecto")
        .to_string();
    let work_dir = crate::vnext::persistence::vnext_root()?
        .join("projects")
        .join(&id);
    std::fs::create_dir_all(&work_dir)?;
    let rec = ContentProjectRecord::new_local(
        id,
        title,
        media_path.into(),
        work_dir.to_string_lossy().into_owned(),
        now,
    );
    insert_project(&rec)?;
    Ok(rec)
}

/// Persist a full clipping run + all candidates. Idempotent upsert per candidate id.
pub fn persist_clipping_run(run: &ClippingRun, content_project_id: &str) -> AppResult<()> {
    let now = chrono::Utc::now().to_rfc3339();
    // Synthetic job id for lineage when no durable job row yet (Phase 3 bridge).
    let source_job_id = format!("clipping_run:{}", run.id);
    insert_clipping_run(
        &run.id,
        content_project_id,
        &run.media_path,
        run.source_duration,
        &run.options,
        &run.summary,
        if run.created_at.is_empty() {
            &now
        } else {
            &run.created_at
        },
    )?;
    for c in &run.candidates {
        upsert_candidate_from_clip(content_project_id, &run.id, &source_job_id, c, &now)?;
    }
    Ok(())
}

pub fn load_run(run_id: &str) -> AppResult<Option<ClippingRun>> {
    load_clipping_run(run_id)
}

/// Apply status change: append decision + update denormalized candidate + return candidate.
pub fn apply_status_decision(
    run_id: &str,
    candidate_id: &str,
    status: ClipReviewStatus,
    reason: Option<String>,
) -> AppResult<ClipCandidate> {
    let project_id = content_project_id_for_run(run_id)?
        .ok_or_else(|| AppError::NotFound(format!("clipping run {run_id}")))?;
    let mut c =
        get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))?;
    c.status = status;
    let now = chrono::Utc::now().to_rfc3339();
    let decision = match status {
        ClipReviewStatus::Approved => "approve",
        ClipReviewStatus::Rejected => "reject",
        ClipReviewStatus::Discarded => "reject",
        ClipReviewStatus::Modified => "modify_span",
        _ => "defer",
    };
    let payload = serde_json::json!({
        "status": crate::vnext::persistence::workflow_status_str(status),
        "startS": c.start,
        "endS": c.end,
        "framing": c.framing,
    });
    insert_decision(&ReviewDecisionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        content_project_id: project_id,
        target_kind: ReviewDecisionRecord::TARGET_SHORT_CANDIDATE.into(),
        target_id: candidate_id.into(),
        decision: decision.into(),
        reason,
        payload_json: payload.to_string(),
        actor: ReviewDecisionRecord::ACTOR_LOCAL.into(),
        related_job_id: None,
        created_at: now.clone(),
    })?;
    update_candidate_workflow(candidate_id, status, c.start, c.end, &c.framing, &now)?;
    get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))
}

pub fn apply_span_decision(
    run_id: &str,
    candidate_id: &str,
    start: f64,
    end: f64,
) -> AppResult<ClipCandidate> {
    let project_id = content_project_id_for_run(run_id)?
        .ok_or_else(|| AppError::NotFound(format!("clipping run {run_id}")))?;
    let mut c =
        get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))?;
    c.set_span(start, end);
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "startS": c.start,
        "endS": c.end,
        "originalStartS": c.original_start,
        "originalEndS": c.original_end,
        "status": crate::vnext::persistence::workflow_status_str(c.status),
    });
    insert_decision(&ReviewDecisionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        content_project_id: project_id,
        target_kind: ReviewDecisionRecord::TARGET_SHORT_CANDIDATE.into(),
        target_id: candidate_id.into(),
        decision: "modify_span".into(),
        reason: None,
        payload_json: payload.to_string(),
        actor: ReviewDecisionRecord::ACTOR_LOCAL.into(),
        related_job_id: None,
        created_at: now.clone(),
    })?;
    update_candidate_workflow(candidate_id, c.status, c.start, c.end, &c.framing, &now)?;
    get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))
}

pub fn apply_framing_decision(
    run_id: &str,
    candidate_id: &str,
    framing: ClipFraming,
) -> AppResult<ClipCandidate> {
    let project_id = content_project_id_for_run(run_id)?
        .ok_or_else(|| AppError::NotFound(format!("clipping run {run_id}")))?;
    let mut c =
        get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))?;
    c.framing = framing;
    if !matches!(c.status, ClipReviewStatus::Exported) {
        c.status = ClipReviewStatus::Modified;
    }
    let now = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "framing": c.framing,
        "status": crate::vnext::persistence::workflow_status_str(c.status),
    });
    insert_decision(&ReviewDecisionRecord {
        id: uuid::Uuid::new_v4().to_string(),
        content_project_id: project_id,
        target_kind: ReviewDecisionRecord::TARGET_SHORT_CANDIDATE.into(),
        target_id: candidate_id.into(),
        decision: "modify_framing".into(),
        reason: None,
        payload_json: payload.to_string(),
        actor: ReviewDecisionRecord::ACTOR_LOCAL.into(),
        related_job_id: None,
        created_at: now.clone(),
    })?;
    update_candidate_workflow(candidate_id, c.status, c.start, c.end, &c.framing, &now)?;
    get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))
}

pub fn list_review_history(candidate_id: &str) -> AppResult<Vec<ReviewDecisionRecord>> {
    list_decisions_for_target(ReviewDecisionRecord::TARGET_SHORT_CANDIDATE, candidate_id)
}

pub fn decision_count(candidate_id: &str) -> AppResult<usize> {
    count_decisions_for_target(ReviewDecisionRecord::TARGET_SHORT_CANDIDATE, candidate_id)
}

/// Derive next action for a content project from durable state.
pub fn project_next_action(content_project_id: &str) -> AppResult<NextAction> {
    use crate::vnext::persistence::get_project;
    let project = get_project(content_project_id)?
        .ok_or_else(|| AppError::NotFound(content_project_id.into()))?;
    let (candidate_count, approved_count) = count_candidates_for_project(content_project_id)?;

    // Lightweight job scan via listing is not yet implemented; probe via known statuses.
    // Phase 3: check if any job for project is failed/interrupted by querying SQLite.
    let conn = crate::vnext::persistence::open_vnext_db()?;
    let failed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE content_project_id = ?1 AND status IN ('failed','interrupted')",
            rusqlite::params![content_project_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let completed_render: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE content_project_id = ?1 AND kind = 'vertical_render' AND status = 'completed'",
            rusqlite::params![content_project_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let has_transcript: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM artifacts WHERE content_project_id = ?1 AND kind IN ('transcript_json','transcript_srt') AND validation_status = 'passed'",
            rusqlite::params![content_project_id],
            |r| r.get(0),
        )
        .unwrap_or(0);

    // Clipping implies transcript was available or fallback used — treat candidates as post-transcript.
    let has_transcript = has_transcript > 0 || candidate_count > 0;

    Ok(derive_next_action(&NextActionInput {
        has_source_media: !project.source_media_path.is_empty(),
        has_probe: project.source_duration_s.is_some() || candidate_count > 0,
        has_transcript,
        candidate_count,
        approved_count,
        has_failed_or_interrupted_job: failed > 0,
        has_completed_render: completed_render > 0,
        has_active_job: false,
        latest_job_status: None,
    }))
}

/// Sync in-memory candidate mutations back to DB without a new decision (export path).
pub fn sync_candidate_state(c: &ClipCandidate, run_id: &str) -> AppResult<()> {
    let Some(project_id) = content_project_id_for_run(run_id)? else {
        return Ok(());
    };
    let now = chrono::Utc::now().to_rfc3339();
    let source_job_id = format!("clipping_run:{run_id}");
    upsert_candidate_from_clip(&project_id, run_id, &source_job_id, c, &now)
}
