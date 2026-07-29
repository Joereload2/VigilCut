use serde::{Deserialize, Serialize};

use super::error::{DomainError, DomainResult};
use super::job_status::JobStatus;

/// Job kinds allowed in MVP (extensible later via migration/docs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    IngestProbe,
    Transcribe,
    GenerateShortCandidates,
    VerticalRender,
}

impl JobKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::IngestProbe => "ingest_probe",
            Self::Transcribe => "transcribe",
            Self::GenerateShortCandidates => "generate_short_candidates",
            Self::VerticalRender => "vertical_render",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "ingest_probe" => Ok(Self::IngestProbe),
            "transcribe" => Ok(Self::Transcribe),
            "generate_short_candidates" => Ok(Self::GenerateShortCandidates),
            "vertical_render" => Ok(Self::VerticalRender),
            other => Err(DomainError::new(
                "invalid_job_kind",
                format!("Kind de job desconocido: {other}"),
                false,
            )),
        }
    }
}

/// Domain command: mark job completed only with a validated artifact id.
#[derive(Debug, Clone)]
pub struct CompleteJobCommand {
    pub job_id: String,
    pub result_artifact_id: String,
    pub artifact_validation_passed: bool,
}

pub fn validate_complete(cmd: &CompleteJobCommand) -> DomainResult<()> {
    if cmd.result_artifact_id.trim().is_empty() {
        return Err(DomainError::artifact_required().with_job(&cmd.job_id));
    }
    if !cmd.artifact_validation_passed {
        return Err(DomainError::artifact_not_validated().with_job(&cmd.job_id));
    }
    Ok(())
}

/// Apply status transition after complete validation.
pub fn complete_status(from: JobStatus, cmd: &CompleteJobCommand) -> DomainResult<JobStatus> {
    validate_complete(cmd)?;
    from.transition(JobStatus::Completed)
        .map_err(|e| DomainError {
            job_id: Some(cmd.job_id.clone()),
            ..e
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_requires_validated_artifact() {
        let missing = CompleteJobCommand {
            job_id: "j1".into(),
            result_artifact_id: String::new(),
            artifact_validation_passed: true,
        };
        assert!(validate_complete(&missing).is_err());

        let bad = CompleteJobCommand {
            job_id: "j1".into(),
            result_artifact_id: "a1".into(),
            artifact_validation_passed: false,
        };
        assert!(validate_complete(&bad).is_err());
        assert!(complete_status(JobStatus::Running, &bad).is_err());

        let ok = CompleteJobCommand {
            job_id: "j1".into(),
            result_artifact_id: "a1".into(),
            artifact_validation_passed: true,
        };
        assert_eq!(
            complete_status(JobStatus::Running, &ok).unwrap(),
            JobStatus::Completed
        );
        assert!(complete_status(JobStatus::Queued, &ok).is_err());
    }
}
