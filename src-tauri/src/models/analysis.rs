use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::artifacts::ArtifactRef;
use super::edl::{EditOp, Edl, ExceptionItem, PolicyConfig};
use super::event::Event;
use super::segment::Segment;

/// Full analysis run result (engine output).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRun {
    pub id: String,
    pub media_path: String,
    pub duration: f64,
    pub method: String,
    pub policy: PolicyConfig,
    pub events: Vec<Event>,
    pub edit_ops: Vec<EditOp>,
    pub exceptions: Vec<ExceptionItem>,
    pub edl: Edl,
    /// Legacy UI projection (derived from events + policy + exception resolutions)
    pub segments: Vec<Segment>,
    pub stats: AnalysisStats,
    /// Paths written after export (optional during pure analysis)
    #[serde(default)]
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisStats {
    pub event_count: usize,
    pub silence_event_count: usize,
    pub auto_cut_count: usize,
    pub exception_count: usize,
    pub pending_exception_count: usize,
    pub speech_duration: f64,
    pub silence_duration: f64,
    pub auto_removed_duration: f64,
    pub output_duration: f64,
}

impl AnalysisRun {
    pub fn new_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn pending_exceptions(&self) -> impl Iterator<Item = &ExceptionItem> {
        self.exceptions.iter().filter(|e| e.is_pending())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveExceptionRequest {
    pub exception_id: String,
    /// "accepted" = cut, "rejected" = keep
    pub resolution: String,
}
