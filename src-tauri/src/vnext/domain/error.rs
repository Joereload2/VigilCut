use serde::{Deserialize, Serialize};

/// Structured application error (contract ApplicationErrorV1, domain side).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DomainError {
    pub contract_version: &'static str,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

impl DomainError {
    pub const CONTRACT_VERSION: &'static str = "v1";

    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            contract_version: Self::CONTRACT_VERSION,
            code: code.into(),
            message: message.into(),
            retryable,
            job_id: None,
        }
    }

    pub fn with_job(mut self, job_id: impl Into<String>) -> Self {
        self.job_id = Some(job_id.into());
        self
    }

    pub fn invalid_transition(from: &str, to: &str) -> Self {
        Self::new(
            "job_invalid_transition",
            format!("Transición de job no permitida: {from} → {to}"),
            false,
        )
    }

    pub fn artifact_required() -> Self {
        Self::new(
            "artifact_required_for_complete",
            "No se puede completar un job sin artifact validado",
            false,
        )
    }

    pub fn artifact_not_validated() -> Self {
        Self::new(
            "artifact_validation_failed",
            "El artifact no tiene validationStatus=passed",
            false,
        )
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DomainError {}

pub type DomainResult<T> = Result<T, DomainError>;
