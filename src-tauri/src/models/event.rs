use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Temporal span on the source media timeline (seconds).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Span {
    pub start: f64,
    pub end: f64,
}

impl Span {
    pub fn new(start: f64, end: f64) -> Self {
        Self {
            start: start.max(0.0),
            end: end.max(start),
        }
    }

    pub fn duration(&self) -> f64 {
        (self.end - self.start).max(0.0)
    }
}

/// Atomic analysis evidence (L1). Detectors emit only Events — never edit decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub run_id: String,
    /// Namespaced type, e.g. "audio.silence", "speech.filler"
    #[serde(rename = "type")]
    pub event_type: String,
    pub detector: String,
    pub span: Span,
    /// Confidence 0..1
    pub score: f64,
    pub payload: serde_json::Value,
    pub tags: Vec<String>,
}

impl Event {
    pub fn new(
        run_id: impl Into<String>,
        event_type: impl Into<String>,
        detector: impl Into<String>,
        span: Span,
        score: f64,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            event_type: event_type.into(),
            detector: detector.into(),
            span,
            score: score.clamp(0.0, 1.0),
            payload,
            tags: Vec::new(),
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

pub const TYPE_AUDIO_SILENCE: &str = "audio.silence";
pub const TYPE_AUDIO_SPEECH: &str = "audio.speech";
pub const TYPE_AUDIO_BREATH: &str = "audio.breath";
pub const TYPE_SPEECH_FILLER: &str = "speech.filler";
pub const TYPE_STRUCTURE_CHAPTER: &str = "structure.chapter";
/// Prefer `short.candidate` (canonical detector id).
pub const TYPE_STRUCTURE_SHORT: &str = "short.candidate";
