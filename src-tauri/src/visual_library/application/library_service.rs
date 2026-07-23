use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::models::visual::MediaAsset;
use crate::models::visual_intel::{MatchCandidate, VisualNeed};
use crate::pipeline::visual::generation::worker::queue_generation_with_key;
use crate::pipeline::visual::intelligent_match::{match_need, MatchOptions};
use crate::pipeline::visual::library::{
    get_asset_by_id, import_image_detailed, open_db, record_usage, ImportOutcome,
};
use crate::pipeline::visual::needs::save_needs;
use crate::visual_library::domain::contracts::{
    AssetIngestionRequest, AssetIngestionResult, AssetMatch, AssetQuery, AssetUsage,
    LibraryGenerationRequest,
};

pub trait VisualLibrary {
    fn search(&self, query: &AssetQuery) -> AppResult<Vec<AssetMatch>>;
    fn get_asset(&self, asset_id: &str) -> AppResult<MediaAsset>;
    fn request_generation(&self, request: LibraryGenerationRequest) -> AppResult<Option<String>>;
    fn record_usage(&self, usage: AssetUsage) -> AppResult<()>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalVisualLibrary;

pub type LibraryService = LocalVisualLibrary;

impl LocalVisualLibrary {
    pub fn new() -> Self {
        Self
    }

    pub fn ingest_asset(&self, request: AssetIngestionRequest) -> AppResult<AssetIngestionResult> {
        let outcome = import_image_detailed(
            &request.source_path,
            request.title,
            request.tags,
            request.concept_terms,
            request.license_status,
        )?;
        let (mut asset, duplicate) = match outcome {
            ImportOutcome::New(asset) => (asset, false),
            ImportOutcome::Duplicate(asset) => (asset, true),
        };

        if !duplicate {
            let conn = open_db()?;
            let provenance = serde_json::to_string(&request.provenance)
                .map_err(|e| AppError::Message(e.to_string()))?;
            conn.execute(
                "UPDATE media_assets
                 SET provenance_json=?1, source=?2, qa_status=?3,
                     technical_score=?4, semantic_score=?5, commercial_use=?6,
                     updated_at=?7
                 WHERE id=?8",
                params![
                    provenance,
                    request.source.as_str(),
                    request.qa_status.as_str(),
                    request.technical_score,
                    request.semantic_score,
                    request.commercial_use.map(i64::from),
                    chrono::Utc::now().to_rfc3339(),
                    asset.id,
                ],
            )
            .map_err(|e| AppError::Message(e.to_string()))?;
            for concept_id in &request.concept_ids {
                conn.execute(
                    "INSERT OR IGNORE INTO asset_concepts(asset_id,concept_id,weight)
                     VALUES(?1,?2,1.0)",
                    params![asset.id, concept_id],
                )
                .map_err(|e| AppError::Message(e.to_string()))?;
            }
            asset.provenance = Some(request.provenance);
            asset.source = Some(request.source.as_str().into());
            asset.qa_status = request.qa_status;
            asset.technical_score = request.technical_score;
            asset.semantic_score = request.semantic_score;
            asset.commercial_use = request.commercial_use;
        }

        Ok(AssetIngestionResult {
            asset_id: asset.id.clone(),
            asset,
            duplicate,
        })
    }

    pub fn search_for_need(&self, need: &VisualNeed) -> AppResult<Vec<MatchCandidate>> {
        let query = AssetQuery::from(need);
        let opts = MatchOptions {
            min_score: query.min_score.unwrap_or_default(),
            prefer_aspect: query.desired_aspect,
            used_in_project: query.used_asset_ids,
        };
        Ok(match_need(need, &opts))
    }
}

impl VisualLibrary for LocalVisualLibrary {
    fn search(&self, query: &AssetQuery) -> AppResult<Vec<AssetMatch>> {
        let synthetic = VisualNeed {
            terms: query.terms.clone(),
            required_contexts: query.required_contexts.clone(),
            forbidden_contexts: query.forbidden_contexts.clone(),
            hard_exclusions: query.hard_exclusions.clone(),
            desired_aspect: query
                .desired_aspect
                .clone()
                .unwrap_or_else(|| "16:9".into()),
            ..VisualNeed::from_label("library_query", query.terms.join(" "))
        };
        let opts = MatchOptions {
            min_score: query.min_score.unwrap_or_default(),
            prefer_aspect: query.desired_aspect.clone(),
            used_in_project: query.used_asset_ids.clone(),
        };
        Ok(match_need(&synthetic, &opts)
            .into_iter()
            .map(|item| AssetMatch {
                asset_id: item.asset_id,
                title: item.asset_title,
                score: item.score,
                reasons: item.reasons,
            })
            .collect())
    }

    fn get_asset(&self, asset_id: &str) -> AppResult<MediaAsset> {
        get_asset_by_id(asset_id)
    }

    fn request_generation(
        &self,
        mut request: LibraryGenerationRequest,
    ) -> AppResult<Option<String>> {
        save_needs(std::slice::from_ref(&request.need))?;
        queue_generation_with_key(
            &mut request.need,
            request.opportunistic,
            &request.idempotency_key,
            &request.origin,
        )
    }

    fn record_usage(&self, usage: AssetUsage) -> AppResult<()> {
        record_usage(
            &usage.asset_id,
            &usage.consumer_ref,
            usage.run_id.as_deref(),
            usage.output_start.unwrap_or_default(),
            usage.output_end.unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual::LicenseStatus;
    use crate::models::visual_intel::{AssetProvenance, QaStatus};
    use crate::pipeline::visual::library::{lock_library_for_test, set_library_root_override};

    #[test]
    fn ingestion_is_idempotent_by_sha_and_story_contract_can_query() {
        let _lock = lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-domain-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let input = dir.join("source.png");
        image::RgbImage::from_pixel(80, 60, image::Rgb([20, 80, 120]))
            .save(&input)
            .unwrap();
        let service = LibraryService::new();
        let request = || AssetIngestionRequest {
            source_path: input.clone(),
            source: crate::visual_library::IngestionSource::ManualImport,
            title: Some("Ciudad".into()),
            tags: vec!["ciudad".into()],
            concept_ids: vec![],
            concept_terms: vec!["ciudad".into()],
            provenance: AssetProvenance {
                source: "manual_import".into(),
                ..Default::default()
            },
            license_status: LicenseStatus::Owned,
            commercial_use: Some(true),
            qa_status: QaStatus::Approved,
            technical_score: Some(1.0),
            semantic_score: None,
        };
        let first = service.ingest_asset(request()).unwrap();
        let second = service.ingest_asset(request()).unwrap();
        assert_eq!(first.asset_id, second.asset_id);
        assert!(!first.duplicate);
        assert!(second.duplicate);
        let matches = service
            .search(&AssetQuery {
                terms: vec!["ciudad".into()],
                ..Default::default()
            })
            .unwrap();
        assert_eq!(matches.first().map(|m| &m.asset_id), Some(&first.asset_id));
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
