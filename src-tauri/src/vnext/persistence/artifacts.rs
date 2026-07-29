use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::vnext::domain::ArtifactValidationStatus;

use super::db::open_vnext_db;

#[derive(Debug, Clone)]
pub struct ArtifactRow {
    pub id: String,
    pub content_project_id: String,
    pub kind: String,
    pub role: String,
    pub path: String,
    pub sha256: String,
    pub byte_size: i64,
    pub mime_type: String,
    pub created_by_job_id: String,
    pub probe_json: Option<String>,
    pub validation_status: String,
    pub validation_notes: Option<String>,
    pub created_at: String,
}

pub struct InsertArtifact {
    pub id: String,
    pub content_project_id: String,
    pub kind: String,
    pub role: String,
    pub path: String,
    pub sha256: String,
    pub byte_size: i64,
    pub mime_type: String,
    pub created_by_job_id: String,
    pub probe_json: Option<String>,
    pub validation_status: ArtifactValidationStatus,
    pub validation_notes: Option<String>,
    pub parent_ids: Vec<String>,
    pub created_at: String,
}

pub fn insert_artifact(a: &InsertArtifact) -> AppResult<ArtifactRow> {
    let mut conn = open_vnext_db()?;
    let tx = conn
        .transaction()
        .map_err(|e| AppError::Message(e.to_string()))?;
    tx.execute(
        r#"INSERT INTO artifacts(
            id, content_project_id, kind, role, path, sha256, byte_size, mime_type,
            created_by_job_id, probe_json, validation_status, validation_notes, created_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"#,
        params![
            a.id,
            a.content_project_id,
            a.kind,
            a.role,
            a.path,
            a.sha256,
            a.byte_size,
            a.mime_type,
            a.created_by_job_id,
            a.probe_json,
            a.validation_status.as_str(),
            a.validation_notes,
            a.created_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    for parent in &a.parent_ids {
        tx.execute(
            "INSERT INTO artifact_parents(artifact_id, parent_artifact_id) VALUES (?1, ?2)",
            params![a.id, parent],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }
    tx.commit().map_err(|e| AppError::Message(e.to_string()))?;
    get_artifact(&a.id)?.ok_or_else(|| AppError::Message("artifact insert vanished".into()))
}

pub fn get_artifact(id: &str) -> AppResult<Option<ArtifactRow>> {
    let conn = open_vnext_db()?;
    use rusqlite::OptionalExtension;
    conn.query_row(
        r#"SELECT id, content_project_id, kind, role, path, sha256, byte_size, mime_type,
                  created_by_job_id, probe_json, validation_status, validation_notes, created_at
           FROM artifacts WHERE id = ?1"#,
        params![id],
        |r| {
            Ok(ArtifactRow {
                id: r.get(0)?,
                content_project_id: r.get(1)?,
                kind: r.get(2)?,
                role: r.get(3)?,
                path: r.get(4)?,
                sha256: r.get(5)?,
                byte_size: r.get(6)?,
                mime_type: r.get(7)?,
                created_by_job_id: r.get(8)?,
                probe_json: r.get(9)?,
                validation_status: r.get(10)?,
                validation_notes: r.get(11)?,
                created_at: r.get(12)?,
            })
        },
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

pub fn list_artifacts_for_project(
    content_project_id: &str,
    limit: usize,
) -> AppResult<Vec<ArtifactRow>> {
    let conn = open_vnext_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, content_project_id, kind, role, path, sha256, byte_size, mime_type,
                      created_by_job_id, probe_json, validation_status, validation_notes, created_at
               FROM artifacts WHERE content_project_id = ?1
               ORDER BY created_at DESC LIMIT ?2"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![content_project_id, limit], |r| {
            Ok(ArtifactRow {
                id: r.get(0)?,
                content_project_id: r.get(1)?,
                kind: r.get(2)?,
                role: r.get(3)?,
                path: r.get(4)?,
                sha256: r.get(5)?,
                byte_size: r.get(6)?,
                mime_type: r.get(7)?,
                created_by_job_id: r.get(8)?,
                probe_json: r.get(9)?,
                validation_status: r.get(10)?,
                validation_notes: r.get(11)?,
                created_at: r.get(12)?,
            })
        })
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}

pub fn list_parent_ids(artifact_id: &str) -> AppResult<Vec<String>> {
    let conn = open_vnext_db()?;
    let mut stmt = conn
        .prepare("SELECT parent_artifact_id FROM artifact_parents WHERE artifact_id = ?1")
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![artifact_id], |r| r.get(0))
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}
