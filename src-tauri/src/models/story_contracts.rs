//! Future "Create video from text" contracts.
//! Not a full Story Builder — shared types so both flows converge on VisualPlan.

use serde::{Deserialize, Serialize};

use super::visual::VisualPlan;
use super::visual_intel::VisualNeed;

/// Lightweight project shell for future text→video.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryProject {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub source_script: Option<String>,
    #[serde(default)]
    pub scenes: Vec<StoryScene>,
    #[serde(default)]
    pub visual_plan_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryScene {
    pub id: String,
    pub order: u32,
    pub title: String,
    #[serde(default)]
    pub narration: Option<String>,
    /// Desired duration on output timeline
    #[serde(default)]
    pub duration_secs: f64,
    #[serde(default)]
    pub requirements: Vec<VisualNeed>,
    #[serde(default)]
    pub assignments: Vec<SceneAssetAssignment>,
}

/// Assignment of a library asset to a scene (≠ MediaAsset ownership).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneAssetAssignment {
    pub id: String,
    pub scene_id: String,
    pub media_asset_id: String,
    #[serde(default)]
    pub need_id: Option<String>,
    pub output_start: f64,
    pub output_end: f64,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub match_score: Option<f64>,
    #[serde(default)]
    pub provenance: String,
}

/// Both factory paths should produce this pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryVisualBundle {
    pub project: StoryProject,
    pub plan: VisualPlan,
}
