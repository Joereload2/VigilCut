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

/// A continuous span on the source media timeline (seconds).
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
}

impl Default for SilenceDetectionOptions {
    fn default() -> Self {
        Self {
            min_silence_duration: 0.4,
            padding: 0.12,
            threshold: 0.5,
            prefer_silero: true,
            auto_cut_silence: true,
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
