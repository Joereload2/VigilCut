use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::models::clipping::{ClipCandidate, ClipFraming, ClipReviewStatus, ClipScoreBreakdown};

use super::db::open_vnext_db;

/// Persist / load short candidates (maps to ClipCandidate for legacy UI).
#[derive(Debug, Clone)]
pub struct ShortCandidateRow {
    pub id: String,
    pub content_project_id: String,
    pub clipping_run_id: Option<String>,
    pub source_job_id: String,
    pub candidate: ClipCandidate,
}

pub fn workflow_status_str(s: ClipReviewStatus) -> &'static str {
    match s {
        ClipReviewStatus::Suggested => "suggested",
        ClipReviewStatus::Preselected => "preselected",
        ClipReviewStatus::Approved => "approved",
        ClipReviewStatus::Rejected => "rejected",
        ClipReviewStatus::Modified => "modified",
        ClipReviewStatus::Exporting => "exporting",
        ClipReviewStatus::Exported => "exported",
        ClipReviewStatus::Error => "error",
        ClipReviewStatus::Discarded => "discarded",
    }
}

pub fn parse_workflow_status(s: &str) -> AppResult<ClipReviewStatus> {
    Ok(match s {
        "suggested" => ClipReviewStatus::Suggested,
        "preselected" => ClipReviewStatus::Preselected,
        "approved" => ClipReviewStatus::Approved,
        "rejected" => ClipReviewStatus::Rejected,
        "modified" => ClipReviewStatus::Modified,
        "exporting" => ClipReviewStatus::Exporting,
        "exported" => ClipReviewStatus::Exported,
        "error" => ClipReviewStatus::Error,
        "discarded" => ClipReviewStatus::Discarded,
        other => {
            return Err(AppError::Invalid(format!(
                "Unknown workflow status: {other}"
            )));
        }
    })
}

pub fn upsert_candidate_from_clip(
    content_project_id: &str,
    clipping_run_id: &str,
    source_job_id: &str,
    c: &ClipCandidate,
    now_rfc3339: &str,
) -> AppResult<()> {
    let conn = open_vnext_db()?;
    let breakdown =
        serde_json::to_string(&c.breakdown).map_err(|e| AppError::Message(e.to_string()))?;
    let reasons =
        serde_json::to_string(&c.reasons).map_err(|e| AppError::Message(e.to_string()))?;
    let warnings =
        serde_json::to_string(&c.warnings).map_err(|e| AppError::Message(e.to_string()))?;
    let strengths =
        serde_json::to_string(&c.strengths).map_err(|e| AppError::Message(e.to_string()))?;
    let risks = serde_json::to_string(&c.risks).map_err(|e| AppError::Message(e.to_string()))?;
    let framing =
        serde_json::to_string(&c.framing).map_err(|e| AppError::Message(e.to_string()))?;
    let status = workflow_status_str(c.status);

    conn.execute(
        r#"INSERT INTO short_candidates(
            id, content_project_id, source_job_id, source_media_path,
            start_s, end_s, duration_s, original_start_s, original_end_s,
            transcript_text, title, summary, score, confidence,
            score_breakdown_json, reasons_json, warnings_json, strengths_json, risks_json,
            workflow_status, variant_group_id, is_primary_variant, framing_json,
            created_at, updated_at, clipping_run_id
        ) VALUES (
            ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?24,?25
        )
        ON CONFLICT(id) DO UPDATE SET
            start_s = excluded.start_s,
            end_s = excluded.end_s,
            duration_s = excluded.duration_s,
            transcript_text = excluded.transcript_text,
            title = excluded.title,
            summary = excluded.summary,
            score = excluded.score,
            confidence = excluded.confidence,
            score_breakdown_json = excluded.score_breakdown_json,
            reasons_json = excluded.reasons_json,
            warnings_json = excluded.warnings_json,
            strengths_json = excluded.strengths_json,
            risks_json = excluded.risks_json,
            workflow_status = excluded.workflow_status,
            variant_group_id = excluded.variant_group_id,
            is_primary_variant = excluded.is_primary_variant,
            framing_json = excluded.framing_json,
            updated_at = excluded.updated_at,
            clipping_run_id = excluded.clipping_run_id
        "#,
        params![
            c.id,
            content_project_id,
            source_job_id,
            c.source_media_path,
            c.start,
            c.end,
            c.duration,
            c.original_start,
            c.original_end,
            c.transcript,
            c.title,
            c.summary,
            c.score,
            c.confidence,
            breakdown,
            reasons,
            warnings,
            strengths,
            risks,
            status,
            c.variant_group_id,
            c.is_primary_variant as i64,
            framing,
            now_rfc3339,
            clipping_run_id,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn get_candidate(id: &str) -> AppResult<Option<ClipCandidate>> {
    let conn = open_vnext_db()?;
    conn.query_row(
        r#"SELECT id, source_media_path, start_s, end_s, duration_s, original_start_s, original_end_s,
                  transcript_text, title, summary, score, confidence,
                  score_breakdown_json, reasons_json, warnings_json, strengths_json, risks_json,
                  workflow_status, variant_group_id, is_primary_variant, framing_json,
                  source_job_id
           FROM short_candidates WHERE id = ?1"#,
        params![id],
        map_clip_row,
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

pub fn list_candidates_for_run(clipping_run_id: &str) -> AppResult<Vec<ClipCandidate>> {
    let conn = open_vnext_db()?;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, source_media_path, start_s, end_s, duration_s, original_start_s, original_end_s,
                      transcript_text, title, summary, score, confidence,
                      score_breakdown_json, reasons_json, warnings_json, strengths_json, risks_json,
                      workflow_status, variant_group_id, is_primary_variant, framing_json,
                      source_job_id
               FROM short_candidates WHERE clipping_run_id = ?1
               ORDER BY score DESC"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![clipping_run_id], map_clip_row)
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}

pub fn count_candidates_for_project(content_project_id: &str) -> AppResult<(usize, usize)> {
    let conn = open_vnext_db()?;
    let total: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM short_candidates WHERE content_project_id = ?1",
            params![content_project_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let approved: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM short_candidates WHERE content_project_id = ?1 AND workflow_status = 'approved'",
            params![content_project_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok((total as usize, approved as usize))
}

pub fn update_candidate_workflow(
    id: &str,
    status: ClipReviewStatus,
    start: f64,
    end: f64,
    framing: &ClipFraming,
    now_rfc3339: &str,
) -> AppResult<()> {
    let conn = open_vnext_db()?;
    let framing_json =
        serde_json::to_string(framing).map_err(|e| AppError::Message(e.to_string()))?;
    let duration = (end - start).max(0.0);
    let n = conn
        .execute(
            r#"UPDATE short_candidates SET
                 workflow_status = ?1,
                 start_s = ?2,
                 end_s = ?3,
                 duration_s = ?4,
                 framing_json = ?5,
                 updated_at = ?6
               WHERE id = ?7"#,
            params![
                workflow_status_str(status),
                start,
                end,
                duration,
                framing_json,
                now_rfc3339,
                id
            ],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    if n == 0 {
        return Err(AppError::NotFound(id.into()));
    }
    Ok(())
}

fn map_clip_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ClipCandidate> {
    let breakdown: ClipScoreBreakdown =
        serde_json::from_str(&r.get::<_, String>(12)?).unwrap_or_default();
    let reasons = serde_json::from_str(&r.get::<_, String>(13)?).unwrap_or_default();
    let warnings = serde_json::from_str(&r.get::<_, String>(14)?).unwrap_or_default();
    let strengths = serde_json::from_str(&r.get::<_, String>(15)?).unwrap_or_default();
    let risks = serde_json::from_str(&r.get::<_, String>(16)?).unwrap_or_default();
    let status_s: String = r.get(17)?;
    let status = parse_workflow_status(&status_s).unwrap_or(ClipReviewStatus::Suggested);
    let framing: ClipFraming = serde_json::from_str(&r.get::<_, String>(20)?).unwrap_or_default();
    let analysis_run_id: String = r.get(21)?;
    Ok(ClipCandidate {
        id: r.get(0)?,
        analysis_run_id,
        source_media_path: r.get(1)?,
        start: r.get(2)?,
        end: r.get(3)?,
        duration: r.get(4)?,
        original_start: r.get(5)?,
        original_end: r.get(6)?,
        transcript: r.get(7)?,
        title: r.get(8)?,
        summary: r.get(9)?,
        score: r.get(10)?,
        confidence: r.get(11)?,
        breakdown,
        reasons,
        warnings,
        strengths,
        risks,
        status,
        variant_group_id: r.get(18)?,
        is_primary_variant: r.get::<_, i64>(19)? != 0,
        framing,
        export_path: None,
        error: None,
    })
}
