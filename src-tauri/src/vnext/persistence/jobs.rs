use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::error::{AppError, AppResult};
use crate::vnext::domain::{JobKind, JobStatus};

use super::db::open_vnext_db;

#[derive(Debug, Clone)]
pub struct JobRow {
    pub id: String,
    pub content_project_id: String,
    pub kind: String,
    pub status: String,
    pub idempotency_key: String,
    pub attempt: i64,
    pub max_attempts: i64,
    pub priority: i64,
    pub input_json: String,
    pub result_artifact_id: Option<String>,
    pub render_plan_id: Option<String>,
    pub error_json: Option<String>,
    pub stage: String,
    pub progress_pct: f64,
    pub locked_by: Option<String>,
    pub lease_expires_at: Option<String>,
    pub cancel_requested: bool,
    pub created_at: String,
    pub updated_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

pub struct EnqueueJob {
    pub id: String,
    pub content_project_id: String,
    pub kind: JobKind,
    pub idempotency_key: String,
    pub max_attempts: u32,
    pub priority: i32,
    pub input_json: String,
    pub now_rfc3339: String,
}

/// Insert job as queued. If idempotency key exists, return existing row (no duplicate).
pub fn enqueue_job(req: &EnqueueJob) -> AppResult<JobRow> {
    let conn = open_vnext_db()?;
    if let Some(existing) = get_job_by_idempotency_conn(&conn, &req.idempotency_key)? {
        return Ok(existing);
    }
    conn.execute(
        r#"INSERT INTO jobs(
            id, content_project_id, kind, status, idempotency_key,
            attempt, max_attempts, priority, input_json,
            stage, progress_pct, cancel_requested, created_at, updated_at
        ) VALUES (?1,?2,?3,'queued',?4,0,?5,?6,?7,'',0,0,?8,?8)"#,
        params![
            req.id,
            req.content_project_id,
            req.kind.as_str(),
            req.idempotency_key,
            req.max_attempts as i64,
            req.priority as i64,
            req.input_json,
            req.now_rfc3339,
        ],
    )
    .map_err(|e| {
        // Unique race: fetch existing
        if e.to_string().contains("UNIQUE") {
            return AppError::Message(e.to_string());
        }
        AppError::Message(e.to_string())
    })?;
    get_job(&req.id)?.ok_or_else(|| AppError::Message("job insert vanished".into()))
}

pub fn get_job(id: &str) -> AppResult<Option<JobRow>> {
    let conn = open_vnext_db()?;
    get_job_conn(&conn, id)
}

pub fn list_jobs_for_project(content_project_id: &str, limit: usize) -> AppResult<Vec<JobRow>> {
    let conn = open_vnext_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, content_project_id, kind, status, idempotency_key,
                      attempt, max_attempts, priority, input_json,
                      result_artifact_id, render_plan_id, error_json, stage, progress_pct,
                      locked_by, lease_expires_at, cancel_requested,
                      created_at, updated_at, started_at, finished_at
               FROM jobs WHERE content_project_id = ?1
               ORDER BY created_at DESC LIMIT ?2"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![content_project_id, limit], map_job_row)
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}

pub fn list_recent_jobs(limit: usize) -> AppResult<Vec<JobRow>> {
    let conn = open_vnext_db()?;
    let limit = limit.clamp(1, 500) as i64;
    let mut stmt = conn
        .prepare(
            r#"SELECT id, content_project_id, kind, status, idempotency_key,
                      attempt, max_attempts, priority, input_json,
                      result_artifact_id, render_plan_id, error_json, stage, progress_pct,
                      locked_by, lease_expires_at, cancel_requested,
                      created_at, updated_at, started_at, finished_at
               FROM jobs ORDER BY updated_at DESC LIMIT ?1"#,
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![limit], map_job_row)
        .map_err(|e| AppError::Message(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| AppError::Message(e.to_string()))?);
    }
    Ok(out)
}

fn get_job_conn(conn: &rusqlite::Connection, id: &str) -> AppResult<Option<JobRow>> {
    conn.query_row(
        r#"SELECT id, content_project_id, kind, status, idempotency_key,
                  attempt, max_attempts, priority, input_json,
                  result_artifact_id, render_plan_id, error_json, stage, progress_pct,
                  locked_by, lease_expires_at, cancel_requested,
                  created_at, updated_at, started_at, finished_at
           FROM jobs WHERE id = ?1"#,
        params![id],
        map_job_row,
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

fn get_job_by_idempotency_conn(
    conn: &rusqlite::Connection,
    key: &str,
) -> AppResult<Option<JobRow>> {
    conn.query_row(
        r#"SELECT id, content_project_id, kind, status, idempotency_key,
                  attempt, max_attempts, priority, input_json,
                  result_artifact_id, render_plan_id, error_json, stage, progress_pct,
                  locked_by, lease_expires_at, cancel_requested,
                  created_at, updated_at, started_at, finished_at
           FROM jobs WHERE idempotency_key = ?1"#,
        params![key],
        map_job_row,
    )
    .optional()
    .map_err(|e| AppError::Message(e.to_string()))
}

fn map_job_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<JobRow> {
    Ok(JobRow {
        id: r.get(0)?,
        content_project_id: r.get(1)?,
        kind: r.get(2)?,
        status: r.get(3)?,
        idempotency_key: r.get(4)?,
        attempt: r.get(5)?,
        max_attempts: r.get(6)?,
        priority: r.get(7)?,
        input_json: r.get(8)?,
        result_artifact_id: r.get(9)?,
        render_plan_id: r.get(10)?,
        error_json: r.get(11)?,
        stage: r.get(12)?,
        progress_pct: r.get(13)?,
        locked_by: r.get(14)?,
        lease_expires_at: r.get(15)?,
        cancel_requested: r.get::<_, i64>(16)? != 0,
        created_at: r.get(17)?,
        updated_at: r.get(18)?,
        started_at: r.get(19)?,
        finished_at: r.get(20)?,
    })
}

/// Atomic claim of next queued job (lowest priority, oldest first).
pub fn claim_next_job(
    worker_id: &str,
    lease_secs: i64,
    now: chrono::DateTime<chrono::Utc>,
) -> AppResult<Option<JobRow>> {
    let mut conn = open_vnext_db()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|e| AppError::Message(e.to_string()))?;

    let id: Option<String> = tx
        .query_row(
            r#"SELECT id FROM jobs
               WHERE status = 'queued' AND cancel_requested = 0
               ORDER BY priority ASC, created_at ASC
               LIMIT 1"#,
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| AppError::Message(e.to_string()))?;

    let Some(id) = id else {
        tx.commit().map_err(|e| AppError::Message(e.to_string()))?;
        return Ok(None);
    };

    let lease = (now + chrono::Duration::seconds(lease_secs)).to_rfc3339();
    let now_s = now.to_rfc3339();
    let changed = tx
        .execute(
            r#"UPDATE jobs SET
                 status = 'running',
                 attempt = attempt + 1,
                 locked_by = ?1,
                 lease_expires_at = ?2,
                 started_at = COALESCE(started_at, ?3),
                 updated_at = ?3,
                 stage = 'running'
               WHERE id = ?4 AND status = 'queued'"#,
            params![worker_id, lease, now_s, id],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;

    if changed == 0 {
        tx.commit().map_err(|e| AppError::Message(e.to_string()))?;
        return Ok(None);
    }

    let row =
        get_job_conn(&tx, &id)?.ok_or_else(|| AppError::Message("claimed job missing".into()))?;
    tx.commit().map_err(|e| AppError::Message(e.to_string()))?;
    Ok(Some(row))
}

pub fn set_job_status(
    id: &str,
    from: JobStatus,
    to: JobStatus,
    now_rfc3339: &str,
    error_json: Option<&str>,
    result_artifact_id: Option<&str>,
) -> AppResult<JobRow> {
    from.transition(to)
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let conn = open_vnext_db()?;
    let finished = matches!(
        to,
        JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
    );
    let clear_lease = matches!(
        to,
        JobStatus::Completed
            | JobStatus::Failed
            | JobStatus::Cancelled
            | JobStatus::Queued
            | JobStatus::Interrupted
    );

    let n = conn
        .execute(
            r#"UPDATE jobs SET
                 status = ?1,
                 updated_at = ?2,
                 error_json = COALESCE(?3, error_json),
                 result_artifact_id = COALESCE(?4, result_artifact_id),
                 finished_at = CASE WHEN ?5 = 1 THEN ?2 ELSE finished_at END,
                 locked_by = CASE WHEN ?6 = 1 THEN NULL ELSE locked_by END,
                 lease_expires_at = CASE WHEN ?6 = 1 THEN NULL ELSE lease_expires_at END,
                 stage = ?1
               WHERE id = ?7 AND status = ?8"#,
            params![
                to.as_str(),
                now_rfc3339,
                error_json,
                result_artifact_id,
                finished as i64,
                clear_lease as i64,
                id,
                from.as_str(),
            ],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    if n == 0 {
        return Err(AppError::Invalid(format!(
            "No se pudo transicionar job {id} de {} a {} (estado cambió)",
            from.as_str(),
            to.as_str()
        )));
    }
    get_job(id)?.ok_or_else(|| AppError::NotFound(id.into()))
}

pub fn request_cancel(id: &str, now_rfc3339: &str) -> AppResult<JobRow> {
    let job = get_job(id)?.ok_or_else(|| AppError::NotFound(id.into()))?;
    let status = JobStatus::parse(&job.status).map_err(|e| AppError::Invalid(e.to_string()))?;
    match status {
        JobStatus::Queued => {
            // queued → cancelling → cancelled immediately (no worker)
            set_job_status(
                id,
                JobStatus::Queued,
                JobStatus::Cancelling,
                now_rfc3339,
                None,
                None,
            )?;
            set_job_status(
                id,
                JobStatus::Cancelling,
                JobStatus::Cancelled,
                now_rfc3339,
                None,
                None,
            )
        }
        JobStatus::Running => {
            let conn = open_vnext_db()?;
            conn.execute(
                "UPDATE jobs SET cancel_requested = 1, status = 'cancelling', updated_at = ?1, stage = 'cancelling' WHERE id = ?2 AND status = 'running'",
                params![now_rfc3339, id],
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
            get_job(id)?.ok_or_else(|| AppError::NotFound(id.into()))
        }
        JobStatus::Cancelling | JobStatus::Cancelled => Ok(job),
        other => Err(AppError::Invalid(format!(
            "No se puede cancelar job en estado {}",
            other.as_str()
        ))),
    }
}

/// Re-queue failed or interrupted job (explicit retry).
pub fn requeue_job(id: &str, now_rfc3339: &str) -> AppResult<JobRow> {
    let job = get_job(id)?.ok_or_else(|| AppError::NotFound(id.into()))?;
    let status = JobStatus::parse(&job.status).map_err(|e| AppError::Invalid(e.to_string()))?;
    match status {
        JobStatus::Failed => set_job_status(
            id,
            JobStatus::Failed,
            JobStatus::Queued,
            now_rfc3339,
            None,
            None,
        ),
        JobStatus::Interrupted => set_job_status(
            id,
            JobStatus::Interrupted,
            JobStatus::Queued,
            now_rfc3339,
            None,
            None,
        ),
        other => Err(AppError::Invalid(format!(
            "Retry solo desde failed/interrupted, no desde {}",
            other.as_str()
        ))),
    }
}

/// Mark running jobs with expired or null lease as interrupted.
pub fn recover_stale_running(now: chrono::DateTime<chrono::Utc>) -> AppResult<u32> {
    let conn = open_vnext_db()?;
    let now_s = now.to_rfc3339();
    let n = conn
        .execute(
            r#"UPDATE jobs SET
                 status = 'interrupted',
                 stage = 'interrupted',
                 locked_by = NULL,
                 lease_expires_at = NULL,
                 updated_at = ?1,
                 error_json = '{"contractVersion":"v1","code":"lease_expired","message":"lease expired or process lost","retryable":true}'
               WHERE status = 'running'
                 AND (lease_expires_at IS NULL OR lease_expires_at < ?1)"#,
            params![now_s],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(n as u32)
}
