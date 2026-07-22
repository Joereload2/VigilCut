//! Visual needs detection and persistence.

use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::models::visual::SemanticEvent;
use crate::models::visual_intel::{
    CoverageSummary, NeedCoverage, VisualNeed,
};
use crate::pipeline::visual::library::open_db;

fn json_vec(v: &[String]) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "[]".into())
}
fn parse_vec(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

/// Build needs from semantic events (deterministic). Caps density.
pub fn detect_needs_from_semantics(
    project_key: &str,
    media_path: Option<&str>,
    events: &[SemanticEvent],
    max_needs: usize,
) -> Vec<VisualNeed> {
    let mut needs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut sorted: Vec<&SemanticEvent> = events.iter().collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for ev in sorted {
        if needs.len() >= max_needs {
            break;
        }
        let key = ev.label.to_lowercase();
        if key.chars().count() < 4 {
            continue;
        }
        if !seen.insert(key.clone()) {
            continue;
        }
        let mut n = VisualNeed::from_label(project_key, &ev.label);
        n.media_path = media_path.map(|s| s.into());
        n.semantic_event_id = Some(ev.id.clone());
        n.terms = ev.terms.clone();
        n.priority = ((ev.score * 100.0) as i32).clamp(1, 100);
        if let Some(out) = ev.output_span {
            n.output_start = Some(out.start);
            n.output_end = Some((out.start + 5.0).min(out.end.max(out.start + 3.5)));
            n.approx_duration_secs = n.output_end.unwrap_or(5.0) - n.output_start.unwrap_or(0.0);
        }
        n.source_start = Some(ev.source_span.start);
        n.source_end = Some(ev.source_span.end);
        needs.push(n);
    }
    needs
}

pub fn save_needs(needs: &[VisualNeed]) -> AppResult<()> {
    let conn = open_db()?;
    for n in needs {
        conn.execute(
            r#"INSERT OR REPLACE INTO visual_needs (
                id, project_key, media_path, semantic_event_id, concept_id, label, terms,
                required_contexts, forbidden_contexts, hard_exclusions, desired_aspect,
                approx_duration_secs, source_start, source_end, output_start, output_end,
                priority, coverage, matched_asset_id, match_score, match_reasons,
                generation_job_id, created_at, updated_at
            ) VALUES (
                ?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24
            )"#,
            params![
                n.id,
                n.project_key,
                n.media_path,
                n.semantic_event_id,
                n.concept_id,
                n.label,
                json_vec(&n.terms),
                json_vec(&n.required_contexts),
                json_vec(&n.forbidden_contexts),
                json_vec(&n.hard_exclusions),
                n.desired_aspect,
                n.approx_duration_secs,
                n.source_start,
                n.source_end,
                n.output_start,
                n.output_end,
                n.priority,
                n.coverage.as_str(),
                n.matched_asset_id,
                n.match_score,
                json_vec(&n.match_reasons),
                n.generation_job_id,
                n.created_at,
                n.updated_at,
            ],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }
    Ok(())
}

pub fn list_needs(project_key: &str) -> AppResult<Vec<VisualNeed>> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT id, project_key, media_path, semantic_event_id, concept_id, label, terms,
             required_contexts, forbidden_contexts, hard_exclusions, desired_aspect,
             approx_duration_secs, source_start, source_end, output_start, output_end,
             priority, coverage, matched_asset_id, match_score, match_reasons,
             generation_job_id, created_at, updated_at
             FROM visual_needs WHERE project_key = ?1 ORDER BY priority DESC, output_start ASC",
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![project_key], row_need)
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(rows.flatten().collect())
}

fn row_need(r: &rusqlite::Row<'_>) -> rusqlite::Result<VisualNeed> {
    let cov: String = r.get(17)?;
    Ok(VisualNeed {
        id: r.get(0)?,
        project_key: r.get(1)?,
        media_path: r.get(2)?,
        semantic_event_id: r.get(3)?,
        concept_id: r.get(4)?,
        label: r.get(5)?,
        terms: parse_vec(&r.get::<_, String>(6)?),
        required_contexts: parse_vec(&r.get::<_, String>(7)?),
        forbidden_contexts: parse_vec(&r.get::<_, String>(8)?),
        hard_exclusions: parse_vec(&r.get::<_, String>(9)?),
        desired_aspect: r.get(10)?,
        approx_duration_secs: r.get(11)?,
        source_start: r.get(12)?,
        source_end: r.get(13)?,
        output_start: r.get(14)?,
        output_end: r.get(15)?,
        priority: r.get(16)?,
        coverage: NeedCoverage::parse(&cov),
        matched_asset_id: r.get(18)?,
        match_score: r.get(19)?,
        match_reasons: parse_vec(&r.get::<_, String>(20)?),
        generation_job_id: r.get(21)?,
        created_at: r.get(22)?,
        updated_at: r.get(23)?,
    })
}

pub fn update_need(n: &VisualNeed) -> AppResult<()> {
    save_needs(std::slice::from_ref(n))
}

pub fn get_need(id: &str) -> AppResult<VisualNeed> {
    let conn = open_db()?;
    conn.query_row(
        "SELECT id, project_key, media_path, semantic_event_id, concept_id, label, terms,
         required_contexts, forbidden_contexts, hard_exclusions, desired_aspect,
         approx_duration_secs, source_start, source_end, output_start, output_end,
         priority, coverage, matched_asset_id, match_score, match_reasons,
         generation_job_id, created_at, updated_at
         FROM visual_needs WHERE id = ?1",
        params![id],
        row_need,
    )
    .map_err(|e| AppError::NotFound(e.to_string()))
}

pub fn coverage_for_project(project_key: &str) -> AppResult<CoverageSummary> {
    let needs = list_needs(project_key)?;
    Ok(CoverageSummary::from_needs(&needs))
}

pub fn skip_need(id: &str) -> AppResult<VisualNeed> {
    let mut n = get_need(id)?;
    n.coverage = NeedCoverage::Skipped;
    n.updated_at = chrono::Utc::now().to_rfc3339();
    update_need(&n)?;
    Ok(n)
}
