use serde::{Deserialize, Serialize};

/// Immutable recipe row — body is opaque JSON validated at write boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductionRecipeRecord {
    pub id: String,
    pub content_project_id: String,
    pub label: String,
    /// Full ProductionRecipeV1 JSON (includes contractVersion).
    pub recipe_json: String,
    pub supersedes_recipe_id: Option<String>,
    pub created_at: String,
}
