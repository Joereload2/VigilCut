//! Visual enrichment domain — separate from EDL cut decisions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::Span;
use super::visual_intel::{AssetProvenance, QaStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssetStatus {
    #[default]
    Active,
    Archived,
    Blocked,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LicenseStatus {
    Owned,
    Licensed,
    PublicDomain,
    AttributionRequired,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: String,
    pub kind: String,
    pub managed_path: String,
    pub thumbnail_path: Option<String>,
    pub sha256: String,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub concepts: Vec<String>,
    pub category: Option<String>,
    pub width: u32,
    pub height: u32,
    pub orientation: String,
    pub mime_type: String,
    pub file_size: u64,
    pub license_status: LicenseStatus,
    pub source: Option<String>,
    pub attribution: Option<String>,
    pub times_used: u32,
    pub last_used_at: Option<String>,
    pub allow_same_video_repeat: bool,
    pub minimum_videos_before_reuse: u32,
    pub quality_score: Option<f64>,
    pub status: AssetStatus,
    pub original_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    // ── Intelligent library extensions (defaults for legacy rows) ──
    #[serde(default)]
    pub literal_description: Vec<String>,
    #[serde(default)]
    pub meanings: Vec<String>,
    #[serde(default)]
    pub positive_contexts: Vec<String>,
    #[serde(default)]
    pub negative_contexts: Vec<String>,
    #[serde(default)]
    pub hard_exclusions: Vec<String>,
    #[serde(default)]
    pub aspect_ratio: Option<String>,
    #[serde(default)]
    pub safe_area: Option<String>,
    #[serde(default)]
    pub perceptual_hash: Option<String>,
    #[serde(default)]
    pub qa_status: QaStatus,
    #[serde(default)]
    pub technical_score: Option<f64>,
    #[serde(default)]
    pub semantic_score: Option<f64>,
    #[serde(default)]
    pub provenance: Option<AssetProvenance>,
    #[serde(default)]
    pub commercial_use: Option<bool>,
}

impl MediaAsset {
    pub fn status_label(&self) -> &'static str {
        match self.status {
            AssetStatus::Active => "active",
            AssetStatus::Archived => "archived",
            AssetStatus::Blocked => "blocked",
            AssetStatus::Missing => "missing",
            AssetStatus::Invalid => "invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticKind {
    Keyword,
    Phrase,
    Entity,
    Person,
    Place,
    Organization,
    Object,
    Action,
    Concept,
    Topic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticEvent {
    pub id: String,
    pub run_id: String,
    pub kind: SemanticKind,
    pub source_span: Span,
    pub output_span: Option<Span>,
    pub label: String,
    pub terms: Vec<String>,
    /// Heuristic operational score 0..1
    pub score: f64,
    pub transcript_segment_ids: Vec<String>,
    pub method: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionStatus {
    #[default]
    Suggested,
    Accepted,
    Rejected,
    Replaced,
    Conflict,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSuggestion {
    pub id: String,
    pub semantic_event_id: String,
    pub asset_id: String,
    pub source_span: Span,
    pub output_span: Span,
    pub match_reasons: Vec<String>,
    pub match_score: f64,
    pub alternatives: Vec<String>,
    pub status: SuggestionStatus,
    pub asset_title: Option<String>,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PlacementMode {
    /// Fullscreen / completa — covers the frame (audio from host video)
    #[default]
    #[serde(
        alias = "completa",
        alias = "complete",
        alias = "full",
        alias = "fullscreen"
    )]
    Fullframe,
    /// Overlay partial — picture-in-picture
    #[serde(
        alias = "parcial",
        alias = "pip",
        alias = "picture-in-picture",
        alias = "overlay"
    )]
    PictureInPicture,
    /// Overlay lower-third style band
    #[serde(alias = "flotante", alias = "float", alias = "lower-third", alias = "lower_third")]
    LowerThird,
}

impl PlacementMode {
    pub fn from_user(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").as_str() {
            "completa" | "fullframe" | "full" | "complete" | "fullscreen" => Self::Fullframe,
            "parcial" | "pip" | "picture_in_picture" | "overlay" => Self::PictureInPicture,
            "flotante" | "float" | "lower_third" => Self::LowerThird,
            _ => Self::Fullframe,
        }
    }

    pub fn is_overlay(self) -> bool {
        matches!(self, Self::PictureInPicture | Self::LowerThird)
    }
}

/// Human + AI supervision state for a composition placement (not Segment legacy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    /// AI proposed or newly placed — needs a glance only if low confidence
    #[default]
    Pending,
    /// Human accepted (or high-confidence auto)
    Approved,
    /// Conflict / low confidence — exception queue
    Conflict,
    Rejected,
}

/// Spatial region the B-roll must not cover (or safe-area reserve).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialZone {
    pub id: String,
    /// face | subtitle | text | logo | product | manual | safe_area
    pub kind: String,
    /// Normalized rect 0..1 relative to output frame
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    /// Optional active window on **output** timeline
    #[serde(default)]
    pub output_start: Option<f64>,
    #[serde(default)]
    pub output_end: Option<f64>,
    #[serde(default)]
    pub label: Option<String>,
    /// info | warn | error
    #[serde(default = "default_severity")]
    pub severity: String,
}

fn default_severity() -> String {
    "warn".into()
}

impl SpatialZone {
    pub fn new(kind: impl Into<String>, x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind: kind.into(),
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            w: w.clamp(0.02, 1.0),
            h: h.clamp(0.02, 1.0),
            output_start: None,
            output_end: None,
            label: None,
            severity: "warn".into(),
        }
    }

    pub fn active_at(&self, t: f64) -> bool {
        match (self.output_start, self.output_end) {
            (Some(s), Some(e)) => t >= s && t < e,
            (Some(s), None) => t >= s,
            (None, Some(e)) => t < e,
            (None, None) => true,
        }
    }

    pub fn rect_overlap_score(&self, bx: f64, by: f64, bw: f64, bh: f64) -> f64 {
        // Axis-aligned overlap area / zone area
        let ax1 = self.x;
        let ay1 = self.y;
        let ax2 = self.x + self.w;
        let ay2 = self.y + self.h;
        let bx1 = (bx - bw * 0.5).clamp(0.0, 1.0);
        let by1 = (by - bh * 0.5).clamp(0.0, 1.0);
        let bx2 = (bx + bw * 0.5).clamp(0.0, 1.0);
        let by2 = (by + bh * 0.5).clamp(0.0, 1.0);
        let ix1 = ax1.max(bx1);
        let iy1 = ay1.max(by1);
        let ix2 = ax2.min(bx2);
        let iy2 = ay2.min(by2);
        let iw = (ix2 - ix1).max(0.0);
        let ih = (iy2 - iy1).max(0.0);
        let inter = iw * ih;
        let za = (self.w * self.h).max(1e-6);
        inter / za
    }
}

/// Exception-style issue for B-roll supervision (human only sees these when flagged).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionIssue {
    pub id: String,
    pub placement_id: String,
    /// semantic_low | timing_unclear | face_covered | subtitle_covered |
    /// safe_area | aspect | overlap | past_idea | protected_time
    pub kind: String,
    /// info | warn | error
    pub severity: String,
    pub message: String,
    #[serde(default)]
    pub suggested_x: Option<f64>,
    #[serde(default)]
    pub suggested_y: Option<f64>,
    #[serde(default)]
    pub suggested_w: Option<f64>,
}

/// Normalized layout for a placement on the output frame (fractions 0..1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementLayout {
    /// Horizontal anchor 0=left … 1=right (center of image for PIP/float).
    pub x: f64,
    /// Vertical anchor 0=top … 1=bottom.
    pub y: f64,
    /// Width as fraction of frame width (PIP/float). Fullframe ignores.
    pub w: f64,
    /// Height as fraction of frame height (optional; keep aspect if 0).
    pub h: f64,
    /// Opacity 0..1
    pub opacity: f64,
}

impl Default for PlacementLayout {
    fn default() -> Self {
        Self {
            x: 0.5,
            y: 0.5,
            w: 0.35,
            h: 0.0,
            opacity: 0.92,
        }
    }
}

impl PlacementLayout {
    pub fn for_mode(mode: PlacementMode) -> Self {
        match mode {
            PlacementMode::Fullframe => Self {
                x: 0.5,
                y: 0.5,
                w: 1.0,
                h: 1.0,
                opacity: 0.92,
            },
            PlacementMode::PictureInPicture => Self {
                x: 0.82,
                y: 0.18,
                w: 0.28,
                h: 0.0,
                opacity: 0.95,
            },
            PlacementMode::LowerThird => Self {
                x: 0.5,
                y: 0.82,
                w: 0.55,
                h: 0.0,
                opacity: 0.9,
            },
        }
    }

    pub fn clamp(mut self) -> Self {
        self.x = self.x.clamp(0.0, 1.0);
        self.y = self.y.clamp(0.0, 1.0);
        self.w = self.w.clamp(0.05, 1.0);
        self.h = self.h.clamp(0.0, 1.0);
        self.opacity = self.opacity.clamp(0.05, 1.0);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualPlacement {
    pub id: String,
    pub asset_id: String,
    pub output_start: f64,
    pub output_end: f64,
    pub mode: PlacementMode,
    /// contain | cover | crop
    pub fit: String,
    pub transition_in: String,
    pub transition_out: String,
    pub status: String,
    pub provenance: String,
    pub suggestion_id: Option<String>,
    /// Layout on frame (optional for older plans — default full center).
    #[serde(default)]
    pub layout: PlacementLayout,
    /// Optional human label
    #[serde(default)]
    pub label: Option<String>,
    // ── Composition supervision (Events→Policy parallel; not Segment legacy) ──
    /// Link to SemanticEvent / phrase when AI-proposed
    #[serde(default)]
    pub semantic_event_id: Option<String>,
    /// Related word/phrase text for inspector highlight
    #[serde(default)]
    pub related_text: Option<String>,
    /// Source-timeline span when mapped from transcript
    #[serde(default)]
    pub source_start: Option<f64>,
    #[serde(default)]
    pub source_end: Option<f64>,
    /// 0..1 AI / heuristic confidence
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub review_status: ReviewStatus,
    /// If true, re-analysis must not overwrite timing/layout
    #[serde(default)]
    pub manual_override: bool,
    /// Zone kinds this placement should avoid
    #[serde(default)]
    pub avoid_zones: Vec<String>,
    /// AI-suggested layout to restore
    #[serde(default)]
    pub suggested_layout: Option<PlacementLayout>,
    #[serde(default)]
    pub suggested_mode: Option<PlacementMode>,
}

fn default_confidence() -> f64 {
    0.75
}

impl VisualPlacement {
    pub fn from_accepted(s: &VisualSuggestion) -> Self {
        let mode = PlacementMode::Fullframe;
        let layout = PlacementLayout::for_mode(mode);
        Self {
            id: Uuid::new_v4().to_string(),
            asset_id: s.asset_id.clone(),
            output_start: s.output_span.start,
            output_end: s.output_span.end,
            mode,
            fit: "cover".into(),
            transition_in: "fade".into(),
            transition_out: "fade".into(),
            status: "active".into(),
            provenance: "human_accepted".into(),
            suggestion_id: Some(s.id.clone()),
            layout: layout.clone(),
            label: s.asset_title.clone(),
            semantic_event_id: Some(s.semantic_event_id.clone()),
            related_text: s.match_reasons.first().cloned(),
            source_start: Some(s.source_span.start),
            source_end: Some(s.source_span.end),
            confidence: s.match_score.clamp(0.0, 1.0),
            review_status: if s.match_score >= 0.72 {
                ReviewStatus::Approved
            } else {
                ReviewStatus::Pending
            },
            manual_override: false,
            avoid_zones: vec![
                "face".into(),
                "subtitle".into(),
                "text".into(),
                "logo".into(),
            ],
            suggested_layout: Some(layout),
            suggested_mode: Some(mode),
        }
    }

    pub fn manual(
        asset_id: impl Into<String>,
        output_start: f64,
        output_end: f64,
        mode: PlacementMode,
        layout: PlacementLayout,
        fit: impl Into<String>,
        label: Option<String>,
    ) -> Self {
        let start = output_start.max(0.0);
        let end = output_end.max(start + 0.25);
        let layout = layout.clamp();
        Self {
            id: Uuid::new_v4().to_string(),
            asset_id: asset_id.into(),
            output_start: start,
            output_end: end,
            mode,
            fit: fit.into(),
            transition_in: "fade".into(),
            transition_out: "fade".into(),
            status: "active".into(),
            provenance: "manual".into(),
            suggestion_id: None,
            layout: layout.clone(),
            label,
            semantic_event_id: None,
            related_text: None,
            source_start: None,
            source_end: None,
            confidence: 1.0,
            review_status: ReviewStatus::Approved,
            manual_override: true,
            avoid_zones: vec!["face".into(), "subtitle".into()],
            suggested_layout: Some(layout),
            suggested_mode: Some(mode),
        }
    }

    pub fn duration(&self) -> f64 {
        (self.output_end - self.output_start).max(0.0)
    }

    /// Center + size (cx, cy, w, h) — same contract as `pipeline::visual::layout`.
    pub fn frame_rect(&self) -> (f64, f64, f64, f64) {
        match self.mode {
            PlacementMode::Fullframe => (0.5, 0.5, 1.0, 1.0),
            PlacementMode::PictureInPicture => {
                let w = self.layout.w.clamp(0.08, 1.0);
                let h = if self.layout.h > 0.01 {
                    self.layout.h.clamp(0.05, 1.0)
                } else {
                    // Match layout.rs: 16:9 on 1280x720
                    (w * 1280.0 * 9.0 / 16.0 / 720.0).clamp(0.05, 1.0)
                };
                (
                    self.layout.x.clamp(0.0, 1.0),
                    self.layout.y.clamp(0.0, 1.0),
                    w,
                    h,
                )
            }
            PlacementMode::LowerThird => {
                let w = self.layout.w.clamp(0.15, 1.0);
                let h = if self.layout.h > 0.01 {
                    self.layout.h.clamp(0.05, 0.5)
                } else {
                    0.22
                };
                (0.5, self.layout.y.clamp(0.0, 1.0), w, h)
            }
        }
    }
}

/// Interval on the **output** timeline where automatic/manual images must not appear.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtectedRange {
    pub id: String,
    pub output_start: f64,
    pub output_end: f64,
    pub reason: String,
    pub created_at: String,
}

impl ProtectedRange {
    pub fn new(output_start: f64, output_end: f64, reason: impl Into<String>) -> Self {
        let start = output_start.max(0.0);
        let end = output_end.max(start + 0.1);
        Self {
            id: Uuid::new_v4().to_string(),
            output_start: start,
            output_end: end,
            reason: reason.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn overlaps(&self, start: f64, end: f64) -> bool {
        start < self.output_end && end > self.output_start
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualPlan {
    pub id: String,
    pub run_id: String,
    pub media_path: String,
    pub edl_fingerprint: String,
    pub placements: Vec<VisualPlacement>,
    /// Temporal zones without B-roll overlays (output timeline)
    #[serde(default)]
    pub protected_ranges: Vec<ProtectedRange>,
    /// Spatial zones on the frame (faces, subtitles, safe areas…)
    #[serde(default)]
    pub spatial_zones: Vec<SpatialZone>,
    /// Latest evaluated composition issues (exception surface)
    #[serde(default)]
    pub issues: Vec<CompositionIssue>,
    pub warnings: Vec<String>,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl VisualPlan {
    pub fn new(run_id: impl Into<String>, media_path: impl Into<String>, edl_fp: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        // Spatial zones are added when detection/user defines them — not fake
        // face boxes on every plan (they confuse supervision on split layouts).
        Self {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            media_path: media_path.into(),
            edl_fingerprint: edl_fp.into(),
            placements: Vec::new(),
            protected_ranges: Vec::new(),
            spatial_zones: Vec::new(),
            issues: Vec::new(),
            warnings: Vec::new(),
            version: 1,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
        self.version = self.version.saturating_add(1);
    }

    pub fn is_protected(&self, start: f64, end: f64) -> bool {
        self.protected_ranges.iter().any(|r| r.overlaps(start, end))
    }
}

/// Fingerprint EDL keep ranges for invalidation.
pub fn edl_fingerprint(keep: &[(f64, f64)]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for (s, e) in keep {
        h.update(format!("{s:.3}-{e:.3};").as_bytes());
    }
    hex::encode(h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placement_mode_from_user_labels() {
        assert_eq!(PlacementMode::from_user("completa"), PlacementMode::Fullframe);
        assert_eq!(PlacementMode::from_user("parcial"), PlacementMode::PictureInPicture);
        assert_eq!(PlacementMode::from_user("flotante"), PlacementMode::LowerThird);
    }

    #[test]
    fn protected_range_overlap() {
        let pr = ProtectedRange::new(10.0, 20.0, "test");
        assert!(pr.overlaps(15.0, 18.0));
        assert!(!pr.overlaps(20.0, 25.0));
        assert!(pr.overlaps(5.0, 10.5));
    }

    #[test]
    fn manual_placement_defaults() {
        let p = VisualPlacement::manual(
            "a1",
            1.0,
            5.0,
            PlacementMode::PictureInPicture,
            PlacementLayout::for_mode(PlacementMode::PictureInPicture),
            "cover",
            Some("x".into()),
        );
        assert_eq!(p.provenance, "manual");
        assert!(p.layout.w < 1.0);
        assert!(p.output_end > p.output_start);
    }
}
