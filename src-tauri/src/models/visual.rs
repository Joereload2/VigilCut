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
    #[default]
    Fullframe,
    PictureInPicture,
    LowerThird,
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
        }
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
            warnings: Vec::new(),
            version: 1,
            created_at: now.clone(),
            updated_at: now,
        }
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
