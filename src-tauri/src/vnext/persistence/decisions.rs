//! Append-only review decisions — no update/delete APIs.

use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::vnext::domain::ReviewDecisionRecord;

use super::db::open_vnext_db;

pub fn insert_decision(d: &ReviewDecisionRecord) -> AppResult<()> {
    let conn = open_vnext_db()?;
    conn.execute(
        r#"INSERT INTO review_decisions(
            id, content_project_id, target_kind, target_id, decision, reason,
            payload_json, actor, related_job_id, created_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#,
        params![
            d.id,
            d.content_project_id,
            d.target_kind,
            d.target_id,
            d.decision,
            d.reason,
            d.payload_json,
            d.actor,
            d.related_job_id,
            d.created_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn list_decisions_for_target(
    target_kind: &str,
    target_id: &str,
) -> AppResult<Vec<ReviewDecisionRecord>> {
    let conn = open_vnext_db()?;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, content_project_id, target_kind, target_id, decision, reason,
                      payload_json, actor, related_job_id, created_at
               FROM review_decisions
               WHERE target_kind = ?1 AND target_id = ?2
               ORDER BY created_at ASC"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![target_kind, target_id], |r| {
            Ok(ReviewDecisionRecord {
                id: r.get(0)?,
                content_project_id: r.get(1)?,
                target_kind: r.get(2)?,
                target_id: r.get(3)?,
                decision: r.get(4)?,
                reason: r.get(5)?,
                payload_json: r.get(6)?,
                actor: r.get(7)?,
                related_job_id: r.get(8)?,
                created_at: r.get(9)?,
            })
        })
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}

pub fn count_decisions_for_target(target_kind: &str, target_id: &str) -> AppResult<usize> {
    let conn = open_vnext_db()?;
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM review_decisions WHERE target_kind = ?1 AND target_id = ?2",
            params![target_kind, target_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(n as usize)
}
