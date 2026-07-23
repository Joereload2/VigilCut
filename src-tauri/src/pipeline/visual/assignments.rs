//! Consumer-owned link between a B-roll need and a library asset.

use serde::{Deserialize, Serialize};

use crate::error::AppResult;
use crate::models::visual_intel::{NeedCoverage, VisualNeed};
use crate::pipeline::visual::matching_adapter::get_asset;
use crate::pipeline::visual::needs::{get_need, update_need};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneAssetAssignment {
    pub need_id: String,
    pub media_asset_id: String,
    pub consumer: String,
    pub match_score: Option<f64>,
    pub created_at: String,
}

pub fn assign_need(need_id: &str, asset_id: &str) -> AppResult<(VisualNeed, SceneAssetAssignment)> {
    let asset = get_asset(asset_id)?;
    let mut need = get_need(need_id)?;
    need.matched_asset_id = Some(asset.id.clone());
    need.coverage = NeedCoverage::Covered;
    need.match_reasons = vec!["user_selected".into()];
    need.updated_at = chrono::Utc::now().to_rfc3339();
    update_need(&need)?;
    let assignment = SceneAssetAssignment {
        need_id: need.id.clone(),
        media_asset_id: asset.id,
        consumer: "broll".into(),
        match_score: need.match_score,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    Ok((need, assignment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assignment_contract_uses_media_asset_id() {
        let value = serde_json::to_value(SceneAssetAssignment {
            need_id: "need".into(),
            media_asset_id: "asset".into(),
            consumer: "broll".into(),
            match_score: Some(0.8),
            created_at: "now".into(),
        })
        .unwrap();
        assert_eq!(value["mediaAssetId"], "asset");
        assert!(value.get("candidateId").is_none());
    }
}
