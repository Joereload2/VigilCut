//! Read model for concept-level library coverage.

use rusqlite::params;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::pipeline::visual::library::open_db;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConceptCoverageRow {
    pub concept_id: String,
    pub title: String,
    pub priority: i32,
    pub useful_assets: u32,
    pub pending_candidates: u32,
    pub active_requests: u32,
    pub requested_target: u32,
    pub state: String,
}

pub fn list(limit: usize) -> AppResult<Vec<ConceptCoverageRow>> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.title, c.priority,
                COUNT(DISTINCT CASE
                    WHEN a.status='active' AND COALESCE(a.qa_status,'none') != 'rejected'
                    THEN a.id END) AS useful_assets,
                COUNT(DISTINCT CASE
                    WHEN gc.status IN ('generated','automated_review','needs_human_review')
                    THEN gc.id END) AS pending_candidates,
                COUNT(DISTINCT CASE
                    WHEN lr.status IN ('draft','active','cancelling')
                    THEN lr.id END) AS active_requests,
                COALESCE(MAX(lr.target_count),0) AS requested_target
             FROM visual_concepts c
             LEFT JOIN asset_concepts ac ON ac.concept_id=c.id
             LEFT JOIN media_assets a ON a.id=ac.asset_id
             LEFT JOIN generation_jobs gj ON gj.concept_id=c.id
             LEFT JOIN generated_candidates gc ON gc.job_id=gj.id
             LEFT JOIN library_requests lr ON lr.concept_id=c.id
             WHERE c.status IN ('active','priority')
             GROUP BY c.id, c.title, c.priority
             ORDER BY useful_assets ASC, c.priority DESC, c.title ASC
             LIMIT ?1",
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![limit.clamp(1, 500) as i64], |r| {
            let useful_assets = r.get::<_, i64>(3)?.max(0) as u32;
            let pending_candidates = r.get::<_, i64>(4)?.max(0) as u32;
            let requested_target = r.get::<_, i64>(6)?.max(0) as u32;
            let state = if useful_assets >= requested_target.max(1) {
                "covered"
            } else if pending_candidates > 0 {
                "in_review"
            } else if useful_assets > 0 {
                "partial"
            } else {
                "uncovered"
            };
            Ok(ConceptCoverageRow {
                concept_id: r.get(0)?,
                title: r.get(1)?,
                priority: r.get(2)?,
                useful_assets,
                pending_candidates,
                active_requests: r.get::<_, i64>(5)?.max(0) as u32,
                requested_target,
                state: state.into(),
            })
        })
        .map_err(|e| AppError::Message(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual_intel::{ConceptStatus, VisualConcept};
    use crate::pipeline::visual::concepts::insert_concept;
    use crate::pipeline::visual::library::{lock_library_for_test, set_library_root_override};

    #[test]
    fn lists_concept_titles_and_real_coverage() {
        let _lock = lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-coverage-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let mut concept = VisualConcept::new("Comercio de barrio", None);
        concept.status = ConceptStatus::Active;
        insert_concept(concept).unwrap();
        let rows = list(20).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].title, "Comercio de barrio");
        assert_eq!(rows[0].state, "uncovered");
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
