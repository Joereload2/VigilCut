//! Supervision snapshot for UI: needs + jobs + candidates (compact panel).

use rusqlite::params;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::visual_intel::{
    CandidateStatus, CoverageSummary, JobStatus, NeedCoverage, VisualNeed,
};
use crate::pipeline::visual::generation::provider::CostKind;
use crate::pipeline::visual::library::open_db;
use crate::pipeline::visual::needs::{coverage_for_project, get_need, list_needs, update_need};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobView {
    pub id: String,
    pub need_id: Option<String>,
    pub status: String,
    pub stage: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub prompt: String,
    pub negative_prompt: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub last_error: Option<String>,
    pub is_paid: bool,
    pub cost_kind: String,
    pub free_verified: bool,
    pub prompt_strategy: Option<String>,
    pub origin: String,
    pub cancel_requested: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateView {
    pub id: String,
    pub job_id: String,
    pub need_id: Option<String>,
    pub local_path: Option<String>,
    pub status: String,
    pub technical_score: Option<f64>,
    pub semantic_score: Option<f64>,
    pub qa_decision: Option<String>,
    pub qa_reason: Option<String>,
    pub approved_asset_id: Option<String>,
    pub origin: String,
    pub reject_reason: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub mime_type: Option<String>,
    pub cost_kind: Option<String>,
    pub free_verified: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub file_exists: bool,
    /// Human-readable concept/need labels for daily review (Codex MED-001).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub concept_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub need_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_title: Option<String>,
    pub request_id: Option<String>,
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub prompt_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeedSupervision {
    pub need: VisualNeed,
    pub job: Option<JobView>,
    pub candidate: Option<CandidateView>,
    /// UI-facing state (maps domain coverage + job)
    pub ui_state: String,
    pub ui_label: String,
    pub primary_action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisionSnapshot {
    pub project_key: String,
    pub coverage: CoverageSummary,
    pub needs: Vec<NeedSupervision>,
    pub pending_review: Vec<CandidateView>,
    pub daily_feed: serde_json::Value,
}

pub fn map_ui_state(
    need: &VisualNeed,
    job: Option<&JobView>,
    cand: Option<&CandidateView>,
) -> (String, String, String) {
    if let Some(c) = cand {
        match CandidateStatus::parse(&c.status) {
            CandidateStatus::NeedsHumanReview | CandidateStatus::AutomatedReview => {
                return (
                    "needs_human_review".into(),
                    "Necesita tu revisión".into(),
                    "review".into(),
                );
            }
            CandidateStatus::Approved => {
                return ("approved".into(), "Lista para usar".into(), "use".into());
            }
            CandidateStatus::Rejected | CandidateStatus::Discarded => {
                return ("rejected".into(), "Rechazada".into(), "regenerate".into());
            }
            _ => {}
        }
    }
    if let Some(j) = job {
        match JobStatus::parse(&j.status) {
            JobStatus::Queued => {
                return ("queued".into(), "Esperando turno".into(), "cancel".into());
            }
            JobStatus::Running => {
                if j.cancel_requested || j.stage == "cancelling" {
                    return ("cancelling".into(), "Cancelando…".into(), "wait".into());
                }
                let label = match j.stage.as_str() {
                    "preparing" => "Preparando solicitud",
                    "waiting_provider" => "Esperando proveedor",
                    "generating" => "Generando imagen",
                    "downloading" => "Descargando",
                    "file_review" => "Revisando archivo",
                    "evaluating" => "Evaluando imagen",
                    "cancelling" => "Cancelando…",
                    _ => "Generando imagen",
                };
                return ("processing".into(), label.into(), "cancel".into());
            }
            JobStatus::Failed | JobStatus::BlockedPolicy => {
                return ("failed".into(), "No se pudo generar".into(), "retry".into());
            }
            JobStatus::Cancelled => {
                return (
                    "cancelled".into(),
                    "Generación cancelada".into(),
                    "generate".into(),
                );
            }
            JobStatus::Succeeded => {}
        }
    }
    match need.coverage {
        NeedCoverage::Matched | NeedCoverage::Covered => {
            ("approved".into(), "Cubierta".into(), "use".into())
        }
        NeedCoverage::Generating => ("queued".into(), "Esperando turno".into(), "cancel".into()),
        NeedCoverage::NeedsReview => (
            "needs_human_review".into(),
            "Necesita tu revisión".into(),
            "review".into(),
        ),
        NeedCoverage::Failed => ("failed".into(), "No se pudo generar".into(), "retry".into()),
        NeedCoverage::Skipped => ("skipped".into(), "Sin imagen".into(), "generate".into()),
        NeedCoverage::Uncovered => (
            "uncovered".into(),
            "Falta una imagen".into(),
            "generate".into(),
        ),
    }
}

pub fn get_job(id: &str) -> AppResult<JobView> {
    let conn = open_db()?;
    conn.query_row(
        r#"SELECT id, need_id, status, COALESCE(stage,'queued'), provider, model, prompt, negative_prompt,
           attempt, max_attempts, last_error, is_paid, COALESCE(cost_kind,'unknown'),
           COALESCE(free_verified,0), prompt_strategy, COALESCE(origin,'video_need'),
           COALESCE(cancel_requested,0), created_at, updated_at
           FROM generation_jobs WHERE id = ?1"#,
        params![id],
        row_job,
    )
    .map_err(|e| AppError::NotFound(e.to_string()))
}

fn row_job(r: &rusqlite::Row<'_>) -> rusqlite::Result<JobView> {
    Ok(JobView {
        id: r.get(0)?,
        need_id: r.get(1)?,
        status: r.get(2)?,
        stage: r.get(3)?,
        provider: r.get(4)?,
        model: r.get(5)?,
        prompt: r.get(6)?,
        negative_prompt: r.get(7)?,
        attempt: r.get::<_, i64>(8)? as u32,
        max_attempts: r.get::<_, i64>(9)? as u32,
        last_error: r.get(10)?,
        is_paid: r.get::<_, i64>(11)? != 0,
        cost_kind: r.get(12)?,
        free_verified: r.get::<_, i64>(13)? != 0,
        prompt_strategy: r.get(14)?,
        origin: r.get(15)?,
        cancel_requested: r.get::<_, i64>(16)? != 0,
        created_at: r.get(17)?,
        updated_at: r.get(18)?,
    })
}

pub fn get_candidate(id: &str) -> AppResult<CandidateView> {
    let conn = open_db()?;
    let mut c = conn
        .query_row(
            r#"SELECT id, job_id, need_id, local_path, status, technical_score, semantic_score,
               qa_decision, qa_reason, approved_asset_id, COALESCE(origin,'video_need'),
               reject_reason, width, height, mime_type, cost_kind, COALESCE(free_verified,0),
               provider, model, created_at, updated_at
               FROM generated_candidates WHERE id = ?1"#,
            params![id],
            row_candidate,
        )
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    c.file_exists = c
        .local_path
        .as_ref()
        .map(|p| std::path::Path::new(p).is_file())
        .unwrap_or(false);
    enrich_candidate_labels(&mut c);
    Ok(c)
}

fn row_candidate(r: &rusqlite::Row<'_>) -> rusqlite::Result<CandidateView> {
    Ok(CandidateView {
        id: r.get(0)?,
        job_id: r.get(1)?,
        need_id: r.get(2)?,
        local_path: r.get(3)?,
        status: r.get(4)?,
        technical_score: r.get(5)?,
        semantic_score: r.get(6)?,
        qa_decision: r.get(7)?,
        qa_reason: r.get(8)?,
        approved_asset_id: r.get(9)?,
        origin: r.get(10)?,
        reject_reason: r.get(11)?,
        width: r.get::<_, Option<i64>>(12)?.map(|v| v as u32),
        height: r.get::<_, Option<i64>>(13)?.map(|v| v as u32),
        mime_type: r.get(14)?,
        cost_kind: r.get(15)?,
        free_verified: r.get::<_, i64>(16)? != 0,
        provider: r.get(17)?,
        model: r.get(18)?,
        created_at: r.get(19)?,
        updated_at: r.get(20)?,
        file_exists: false,
        concept_title: None,
        need_label: None,
        theme_title: None,
        request_id: None,
        prompt: None,
        negative_prompt: None,
        prompt_strategy: None,
    })
}

fn enrich_candidate_labels(c: &mut CandidateView) {
    if let Ok(conn) = open_db() {
        if let Ok((prompt, negative, strategy)) = conn.query_row(
            "SELECT prompt, negative_prompt, prompt_strategy FROM generation_jobs WHERE id=?1",
            params![c.job_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        ) {
            c.prompt = Some(prompt);
            c.negative_prompt = Some(negative);
            c.prompt_strategy = strategy;
        }
    }
    if let Some(nid) = &c.need_id {
        if let Ok(n) = get_need(nid) {
            c.need_label = Some(n.label.clone());
            c.request_id = n
                .project_key
                .strip_prefix("library_request:")
                .map(str::to_string);
            if let Some(request_id) = &c.request_id {
                if let Ok(conn) = open_db() {
                    c.theme_title = conn
                        .query_row(
                            "SELECT NULLIF(theme, '') FROM library_requests WHERE id=?1",
                            params![request_id],
                            |row| row.get::<_, Option<String>>(0),
                        )
                        .ok()
                        .flatten();
                }
            }
            if let Some(concept_id) = n.concept_id {
                if let Ok(conn) = open_db() {
                    if let Ok((concept, theme)) = conn.query_row(
                        "SELECT c.title, t.title FROM visual_concepts c
                         LEFT JOIN themes t ON t.id=c.theme_id WHERE c.id=?1",
                        params![concept_id],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
                    ) {
                        c.concept_title = Some(concept);
                        c.theme_title = theme;
                    }
                }
            }
            if c.concept_title.is_none() {
                c.concept_title = Some(n.label);
            }
        }
    }
    if c.concept_title.is_none() {
        // Fall back to job prompt prefix for daily rows without need linkage
        if let Ok(conn) = open_db() {
            if let Ok(prompt) = conn.query_row(
                "SELECT prompt FROM generation_jobs WHERE id=?1",
                params![c.job_id],
                |r| r.get::<_, String>(0),
            ) {
                let short: String = prompt.chars().take(48).collect();
                c.concept_title = Some(short);
            }
        }
    }
}

pub fn latest_candidate_for_need(need_id: &str) -> AppResult<Option<CandidateView>> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, job_id, need_id, local_path, status, technical_score, semantic_score,
               qa_decision, qa_reason, approved_asset_id, COALESCE(origin,'video_need'),
               reject_reason, width, height, mime_type, cost_kind, COALESCE(free_verified,0),
               provider, model, created_at, updated_at
               FROM generated_candidates WHERE need_id = ?1
               ORDER BY created_at DESC LIMIT 1"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut rows = stmt
        .query_map(params![need_id], row_candidate)
        .map_err(|e| AppError::Message(e.to_string()))?;
    if let Some(Ok(mut c)) = rows.next() {
        c.file_exists = c
            .local_path
            .as_ref()
            .map(|p| std::path::Path::new(p).is_file())
            .unwrap_or(false);
        enrich_candidate_labels(&mut c);
        return Ok(Some(c));
    }
    Ok(None)
}

pub fn latest_job_for_need(need_id: &str) -> AppResult<Option<JobView>> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, need_id, status, COALESCE(stage,'queued'), provider, model, prompt, negative_prompt,
               attempt, max_attempts, last_error, is_paid, COALESCE(cost_kind,'unknown'),
               COALESCE(free_verified,0), prompt_strategy, COALESCE(origin,'video_need'),
               COALESCE(cancel_requested,0), created_at, updated_at
               FROM generation_jobs WHERE need_id = ?1
               ORDER BY created_at DESC LIMIT 1"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut rows = stmt
        .query_map(params![need_id], row_job)
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(rows.next().and_then(|r| r.ok()))
}

pub fn supervision_snapshot(project_key: &str) -> AppResult<SupervisionSnapshot> {
    let needs = list_needs(project_key)?;
    let coverage = coverage_for_project(project_key)?;
    let mut out = Vec::new();
    for n in needs {
        let job = n
            .generation_job_id
            .as_ref()
            .and_then(|id| get_job(id).ok())
            .or_else(|| latest_job_for_need(&n.id).ok().flatten());
        let candidate = latest_candidate_for_need(&n.id).ok().flatten();
        let (ui_state, ui_label, primary_action) =
            map_ui_state(&n, job.as_ref(), candidate.as_ref());
        out.push(NeedSupervision {
            need: n,
            job,
            candidate,
            ui_state,
            ui_label,
            primary_action,
        });
    }
    let pending = list_pending_candidates(50)?;
    let daily = crate::pipeline::visual::generation::daily_feed::settings_json()?;
    Ok(SupervisionSnapshot {
        project_key: project_key.into(),
        coverage,
        needs: out,
        pending_review: pending,
        daily_feed: daily,
    })
}

/// Snapshot without project needs — daily feed + global pending review (HIGH-008).
pub fn supervision_snapshot_global() -> AppResult<SupervisionSnapshot> {
    let pending = list_pending_candidates(50)?;
    let daily = crate::pipeline::visual::generation::daily_feed::settings_json()?;
    Ok(SupervisionSnapshot {
        project_key: String::new(),
        coverage: CoverageSummary::default(),
        needs: Vec::new(),
        pending_review: pending,
        daily_feed: daily,
    })
}

pub fn list_pending_candidates(limit: usize) -> AppResult<Vec<CandidateView>> {
    let conn = open_db()?;
    let limit = limit.clamp(1, 100) as i64;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, job_id, need_id, local_path, status, technical_score, semantic_score,
               qa_decision, qa_reason, approved_asset_id, COALESCE(origin,'video_need'),
               reject_reason, width, height, mime_type, cost_kind, COALESCE(free_verified,0),
               provider, model, created_at, updated_at
               FROM generated_candidates
               WHERE status IN ('needs_human_review','automated_review','generated')
               ORDER BY created_at DESC LIMIT ?1"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![limit], row_candidate)
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for mut c in rows.flatten() {
        c.file_exists = c
            .local_path
            .as_ref()
            .map(|p| std::path::Path::new(p).is_file())
            .unwrap_or(false);
        enrich_candidate_labels(&mut c);
        out.push(c);
    }
    Ok(out)
}

/// Cancel job: queued → cancelled; running → cancel_requested + in-process abort flag.
pub fn cancel_job(job_id: &str) -> AppResult<JobView> {
    let mut job = get_job(job_id)?;
    crate::pipeline::visual::generation::supervisor::cancel_registry::request_cancel(job_id);
    match JobStatus::parse(&job.status) {
        JobStatus::Queued => {
            let conn = open_db()?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE generation_jobs SET status='cancelled', stage='cancelled', cancel_requested=1, updated_at=?1 WHERE id=?2",
                params![now, job_id],
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
            if let Some(nid) = &job.need_id {
                if let Ok(mut n) = get_need(nid) {
                    n.coverage = NeedCoverage::Uncovered;
                    n.updated_at = now;
                    update_need(&n)?;
                }
            }
        }
        JobStatus::Running => {
            let conn = open_db()?;
            let now = chrono::Utc::now().to_rfc3339();
            conn.execute(
                "UPDATE generation_jobs SET cancel_requested=1, stage='cancelling', updated_at=?1 WHERE id=?2",
                params![now, job_id],
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
        }
        _ => {
            return Err(AppError::Invalid(
                "Solo se pueden cancelar trabajos en cola o en curso.".into(),
            ));
        }
    }
    job = get_job(job_id)?;
    Ok(job)
}

pub fn is_cancel_requested(job_id: &str) -> bool {
    get_job(job_id)
        .map(|j| j.cancel_requested || j.status == "cancelled")
        .unwrap_or(false)
}

pub fn set_job_stage(job_id: &str, stage: &str) -> AppResult<()> {
    let conn = open_db()?;
    conn.execute(
        "UPDATE generation_jobs SET stage=?1, updated_at=?2 WHERE id=?3",
        params![stage, chrono::Utc::now().to_rfc3339(), job_id],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

/// New generation after reject — enqueue first, discard previous only on success (HIGH-007).
pub fn queue_regenerate(need_id: &str) -> AppResult<String> {
    let mut need = get_need(need_id)?;
    let version = {
        let conn = open_db()?;
        let n: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(attempt_version),0) FROM generation_jobs WHERE need_id = ?1",
                params![need_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        n + 1
    };
    let idem = format!("need:{need_id}:v{version}");
    let job_id =
        super::worker::queue_generation_with_key(&mut need, false, &idem, "video_need")?
            .ok_or_else(|| AppError::Invalid("No se pudo encolar (política de coste)".into()))?;

    // Only after successful enqueue: mark previous review candidates discarded
    if let Ok(Some(c)) = latest_candidate_for_need(need_id) {
        if c.job_id != job_id {
            let conn = open_db()?;
            let _ = conn.execute(
                "UPDATE generated_candidates SET status='discarded', updated_at=?1
                 WHERE id=?2 AND status IN ('needs_human_review','automated_review','generated','rejected')",
                params![chrono::Utc::now().to_rfc3339(), c.id],
            );
        }
    }
    // Bump attempt_version on new job
    {
        let conn = open_db()?;
        let _ = conn.execute(
            "UPDATE generation_jobs SET attempt_version=?1 WHERE id=?2",
            params![version, job_id],
        );
    }
    super::supervisor::request_wake();
    Ok(job_id)
}

pub fn cost_label(kind: &str, free_verified: bool) -> String {
    let k = CostKind::parse(kind);
    if matches!(k, CostKind::FreeConfigured) && !free_verified {
        return "Gratis configurado, no verificado".into();
    }
    k.label_es().into()
}
