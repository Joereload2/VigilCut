use serde::{Deserialize, Serialize};

use super::job_status::JobStatus;

/// Suggested UI next step for a ContentProject (derived, not stored as sole truth).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextAction {
    Ingest,
    Transcribe,
    GenerateCandidates,
    Review,
    Render,
    Done,
    ResolveFailedJob,
}

impl NextAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ingest => "ingest",
            Self::Transcribe => "transcribe",
            Self::GenerateCandidates => "generate_candidates",
            Self::Review => "review",
            Self::Render => "render",
            Self::Done => "done",
            Self::ResolveFailedJob => "resolve_failed_job",
        }
    }
}

/// Inputs for pure derivation of next action.
#[derive(Debug, Clone, Default)]
pub struct NextActionInput {
    pub has_source_media: bool,
    pub has_probe: bool,
    pub has_transcript: bool,
    pub candidate_count: usize,
    pub approved_count: usize,
    pub has_failed_or_interrupted_job: bool,
    pub has_completed_render: bool,
    /// Any job currently running / queued that blocks idle next-action.
    pub has_active_job: bool,
    pub latest_job_status: Option<JobStatus>,
}

/// Pure policy (Phase 3). Extended in later phases without breaking callers.
pub fn derive_next_action(i: &NextActionInput) -> NextAction {
    if i.has_failed_or_interrupted_job {
        return NextAction::ResolveFailedJob;
    }
    if !i.has_source_media {
        return NextAction::Ingest;
    }
    if !i.has_probe {
        return NextAction::Ingest;
    }
    if !i.has_transcript {
        return NextAction::Transcribe;
    }
    if i.candidate_count == 0 {
        return NextAction::GenerateCandidates;
    }
    if i.approved_count == 0 {
        return NextAction::Review;
    }
    if !i.has_completed_render {
        return NextAction::Render;
    }
    NextAction::Done
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_basic_flow() {
        assert_eq!(
            derive_next_action(&NextActionInput::default()),
            NextAction::Ingest
        );
        assert_eq!(
            derive_next_action(&NextActionInput {
                has_source_media: true,
                has_probe: true,
                has_transcript: true,
                candidate_count: 0,
                ..Default::default()
            }),
            NextAction::GenerateCandidates
        );
        assert_eq!(
            derive_next_action(&NextActionInput {
                has_source_media: true,
                has_probe: true,
                has_transcript: true,
                candidate_count: 3,
                approved_count: 0,
                ..Default::default()
            }),
            NextAction::Review
        );
        assert_eq!(
            derive_next_action(&NextActionInput {
                has_source_media: true,
                has_probe: true,
                has_transcript: true,
                candidate_count: 3,
                approved_count: 1,
                has_completed_render: false,
                ..Default::default()
            }),
            NextAction::Render
        );
        assert_eq!(
            derive_next_action(&NextActionInput {
                has_source_media: true,
                has_probe: true,
                has_transcript: true,
                candidate_count: 1,
                approved_count: 1,
                has_completed_render: true,
                ..Default::default()
            }),
            NextAction::Done
        );
        assert_eq!(
            derive_next_action(&NextActionInput {
                has_source_media: true,
                has_probe: true,
                has_transcript: true,
                candidate_count: 1,
                has_failed_or_interrupted_job: true,
                ..Default::default()
            }),
            NextAction::ResolveFailedJob
        );
    }
}
