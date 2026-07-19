use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Kind of content detected in a timeline span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentKind {
    Speech,
    Silence,
    Music,
    Noise,
    ClipCandidate,
    Manual,
}

/// Whether the user keeps or cuts this segment in the final export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentDecision {
    Keep,
    Cut,
    Pending,
}

/// Legacy UI projection of analysis (derived view — not the engine source of truth).
/// Prefer Events + EDL in new code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub kind: SegmentKind,
    pub decision: SegmentDecision,
    /// Confidence 0..1 from detector (VAD, etc.)
    pub confidence: f64,
    /// Optional human label / note
    pub label: Option<String>,
    /// Energy / loudness estimate (dBFS) when available
    pub energy_db: Option<f64>,
    /// Linked analysis event (if any)
    #[serde(default)]
    pub event_id: Option<String>,
    /// Policy auto-applied this decision (no human needed)
    #[serde(default)]
    pub auto_applied: bool,
    /// Needs human supervision (exception queue)
    #[serde(default)]
    pub needs_review: bool,
}

impl Segment {
    pub fn new(start: f64, end: f64, kind: SegmentKind, decision: SegmentDecision) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            start,
            end,
            kind,
            decision,
            confidence: 1.0,
            label: None,
            energy_db: None,
            event_id: None,
            auto_applied: false,
            needs_review: false,
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end - self.start).max(0.0)
    }

    pub fn toggle_decision(&mut self) {
        self.decision = match self.decision {
            SegmentDecision::Keep => SegmentDecision::Cut,
            SegmentDecision::Cut => SegmentDecision::Keep,
            SegmentDecision::Pending => SegmentDecision::Keep,
        };
    }

    pub fn is_kept(&self) -> bool {
        matches!(self.decision, SegmentDecision::Keep)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentEdit {
    pub id: String,
    pub decision: Option<SegmentDecision>,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SilenceDetectionOptions {
    /// Minimum silence duration in seconds to mark as cuttable (default 0.4)
    pub min_silence_duration: f64,
    /// Padding kept around speech edges in seconds (default 0.12)
    pub padding: f64,
    /// Silero / energy threshold 0..1 (default 0.5)
    pub threshold: f64,
    /// Prefer Silero VAD when model available; falls back to FFmpeg silencedetect
    pub prefer_silero: bool,
    /// Auto-mark silences as Cut (user can still toggle)
    pub auto_cut_silence: bool,
    /// Policy: min event score for auto-cut without human review (default 0.80)
    #[serde(default = "default_auto_approve")]
    pub auto_approve_min_score: f64,
}

fn default_auto_approve() -> f64 {
    0.80
}

impl Default for SilenceDetectionOptions {
    fn default() -> Self {
        Self {
            min_silence_duration: 0.4,
            padding: 0.12,
            threshold: 0.5,
            prefer_silero: true,
            auto_cut_silence: true,
            auto_approve_min_score: 0.80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SilenceDetectionResult {
    pub media_path: String,
    pub duration: f64,
    pub segments: Vec<Segment>,
    pub method: String,
    pub speech_duration: f64,
    pub silence_duration: f64,
    pub cut_duration: f64,
}
