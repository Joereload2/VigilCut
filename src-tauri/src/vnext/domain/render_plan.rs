use serde::{Deserialize, Serialize};

use crate::models::clipping::ClipFraming;

use super::error::{DomainError, DomainResult};

/// Closed MVP subtitle preset id (Phase 4 — not expanded without approval).
pub const SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1: &str = "safe_center_bottom_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleCueV1 {
    pub id: String,
    /// Seconds on **output** short timeline (0 = start of clip).
    pub start_s: f64,
    pub end_s: f64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleBurnPlanV1 {
    pub enabled: bool,
    pub preset_id: String,
    pub cues: Vec<SubtitleCueV1>,
    #[serde(default)]
    pub source_transcript_artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputSpecV1 {
    pub width: u32,
    pub height: u32,
    pub video_codec: String,
    pub audio_codec: String,
    pub audio_bitrate_kbps: u32,
    pub crf: u32,
    pub pixel_format: String,
    pub faststart: bool,
}

impl Default for OutputSpecV1 {
    fn default() -> Self {
        Self {
            width: 1080,
            height: 1920,
            video_codec: "libx264".into(),
            audio_codec: "aac".into(),
            audio_bitrate_kbps: 160,
            crf: 20,
            pixel_format: "yuv420p".into(),
            faststart: true,
        }
    }
}

/// Immutable VerticalRenderPlanV1 document.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerticalRenderPlanV1 {
    pub contract_version: String,
    pub id: String,
    pub content_project_id: String,
    pub recipe_id: String,
    pub candidate_id: String,
    pub source_media_path: String,
    pub source_start_s: f64,
    pub source_end_s: f64,
    pub framing: ClipFraming,
    pub subtitles: SubtitleBurnPlanV1,
    pub output_spec: OutputSpecV1,
    pub created_at: String,
    pub created_by: String,
}

impl VerticalRenderPlanV1 {
    pub fn validate(&self) -> DomainResult<()> {
        if self.contract_version != "v1" {
            return Err(DomainError::new(
                "invalid_contract_version",
                format!("contractVersion no soportado: {}", self.contract_version),
                false,
            ));
        }
        if self.source_end_s <= self.source_start_s {
            return Err(DomainError::new(
                "invalid_span",
                "sourceEndS debe ser mayor que sourceStartS",
                false,
            ));
        }
        if self.output_spec.width != 1080 || self.output_spec.height != 1920 {
            return Err(DomainError::new(
                "invalid_output_geometry",
                "MVP exige 1080×1920",
                false,
            ));
        }
        if self.subtitles.enabled
            && self.subtitles.preset_id != SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1
        {
            return Err(DomainError::new(
                "subtitle_preset_not_approved",
                format!(
                    "Preset de subtítulos no aprobado: {}",
                    self.subtitles.preset_id
                ),
                false,
            ));
        }
        for c in &self.subtitles.cues {
            if c.end_s <= c.start_s {
                return Err(DomainError::new(
                    "invalid_cue",
                    format!("Cue {} tiene tiempos inválidos", c.id),
                    false,
                ));
            }
        }
        Ok(())
    }

    pub fn duration_s(&self) -> f64 {
        (self.source_end_s - self.source_start_s).max(0.15)
    }
}

/// Build ASS force_style for safe bottom-center burn-in (preset closed).
pub fn ass_force_style_safe_center_bottom() -> &'static str {
    // Fontsize relative to 1080x1920; MarginV keeps text in lower safe zone.
    "FontName=Arial,FontSize=42,PrimaryColour=&H00FFFFFF,OutlineColour=&H00000000,BorderStyle=1,Outline=3,Shadow=0,Alignment=2,MarginV=160"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_geometry_and_span() {
        let mut p = VerticalRenderPlanV1 {
            contract_version: "v1".into(),
            id: "p1".into(),
            content_project_id: "c1".into(),
            recipe_id: "r1".into(),
            candidate_id: "a1".into(),
            source_media_path: r"C:\a.mp4".into(),
            source_start_s: 5.0,
            source_end_s: 1.0,
            framing: ClipFraming::default(),
            subtitles: SubtitleBurnPlanV1 {
                enabled: false,
                preset_id: SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1.into(),
                cues: vec![],
                source_transcript_artifact_id: None,
            },
            output_spec: OutputSpecV1::default(),
            created_at: "t".into(),
            created_by: "local_operator".into(),
        };
        assert!(p.validate().is_err());
        p.source_end_s = 10.0;
        p.output_spec.width = 720;
        assert!(p.validate().is_err());
        p.output_spec.width = 1080;
        assert!(p.validate().is_ok());
    }
}
