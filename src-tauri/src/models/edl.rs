use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::Span;

/// Montage operation proposed by a policy (L2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditOp {
    pub id: String,
    pub op: EditOpKind,
    pub span: Span,
    pub priority: i32,
    pub source_event_ids: Vec<String>,
    pub rationale: String,
    /// true if policy auto-applied without human review
    pub auto_applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditOpKind {
    RemoveSpan,
    KeepSpan,
}

impl EditOp {
    pub fn remove(
        span: Span,
        event_ids: Vec<String>,
        rationale: impl Into<String>,
        auto_applied: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            op: EditOpKind::RemoveSpan,
            span,
            priority: 100,
            source_event_ids: event_ids,
            rationale: rationale.into(),
            auto_applied,
        }
    }
}

/// Exception requiring human supervision (low confidence / conflict).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionItem {
    pub id: String,
    pub event_ids: Vec<String>,
    pub reason: ExceptionReason,
    pub span: Span,
    pub confidence: f64,
    pub suggested_op: EditOpKind,
    pub rationale: String,
    /// Human resolution: none | accepted | rejected
    pub resolution: ExceptionResolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionReason {
    LowConfidence,
    PolicyConflict,
    DurationEdge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExceptionResolution {
    #[default]
    Pending,
    /// Accept suggested remove (cut)
    Accepted,
    /// Reject suggestion — keep in final video
    Rejected,
}

impl ExceptionItem {
    pub fn new(
        event_ids: Vec<String>,
        reason: ExceptionReason,
        span: Span,
        confidence: f64,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_ids,
            reason,
            span,
            confidence,
            suggested_op: EditOpKind::RemoveSpan,
            rationale: rationale.into(),
            resolution: ExceptionResolution::Pending,
        }
    }

    pub fn is_pending(&self) -> bool {
        matches!(self.resolution, ExceptionResolution::Pending)
    }
}

/// Edit Decision List — continuous keep ranges for render (L3).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edl {
    pub media_path: String,
    pub source_duration: f64,
    /// Ordered non-overlapping keep ranges on source timeline
    pub video_track: Vec<Span>,
    pub output_duration: f64,
    pub removed_duration: f64,
}

impl Edl {
    pub fn from_remove_spans(
        media_path: impl Into<String>,
        source_duration: f64,
        remove: &[(f64, f64)],
    ) -> Self {
        let source_duration = if !source_duration.is_finite() || source_duration < 0.0 {
            0.0
        } else {
            source_duration
        };
        let mut cuts: Vec<(f64, f64)> = remove
            .iter()
            .map(|(s, e)| {
                let s = if s.is_finite() { (*s).max(0.0) } else { 0.0 };
                let e = if e.is_finite() {
                    (*e).min(source_duration).max(s)
                } else {
                    s
                };
                (s, e)
            })
            .filter(|(s, e)| e > s && *s < source_duration)
            .collect();
        cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Merge overlapping removes
        let mut merged: Vec<(f64, f64)> = Vec::new();
        for (s, e) in cuts {
            if let Some(last) = merged.last_mut() {
                if s <= last.1 + 0.02 {
                    last.1 = last.1.max(e);
                    continue;
                }
            }
            merged.push((s, e));
        }

        let mut keep = Vec::new();
        let mut cursor = 0.0_f64;
        for (s, e) in &merged {
            if *s > cursor + 0.01 {
                keep.push(Span::new(cursor, *s));
            }
            cursor = cursor.max(*e);
        }
        if cursor < source_duration - 0.01 {
            keep.push(Span::new(cursor, source_duration));
        }
        if keep.is_empty() && source_duration > 0.0 {
            // Everything removed — keep nothing? Prefer keep full to avoid empty export
            keep.push(Span::new(0.0, source_duration));
        }

        let output_duration: f64 = keep.iter().map(|s| s.duration()).sum();
        let removed_duration = (source_duration - output_duration).max(0.0);

        Self {
            media_path: media_path.into(),
            source_duration,
            video_track: keep,
            output_duration,
            removed_duration,
        }
    }

    pub fn keep_ranges(&self) -> Vec<(f64, f64)> {
        self.video_track.iter().map(|s| (s.start, s.end)).collect()
    }
}

#[cfg(test)]
mod edl_edge_tests {
    use super::*;

    #[test]
    fn rejects_nan_remove_spans_gracefully() {
        let edl = Edl::from_remove_spans("x.mp4", 10.0, &[(f64::NAN, 2.0), (3.0, f64::INFINITY)]);
        assert!(edl.output_duration > 0.0);
        assert!(edl.output_duration <= 10.0);
    }

    #[test]
    fn clamps_out_of_range() {
        let edl = Edl::from_remove_spans("x.mp4", 10.0, &[(-5.0, 2.0), (8.0, 50.0)]);
        for s in &edl.video_track {
            assert!(s.start >= 0.0);
            assert!(s.end <= 10.0 + 0.001);
        }
    }

    #[test]
    fn zero_duration_source() {
        let edl = Edl::from_remove_spans("x.mp4", 0.0, &[(0.0, 1.0)]);
        assert_eq!(edl.source_duration, 0.0);
    }

    #[test]
    fn full_silence_keep_fallback() {
        // Removing everything falls back to keep full (avoid empty export)
        let edl = Edl::from_remove_spans("x.mp4", 5.0, &[(0.0, 5.0)]);
        assert!((edl.output_duration - 5.0).abs() < 0.01);
    }
}

/// Policy knobs for silence auto-approve (factory defaults).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyConfig {
    /// Silences with score >= this are auto-cut without human review
    pub auto_approve_min_score: f64,
    /// Minimum silence duration (seconds) to consider
    pub min_silence_duration: f64,
    /// Padding preserved around speech (seconds)
    pub padding: f64,
    /// Detection threshold (passed to detector)
    pub threshold: f64,
    pub prefer_silero: bool,
    /// Run Whisper CLI during silence analysis (slow; off by default — use for fillers / captions).
    #[serde(default)]
    pub prefer_whisper: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            // FFmpeg silence scores ~0.78–0.92; factory default auto-cuts most gaps
            auto_approve_min_score: 0.80,
            min_silence_duration: 0.4,
            padding: 0.12,
            threshold: 0.5,
            prefer_silero: true,
            prefer_whisper: false,
        }
    }
}
