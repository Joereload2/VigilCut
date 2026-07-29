use serde::{Deserialize, Serialize};

use super::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactValidationStatus {
    Pending,
    Passed,
    Failed,
}

impl ArtifactValidationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            other => Err(DomainError::new(
                "invalid_validation_status",
                format!("validationStatus desconocido: {other}"),
                false,
            )),
        }
    }

    pub fn is_passed(self) -> bool {
        matches!(self, Self::Passed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Intermediate,
    FinalDeliverable,
    Sidecar,
}

impl ArtifactRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Intermediate => "intermediate",
            Self::FinalDeliverable => "final_deliverable",
            Self::Sidecar => "sidecar",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "intermediate" => Ok(Self::Intermediate),
            "final_deliverable" => Ok(Self::FinalDeliverable),
            "sidecar" => Ok(Self::Sidecar),
            other => Err(DomainError::new(
                "invalid_artifact_role",
                format!("role desconocido: {other}"),
                false,
            )),
        }
    }
}

/// Input path and output path must never be the same file (safe_paths invariant).
pub fn assert_distinct_paths(input: &str, output: &str) -> DomainResult<()> {
    let a = normalize_path(input);
    let b = normalize_path(output);
    if a == b {
        return Err(DomainError::new(
            "path_input_equals_output",
            "La salida no puede ser el mismo path que la entrada",
            false,
        ));
    }
    Ok(())
}

fn normalize_path(s: &str) -> String {
    s.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_same_path() {
        assert!(assert_distinct_paths(r"C:\a\v.mp4", r"C:\a\v.mp4").is_err());
        assert!(assert_distinct_paths(r"C:\a\v.mp4", r"C:\a\out.mp4").is_ok());
    }
}
