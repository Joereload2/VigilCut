use serde::{Deserialize, Serialize};

use super::error::{DomainError, DomainResult};

/// Canonical job lifecycle (JobSnapshotV1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    WaitingReview,
    Completed,
    Failed,
    Interrupted,
    Cancelling,
    Cancelled,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::WaitingReview => "waiting_review",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
            Self::Cancelling => "cancelling",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "waiting_review" => Ok(Self::WaitingReview),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "interrupted" => Ok(Self::Interrupted),
            "cancelling" => Ok(Self::Cancelling),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(DomainError::new(
                "invalid_job_status",
                format!("Estado de job desconocido: {other}"),
                false,
            )),
        }
    }

    /// Pure transition table — no side effects.
    pub fn can_transition(self, to: Self) -> bool {
        use JobStatus::*;
        matches!(
            (self, to),
            (Queued, Running)
                | (Queued, Cancelling)
                | (Running, WaitingReview)
                | (Running, Completed)
                | (Running, Failed)
                | (Running, Interrupted)
                | (Running, Cancelling)
                | (WaitingReview, Queued)
                | (WaitingReview, Running)
                | (Failed, Queued)
                | (Interrupted, Queued)
                | (Cancelling, Cancelled)
        )
    }

    pub fn transition(self, to: Self) -> DomainResult<Self> {
        if self.can_transition(to) {
            Ok(to)
        } else {
            Err(DomainError::invalid_transition(self.as_str(), to.as_str()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_transitions() {
        let cases = [
            (JobStatus::Queued, JobStatus::Running),
            (JobStatus::Queued, JobStatus::Cancelling),
            (JobStatus::Running, JobStatus::WaitingReview),
            (JobStatus::Running, JobStatus::Completed),
            (JobStatus::Running, JobStatus::Failed),
            (JobStatus::Running, JobStatus::Interrupted),
            (JobStatus::Running, JobStatus::Cancelling),
            (JobStatus::WaitingReview, JobStatus::Queued),
            (JobStatus::WaitingReview, JobStatus::Running),
            (JobStatus::Failed, JobStatus::Queued),
            (JobStatus::Interrupted, JobStatus::Queued),
            (JobStatus::Cancelling, JobStatus::Cancelled),
        ];
        for (from, to) in cases {
            assert_eq!(from.transition(to).unwrap(), to, "{from:?} → {to:?}");
        }
    }

    #[test]
    fn invalid_transitions_rejected() {
        let bad = [
            (JobStatus::Completed, JobStatus::Queued),
            (JobStatus::Cancelled, JobStatus::Running),
            (JobStatus::Queued, JobStatus::Completed),
            (JobStatus::Failed, JobStatus::Completed),
            (JobStatus::Interrupted, JobStatus::Completed),
            (JobStatus::WaitingReview, JobStatus::Completed),
            (JobStatus::Cancelling, JobStatus::Running),
            (JobStatus::Running, JobStatus::Queued),
        ];
        for (from, to) in bad {
            assert!(
                from.transition(to).is_err(),
                "should reject {from:?} → {to:?}"
            );
        }
    }

    #[test]
    fn parse_roundtrip() {
        for s in [
            "queued",
            "running",
            "waiting_review",
            "completed",
            "failed",
            "interrupted",
            "cancelling",
            "cancelled",
        ] {
            assert_eq!(JobStatus::parse(s).unwrap().as_str(), s);
        }
    }
}
