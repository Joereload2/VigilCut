use rusqlite::{params, OptionalExtension};

use crate::error::{AppError, AppResult};
use crate::vnext::domain::ContentProjectRecord;

use super::db::open_vnext_db;

pub fn insert_project(p: &ContentProjectRecord) -> AppResult<()> {
    let conn = open_vnext_db()?;
    conn.execute(
        r#"INSERT INTO content_projects(
            id, title, client_label, privacy_mode, status,
            source_media_path, source_sha256, source_duration_s,
            source_width, source_height, source_has_audio, source_has_video, source_container,
            work_dir, active_recipe_id, legacy_project_id, created_at, updated_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
        params![
            p.id,
            p.title,
            p.client_label,
            p.privacy_mode,
            p.status,
            p.source_media_path,
            p.source_sha256,
            p.source_duration_s,
            p.source_width,
            p.source_height,
            p.source_has_audio.map(|b| b as i64),
            p.source_has_video.map(|b| b as i64),
            p.source_container,
            p.work_dir,
            p.active_recipe_id,
            p.legacy_project_id,
            p.created_at,
            p.updated_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn get_project(id: &str) -> AppResult<Option<ContentProjectRecord>> {
    let conn = open_vnext_db()?;
    conn.query_row(
        r#"SELECT id, title, client_label, privacy_mode, status,
                  source_media_path, source_sha256, source_duration_s,
                  source_width, source_height, source_has_audio, source_has_video, source_container,
                  work_dir, active_recipe_id, legacy_project_id, created_at, updated_at
           FROM content_projects WHERE id = ?1"#,
        params![id],
        |r| {
            Ok(ContentProjectRecord {
                id: r.get(0)?,
                title: r.get(1)?,
                client_label: r.get(2)?,
                privacy_mode: r.get(3)?,
                status: r.get(4)?,
                source_media_path: r.get(5)?,
                source_sha256: r.get(6)?,
                source_duration_s: r.get(7)?,
                source_width: r.get(8)?,
                source_height: r.get(9)?,
                source_has_audio: r.get::<_, Option<i64>>(10)?.map(|v| v != 0),
                source_has_video: r.get::<_, Option<i64>>(11)?.map(|v| v != 0),
                source_container: r.get(12)?,
                work_dir: r.get(13)?,
                active_recipe_id: r.get(14)?,
                legacy_project_id: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        },
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

/// Find active project by exact source media path (legacy clipping bridge).
pub fn find_project_by_media_path(media_path: &str) -> AppResult<Option<ContentProjectRecord>> {
    let conn = open_vnext_db()?;
    conn.query_row(
        r#"SELECT id, title, client_label, privacy_mode, status,
                  source_media_path, source_sha256, source_duration_s,
                  source_width, source_height, source_has_audio, source_has_video, source_container,
                  work_dir, active_recipe_id, legacy_project_id, created_at, updated_at
           FROM content_projects
           WHERE source_media_path = ?1 AND status = 'active'
           ORDER BY updated_at DESC LIMIT 1"#,
        params![media_path],
        |r| {
            Ok(ContentProjectRecord {
                id: r.get(0)?,
                title: r.get(1)?,
                client_label: r.get(2)?,
                privacy_mode: r.get(3)?,
                status: r.get(4)?,
                source_media_path: r.get(5)?,
                source_sha256: r.get(6)?,
                source_duration_s: r.get(7)?,
                source_width: r.get(8)?,
                source_height: r.get(9)?,
                source_has_audio: r.get::<_, Option<i64>>(10)?.map(|v| v != 0),
                source_has_video: r.get::<_, Option<i64>>(11)?.map(|v| v != 0),
                source_container: r.get(12)?,
                work_dir: r.get(13)?,
                active_recipe_id: r.get(14)?,
                legacy_project_id: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        },
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

pub fn list_projects(limit: usize) -> AppResult<Vec<ContentProjectRecord>> {
    let conn = open_vnext_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, title, client_label, privacy_mode, status,
                      source_media_path, source_sha256, source_duration_s,
                      source_width, source_height, source_has_audio, source_has_video, source_container,
                      work_dir, active_recipe_id, legacy_project_id, created_at, updated_at
               FROM content_projects ORDER BY updated_at DESC LIMIT ?1"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![limit], |r| {
            Ok(ContentProjectRecord {
                id: r.get(0)?,
                title: r.get(1)?,
                client_label: r.get(2)?,
                privacy_mode: r.get(3)?,
                status: r.get(4)?,
                source_media_path: r.get(5)?,
                source_sha256: r.get(6)?,
                source_duration_s: r.get(7)?,
                source_width: r.get(8)?,
                source_height: r.get(9)?,
                source_has_audio: r.get::<_, Option<i64>>(10)?.map(|v| v != 0),
                source_has_video: r.get::<_, Option<i64>>(11)?.map(|v| v != 0),
                source_container: r.get(12)?,
                work_dir: r.get(13)?,
                active_recipe_id: r.get(14)?,
                legacy_project_id: r.get(15)?,
                created_at: r.get(16)?,
                updated_at: r.get(17)?,
            })
        })
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}
