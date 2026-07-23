use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::visual::{LicenseStatus, MediaAsset};
use crate::models::visual_intel::{AssetProvenance, QaStatus, VisualNeed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionSource {
    ManualImport,
    FolderImport,
    DailyGeneration,
    BrollGeneration,
    StoryBuilderGeneration,
    RemoteSync,
}

impl IngestionSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ManualImport => "manual_import",
            Self::FolderImport => "folder_import",
            Self::DailyGeneration => "daily_generation",
            Self::BrollGeneration => "broll_generation",
            Self::StoryBuilderGeneration => "story_builder_generation",
            Self::RemoteSync => "remote_sync",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssetIngestionRequest {
    pub source_path: PathBuf,
    pub source: IngestionSource,
    pub title: Option<String>,
    pub tags: Vec<String>,
    pub concept_ids: Vec<String>,
    pub concept_terms: Vec<String>,
    pub provenance: AssetProvenance,
    pub license_status: LicenseStatus,
    pub commercial_use: Option<bool>,
    pub qa_status: QaStatus,
    pub technical_score: Option<f64>,
    pub semantic_score: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIngestionResult {
    pub asset_id: String,
    pub asset: MediaAsset,
    pub duplicate: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetQuery {
    #[serde(default)]
    pub terms: Vec<String>,
    #[serde(default)]
    pub required_contexts: Vec<String>,
    #[serde(default)]
    pub forbidden_contexts: Vec<String>,
    #[serde(default)]
    pub hard_exclusions: Vec<String>,
    #[serde(default)]
    pub desired_aspect: Option<String>,
    #[serde(default)]
    pub used_asset_ids: Vec<String>,
    #[serde(default)]
    pub min_score: Option<f64>,
}

impl From<&VisualNeed> for AssetQuery {
    fn from(need: &VisualNeed) -> Self {
        Self {
            terms: need.terms.clone(),
            required_contexts: need.required_contexts.clone(),
            forbidden_contexts: need.forbidden_contexts.clone(),
            hard_exclusions: need.hard_exclusions.clone(),
            desired_aspect: Some(need.desired_aspect.clone()),
            used_asset_ids: Vec::new(),
            min_score: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMatch {
    pub asset_id: String,
    pub title: String,
    pub score: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSelection {
    pub consumer: String,
    pub consumer_ref: String,
    pub asset_id: String,
    pub match_score: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct LibraryGenerationRequest {
    pub need: VisualNeed,
    pub origin: String,
    pub idempotency_key: String,
    pub opportunistic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUsage {
    pub asset_id: String,
    pub consumer: String,
    pub consumer_ref: String,
    pub run_id: Option<String>,
    pub output_start: Option<f64>,
    pub output_end: Option<f64>,
}
