//! Job lifecycle use-cases.

use crate::error::{AppError, AppResult};
use crate::vnext::domain::{
    complete_status, validate_complete, ArtifactValidationStatus, CompleteJobCommand, JobKind,
    JobStatus,
};
use crate::vnext::persistence::{
    claim_next_job, enqueue_job, get_artifact, get_job, insert_artifact, recover_stale_running,
    request_cancel, requeue_job, set_job_status, EnqueueJob, InsertArtifact, JobRow,
};

pub const DEFAULT_LEASE_SECS: i64 = 120;
pub const DEFAULT_WORKER: &str = "local-vnext";

pub fn enqueue(
    content_project_id: &str,
    kind: JobKind,
    idempotency_key: &str,
    input_json: &str,
    max_attempts: u32,
) -> AppResult<JobRow> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    enqueue_job(&EnqueueJob {
        id,
        content_project_id: content_project_id.into(),
        kind,
        idempotency_key: idempotency_key.into(),
        max_attempts: max_attempts.max(1),
        priority: 100,
        input_json: input_json.into(),
        now_rfc3339: now,
    })
}

pub fn claim(worker_id: &str) -> AppResult<Option<JobRow>> {
    claim_next_job(worker_id, DEFAULT_LEASE_SECS, chrono::Utc::now())
}

/// Complete only when artifact exists and validation passed (domain + DB).
pub fn complete_job_with_artifact(job_id: &str, artifact_id: &str) -> AppResult<JobRow> {
    let job = get_job(job_id)?.ok_or_else(|| AppError::NotFound(job_id.into()))?;
    let from = JobStatus::parse(&job.status).map_err(|e| AppError::Invalid(e.to_string()))?;
    let art = get_artifact(artifact_id)?.ok_or_else(|| AppError::NotFound(artifact_id.into()))?;
    let passed = ArtifactValidationStatus::parse(&art.validation_status)
        .map_err(|e| AppError::Invalid(e.to_string()))?
        .is_passed();
    let cmd = CompleteJobCommand {
        job_id: job_id.into(),
        result_artifact_id: artifact_id.into(),
        artifact_validation_passed: passed,
    };
    complete_status(from, &cmd).map_err(|e| AppError::Invalid(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();
    set_job_status(
        job_id,
        from,
        JobStatus::Completed,
        &now,
        None,
        Some(artifact_id),
    )
}

pub fn fail_job(job_id: &str, error_json: &str) -> AppResult<JobRow> {
    let job = get_job(job_id)?.ok_or_else(|| AppError::NotFound(job_id.into()))?;
    let from = JobStatus::parse(&job.status).map_err(|e| AppError::Invalid(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();
    set_job_status(
        job_id,
        from,
        JobStatus::Failed,
        &now,
        Some(error_json),
        None,
    )
}

pub fn cancel_job(job_id: &str) -> AppResult<JobRow> {
    let now = chrono::Utc::now().to_rfc3339();
    request_cancel(job_id, &now)
}

pub fn retry_job(job_id: &str) -> AppResult<JobRow> {
    let now = chrono::Utc::now().to_rfc3339();
    requeue_job(job_id, &now)
}

pub fn recover_interrupted() -> AppResult<u32> {
    recover_stale_running(chrono::Utc::now())
}

/// Helper for tests: insert passed artifact and complete.
pub fn insert_passed_artifact_and_complete(
    job_id: &str,
    content_project_id: &str,
    path: &str,
    sha256: &str,
) -> AppResult<JobRow> {
    validate_complete(&CompleteJobCommand {
        job_id: job_id.into(),
        result_artifact_id: "pending".into(),
        artifact_validation_passed: true,
    })
    .ok();
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    insert_artifact(&InsertArtifact {
        id: id.clone(),
        content_project_id: content_project_id.into(),
        kind: "vertical_mp4".into(),
        role: "final_deliverable".into(),
        path: path.into(),
        sha256: sha256.into(),
        byte_size: 1,
        mime_type: "video/mp4".into(),
        created_by_job_id: job_id.into(),
        probe_json: Some(r#"{"durationS":1.0,"hasAudio":true,"hasVideo":true}"#.into()),
        validation_status: ArtifactValidationStatus::Passed,
        validation_notes: None,
        parent_ids: vec![],
        created_at: now,
    })?;
    complete_job_with_artifact(job_id, &id)
}
