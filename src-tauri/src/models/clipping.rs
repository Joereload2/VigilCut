//! Intelligent clipping domain (separate from silence Keep/Cut segments).
//!
//! Detection → scoring → human review → framing → export stay distinct.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::Span;

// ── Profiles ───────────────────────────────────────────────────────────────

/// Target duration band for generated clips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DurationProfile {
    Micro,     // 10–20s
    #[default]
    Short,     // 20–40s
    Standard,  // 40–60s
    Extended,  // 60–90s
    Custom,
}

impl DurationProfile {
    pub fn bounds(self) -> (f64, f64, f64) {
        // (min, ideal, max)
        match self {
            Self::Micro => (10.0, 15.0, 20.0),
            Self::Short => (20.0, 30.0, 40.0),
            Self::Standard => (40.0, 50.0, 60.0),
            Self::Extended => (60.0, 75.0, 90.0),
            Self::Custom => (15.0, 30.0, 60.0),
        }
    }
}

/// How aggressive preselection is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SelectionProfile {
    Conservative,
    #[default]
    Balanced,
    Broad,
    Exploratory,
}

impl SelectionProfile {
    /// Max preselected + min score floor for auto-preselect.
    pub fn limits(self) -> (usize, f64) {
        match self {
            Self::Conservative => (5, 0.72),
            Self::Balanced => (8, 0.58),
            Self::Broad => (14, 0.45),
            Self::Exploratory => (20, 0.35),
        }
    }
}

// ── Transcript ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptSourceKind {
    SrtFile,
    VttFile,
    WhisperCli,
    AnalysisSpeechFallback,
    Manual,
}

/// Atomic timed text unit from any transcript provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptCue {
    pub id: String,
    pub span: Span,
    pub text: String,
}

impl TranscriptCue {
    pub fn new(span: Span, text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            span,
            text: text.into(),
        }
    }
}

/// Semantic unit: one or more cues merged into a coherent speech block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticUnit {
    pub id: String,
    pub span: Span,
    pub text: String,
    pub cue_ids: Vec<String>,
    /// Rough energy proxy 0..1 when available
    pub energy: f64,
}

// ── Scoring ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClipScoreBreakdown {
    pub hook_quality: f64,
    pub semantic_coherence: f64,
    pub standalone: f64,
    pub clarity: f64,
    pub energy: f64,
    pub information_density: f64,
    pub has_conclusion: f64,
    pub duration_fit: f64,
    pub silence_penalty: f64,
    pub incomplete_penalty: f64,
}

impl ClipScoreBreakdown {
    /// Weighted total 0..100 (explainable, not “viral probability”).
    pub fn total(&self) -> f64 {
        let raw = self.hook_quality * 0.16
            + self.semantic_coherence * 0.14
            + self.standalone * 0.14
            + self.clarity * 0.10
            + self.energy * 0.10
            + self.information_density * 0.10
            + self.has_conclusion * 0.10
            + self.duration_fit * 0.12
            - self.silence_penalty * 0.08
            - self.incomplete_penalty * 0.10;
        (raw * 100.0).clamp(0.0, 100.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipReason {
    pub code: String,
    pub label: String,
    pub weight: f64,
}

// ── Framing ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FramingMode {
    /// Center crop to 9:16 (static). Tracking can replace later.
    #[default]
    AutoCenter,
    Manual,
    BlurredBackground,
    FitWithBars,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipFraming {
    pub mode: FramingMode,
    /// Normalized crop center X in source frame (0..1)
    pub center_x: f64,
    /// Normalized crop center Y (0..1)
    pub center_y: f64,
    /// Zoom factor >= 1.0
    pub zoom: f64,
    pub output_width: u32,
    pub output_height: u32,
    /// Reserved for future face-tracking keyframes
    #[serde(default)]
    pub tracking_ready: bool,
}

impl Default for ClipFraming {
    fn default() -> Self {
        Self {
            mode: FramingMode::AutoCenter,
            center_x: 0.5,
            center_y: 0.45,
            zoom: 1.0,
            output_width: 1080,
            output_height: 1920,
            tracking_ready: false,
        }
    }
}

// ── Candidate lifecycle ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClipReviewStatus {
    #[default]
    Suggested,
    Preselected,
    Approved,
    Rejected,
    Modified,
    Exporting,
    Exported,
    Error,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipCandidate {
    pub id: String,
    pub analysis_run_id: String,
    pub source_media_path: String,
    pub start: f64,
    pub end: f64,
    pub duration: f64,
    pub transcript: String,
    pub title: String,
    pub summary: String,
    /// Total score 0..100
    pub score: f64,
    /// Model confidence 0..1
    pub confidence: f64,
    pub breakdown: ClipScoreBreakdown,
    pub reasons: Vec<ClipReason>,
    pub warnings: Vec<String>,
    pub strengths: Vec<String>,
    pub risks: Vec<String>,
    pub status: ClipReviewStatus,
    /// Variants share this id; principal is highest score
    pub variant_group_id: String,
    pub is_primary_variant: bool,
    pub framing: ClipFraming,
    /// Original proposal before user edits
    pub original_start: f64,
    pub original_end: f64,
    pub export_path: Option<String>,
    pub error: Option<String>,
}

impl ClipCandidate {
    pub fn set_span(&mut self, start: f64, end: f64) {
        let start = start.max(0.0);
        let end = end.max(start + 0.1);
        if (start - self.start).abs() > 0.01 || (end - self.end).abs() > 0.01 {
            if !matches!(
                self.status,
                ClipReviewStatus::Approved | ClipReviewStatus::Rejected
            ) {
                self.status = ClipReviewStatus::Modified;
            } else {
                self.status = ClipReviewStatus::Modified;
            }
        }
        self.start = start;
        self.end = end;
        self.duration = end - start;
    }

    pub fn restore_original_span(&mut self) {
        self.start = self.original_start;
        self.end = self.original_end;
        self.duration = (self.end - self.start).max(0.0);
    }
}

// ── Run config & result ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClippingOptions {
    pub duration_profile: DurationProfile,
    pub selection_profile: SelectionProfile,
    pub min_duration: Option<f64>,
    pub ideal_duration: Option<f64>,
    pub max_duration: Option<f64>,
    /// Padding before semantic start (seconds)
    pub pad_before: f64,
    /// Padding after semantic end (seconds)
    pub pad_after: f64,
    /// Optional path to SRT/VTT; if None, speech-event fallback
    pub transcript_path: Option<String>,
    pub prefer_whisper: bool,
    pub max_candidates: usize,
}

impl Default for ClippingOptions {
    fn default() -> Self {
        let (min, ideal, max) = DurationProfile::Short.bounds();
        Self {
            duration_profile: DurationProfile::Short,
            selection_profile: SelectionProfile::Balanced,
            min_duration: Some(min),
            ideal_duration: Some(ideal),
            max_duration: Some(max),
            pad_before: 0.25,
            pad_after: 0.35,
            transcript_path: None,
            prefer_whisper: true,
            max_candidates: 40,
        }
    }
}

impl ClippingOptions {
    pub fn resolved_bounds(&self) -> (f64, f64, f64) {
        let (dmin, dideal, dmax) = self.duration_profile.bounds();
        (
            self.min_duration.unwrap_or(dmin),
            self.ideal_duration.unwrap_or(dideal),
            self.max_duration.unwrap_or(dmax),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClippingSummary {
    pub source_duration: f64,
    pub analysis_seconds: f64,
    pub candidates_found: usize,
    pub preselected: usize,
    pub high_confidence: usize,
    pub needs_review: usize,
    pub discarded: usize,
    pub best_score: f64,
    pub selected_total_duration: f64,
    pub transcript_source: TranscriptSourceKind,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClippingRun {
    pub id: String,
    pub media_path: String,
    pub source_duration: f64,
    pub options: ClippingOptions,
    pub candidates: Vec<ClipCandidate>,
    pub summary: ClippingSummary,
    pub created_at: String,
}

impl ClippingRun {
    pub fn new_id() -> String {
        Uuid::new_v4().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipExportRequest {
    pub media_path: String,
    pub output_dir: String,
    pub candidate_ids: Vec<String>,
    pub framing_override: Option<ClipFraming>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipExportResult {
    pub candidate_id: String,
    pub ok: bool,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_breakdown_clamped() {
        let b = ClipScoreBreakdown {
            hook_quality: 1.0,
            semantic_coherence: 1.0,
            standalone: 1.0,
            clarity: 1.0,
            energy: 1.0,
            information_density: 1.0,
            has_conclusion: 1.0,
            duration_fit: 1.0,
            silence_penalty: 0.0,
            incomplete_penalty: 0.0,
        };
        // Weights sum to 0.96 when all factors are 1 and penalties are 0
        assert!(b.total() > 90.0 && b.total() <= 100.0);
    }

    #[test]
    fn duration_profiles_ordered() {
        let m = DurationProfile::Micro.bounds();
        let e = DurationProfile::Extended.bounds();
        assert!(m.2 < e.0);
    }
}
