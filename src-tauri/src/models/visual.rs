//! Visual enrichment domain — separate from EDL cut decisions.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::Span;

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
    /// Completa — full frame overlay
    #[default]
    #[serde(alias = "completa", alias = "complete", alias = "full")]
    Fullframe,
    /// Parcial — picture-in-picture
    #[serde(alias = "parcial", alias = "pip", alias = "picture-in-picture")]
    PictureInPicture,
    /// Flotante — lower-third style band
    #[serde(alias = "flotante", alias = "float", alias = "lower-third", alias = "lower_third")]
    LowerThird,
}

impl PlacementMode {
    pub fn from_user(s: &str) -> Self {
        match s.to_lowercase().replace('-', "_").as_str() {
            "completa" | "fullframe" | "full" | "complete" => Self::Fullframe,
            "parcial" | "pip" | "picture_in_picture" => Self::PictureInPicture,
            "flotante" | "float" | "lower_third" => Self::LowerThird,
            _ => Self::Fullframe,
        }
    }
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
}

impl VisualPlacement {
    pub fn from_accepted(s: &VisualSuggestion) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            asset_id: s.asset_id.clone(),
            output_start: s.output_span.start,
            output_end: s.output_span.end,
            mode: PlacementMode::Fullframe,
            fit: "cover".into(),
            transition_in: "fade".into(),
            transition_out: "fade".into(),
            status: "active".into(),
            provenance: "human_accepted".into(),
            suggestion_id: Some(s.id.clone()),
            layout: PlacementLayout::for_mode(PlacementMode::Fullframe),
            label: s.asset_title.clone(),
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
            layout: layout.clamp(),
            label,
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
    /// Zones without B-roll overlays
    #[serde(default)]
    pub protected_ranges: Vec<ProtectedRange>,
    pub warnings: Vec<String>,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl VisualPlan {
    pub fn new(run_id: impl Into<String>, media_path: impl Into<String>, edl_fp: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            run_id: run_id.into(),
            media_path: media_path.into(),
            edl_fingerprint: edl_fp.into(),
            placements: Vec::new(),
            protected_ranges: Vec::new(),
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
