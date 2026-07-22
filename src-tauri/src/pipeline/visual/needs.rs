//! Visual needs detection and persistence.

use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::models::visual::SemanticEvent;
use crate::models::visual_intel::{CoverageSummary, NeedCoverage, VisualNeed};
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
        insert_need_row(&conn, n)?;
    }
    Ok(())
}

fn insert_need_row(conn: &rusqlite::Connection, n: &VisualNeed) -> AppResult<()> {
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
    Ok(())
}

/// Merge newly detected needs without destroying in-flight jobs or approved coverage.
/// - Preserves needs that have active jobs, matched assets, or non-uncovered coverage.
/// - Adds new labels not already present (by label key).
/// - Updates timing/priority on still-uncovered needs with same label.
pub fn merge_detected_needs(
    project_key: &str,
    detected: Vec<VisualNeed>,
) -> AppResult<Vec<VisualNeed>> {
    let existing = list_needs(project_key).unwrap_or_default();
    let mut by_label: std::collections::HashMap<String, VisualNeed> = existing
        .into_iter()
        .map(|n| (n.label.to_lowercase(), n))
        .collect();

    for mut d in detected {
        let key = d.label.to_lowercase();
        if let Some(prev) = by_label.get(&key) {
            let protected = prev.generation_job_id.is_some()
                || prev.matched_asset_id.is_some()
                || !matches!(
                    prev.coverage,
                    NeedCoverage::Uncovered | NeedCoverage::Skipped
                );
            if protected {
                // Keep previous; optionally refresh times if still uncovered only — skip
                continue;
            }
            // Merge timing into new detection but keep id
            d.id = prev.id.clone();
            d.created_at = prev.created_at.clone();
            d.coverage = prev.coverage;
            d.matched_asset_id = prev.matched_asset_id.clone();
            d.generation_job_id = prev.generation_job_id.clone();
            d.match_score = prev.match_score;
            d.match_reasons = prev.match_reasons.clone();
            d.updated_at = chrono::Utc::now().to_rfc3339();
            by_label.insert(key, d);
        } else {
            by_label.insert(key, d);
        }
    }

    let conn = open_db()?;
    let out: Vec<VisualNeed> = by_label.into_values().collect();
    for n in &out {
        insert_need_row(&conn, n)?;
    }
    // Return sorted like list_needs
    list_needs(project_key)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::library::set_library_root_override;

    #[test]
    fn merge_preserves_covered_need() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-merge-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));

        let mut existing = VisualNeed::from_label("pk", "inflacion");
        existing.coverage = NeedCoverage::Covered;
        existing.matched_asset_id = Some("asset-1".into());
        existing.generation_job_id = Some("job-1".into());
        save_needs(std::slice::from_ref(&existing)).unwrap();

        let mut fresh = VisualNeed::from_label("pk", "inflacion");
        fresh.priority = 99;
        let mut other = VisualNeed::from_label("pk", "nuevo_concepto");
        other.priority = 10;

        let merged = merge_detected_needs("pk", vec![fresh, other]).unwrap();
        let inf = merged.iter().find(|n| n.label == "inflacion").unwrap();
        assert_eq!(inf.matched_asset_id.as_deref(), Some("asset-1"));
        assert_eq!(inf.coverage, NeedCoverage::Covered);
        assert!(merged.iter().any(|n| n.label == "nuevo_concepto"));

        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
