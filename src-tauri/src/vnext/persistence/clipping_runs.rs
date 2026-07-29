use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::models::clipping::{ClippingOptions, ClippingRun, ClippingSummary};

use super::candidates::list_candidates_for_run;
use super::db::open_vnext_db;

#[derive(Debug, Clone)]
pub struct ClippingRunMeta {
    pub id: String,
    pub content_project_id: String,
    pub media_path: String,
    pub source_duration: f64,
    pub options_json: String,
    pub summary_json: String,
    pub created_at: String,
}

pub fn insert_clipping_run(
    id: &str,
    content_project_id: &str,
    media_path: &str,
    source_duration: f64,
    options: &ClippingOptions,
    summary: &ClippingSummary,
    created_at: &str,
) -> AppResult<()> {
    let conn = open_vnext_db()?;
    let options_json =
        serde_json::to_string(options).map_err(|e| AppError::Message(e.to_string()))?;
    let summary_json =
        serde_json::to_string(summary).map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute(
        r#"INSERT OR REPLACE INTO clipping_runs(
            id, content_project_id, media_path, source_duration,
            options_json, summary_json, created_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
        params![
            id,
            content_project_id,
            media_path,
            source_duration,
            options_json,
            summary_json,
            created_at
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn get_clipping_run_meta(id: &str) -> AppResult<Option<ClippingRunMeta>> {
    let conn = open_vnext_db()?;
    conn.query_row(
        r#"SELECT id, content_project_id, media_path, source_duration,
                  options_json, summary_json, created_at
           FROM clipping_runs WHERE id = ?1"#,
        params![id],
        |r| {
            Ok(ClippingRunMeta {
                id: r.get(0)?,
                content_project_id: r.get(1)?,
                media_path: r.get(2)?,
                source_duration: r.get(3)?,
                options_json: r.get(4)?,
                summary_json: r.get(5)?,
                created_at: r.get(6)?,
            })
        },
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

/// Rebuild full ClippingRun from meta + short_candidates.
pub fn load_clipping_run(id: &str) -> AppResult<Option<ClippingRun>> {
    let Some(meta) = get_clipping_run_meta(id)? else {
        return Ok(None);
    };
    let options: ClippingOptions =
        serde_json::from_str(&meta.options_json).map_err(|e| AppError::Message(e.to_string()))?;
    let summary: ClippingSummary =
        serde_json::from_str(&meta.summary_json).map_err(|e| AppError::Message(e.to_string()))?;
    let candidates = list_candidates_for_run(id)?;
    Ok(Some(ClippingRun {
        id: meta.id,
        media_path: meta.media_path,
        source_duration: meta.source_duration,
        options,
        candidates,
        summary,
        created_at: meta.created_at,
    }))
}

pub fn content_project_id_for_run(run_id: &str) -> AppResult<Option<String>> {
    Ok(get_clipping_run_meta(run_id)?.map(|m| m.content_project_id))
}
