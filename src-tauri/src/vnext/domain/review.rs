use serde::{Deserialize, Serialize};

/// Append-only human decision (ReviewDecisionV1 domain row).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewDecisionRecord {
    pub id: String,
    pub content_project_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub decision: String,
    pub reason: Option<String>,
    pub payload_json: String,
    pub actor: String,
    pub related_job_id: Option<String>,
    pub created_at: String,
}

impl ReviewDecisionRecord {
    pub const TARGET_SHORT_CANDIDATE: &'static str = "short_candidate";
    pub const ACTOR_LOCAL: &'static str = "local_operator";
}
