//! Canonical transcript (source timeline). SRT/TXT are projections.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptStatus {
    #[default]
    Ready,
    Empty,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptWord {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub span: Span,
    pub text: String,
    #[serde(default)]
    pub words: Vec<TranscriptWord>,
}

impl TranscriptSegment {
    pub fn new(span: Span, text: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            span,
            text: text.into(),
            words: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub id: String,
    pub media_path: String,
    pub run_id: Option<String>,
    pub language: String,
    pub engine: String,
    pub engine_version: Option<String>,
    pub segments: Vec<TranscriptSegment>,
    pub status: TranscriptStatus,
    pub warnings: Vec<String>,
    pub source_hash: Option<String>,
    pub created_at: String,
}

impl Transcript {
    pub fn new(media_path: impl Into<String>, engine: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            media_path: media_path.into(),
            run_id: None,
            language: "und".into(),
            engine: engine.into(),
            engine_version: None,
            segments: Vec::new(),
            status: TranscriptStatus::Empty,
            warnings: Vec::new(),
            source_hash: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn full_text(&self) -> String {
        self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn to_srt(&self) -> String {
        let mut out = String::new();
        for (i, seg) in self.segments.iter().enumerate() {
            out.push_str(&format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                format_srt_ts(seg.span.start),
                format_srt_ts(seg.span.end),
                seg.text.trim()
            ));
        }
        out
    }

    pub fn to_txt_timed(&self) -> String {
        let mut out = String::new();
        for seg in &self.segments {
            out.push_str(&format!(
                "[{} → {}]\n{}\n\n",
                format_clock(seg.span.start),
                format_clock(seg.span.end),
                seg.text.trim()
            ));
        }
        out
    }
}

fn format_srt_ts(t: f64) -> String {
    let t = t.max(0.0);
    let h = (t / 3600.0).floor() as u32;
    let m = ((t % 3600.0) / 60.0).floor() as u32;
    let s = (t % 60.0).floor() as u32;
    let ms = ((t.fract()) * 1000.0).round() as u32;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

fn format_clock(t: f64) -> String {
    let t = t.max(0.0);
    let h = (t / 3600.0).floor() as u32;
    let m = ((t % 3600.0) / 60.0).floor() as u32;
    let s = t % 60.0;
    format!("{h:02}:{m:02}:{s:06.3}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srt_and_txt_roundtrip_shape() {
        let mut tr = Transcript::new("a.mp4", "test");
        tr.segments.push(TranscriptSegment::new(
            Span::new(1.5, 3.25),
            "Hola mundo",
        ));
        tr.status = TranscriptStatus::Ready;
        let srt = tr.to_srt();
        assert!(srt.contains("-->"));
        assert!(srt.contains("Hola mundo"));
        let txt = tr.to_txt_timed();
        assert!(txt.contains("["));
        assert!(txt.contains("Hola mundo"));
    }
}
