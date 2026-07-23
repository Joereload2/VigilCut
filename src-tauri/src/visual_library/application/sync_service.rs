use std::future::Future;

use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::visual::MediaAsset;
use crate::pipeline::visual::library::{get_asset_by_id, open_db};
use crate::visual_library::infrastructure::storage::supabase_storage::{
    SupabaseConfig, SupabaseStorage,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub enabled: bool,
    pub connected: bool,
    pub mode: &'static str,
    pub pending: u32,
    pub failed: u32,
    pub succeeded: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct SyncItem {
    id: String,
    entity_id: String,
    attempt: u32,
}

pub fn enqueue_asset(asset_id: &str) -> AppResult<String> {
    let asset = get_asset_by_id(asset_id)?;
    if asset.qa_status.as_str() != "approved" {
        return Err(AppError::Invalid(
            "Solo se sincronizan assets aprobados".into(),
        ));
    }
    let conn = open_db()?;
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        r#"INSERT INTO sync_queue(
             id,entity_type,entity_id,operation,status,attempt,created_at,updated_at
           ) VALUES(?1,'media_asset',?2,'push','pending',0,?3,?3)
           ON CONFLICT(entity_type,entity_id,operation) DO UPDATE SET
             status=CASE WHEN sync_queue.status='succeeded' THEN 'succeeded' ELSE 'pending' END,
             last_error=NULL,
             next_attempt_at=NULL,
             updated_at=excluded.updated_at"#,
        params![id, asset_id, now],
    )
    .map_err(|error| AppError::Message(error.to_string()))?;
    let stable_id = conn
        .query_row(
            "SELECT id FROM sync_queue WHERE entity_type='media_asset' AND entity_id=?1 AND operation='push'",
            params![asset_id],
            |row| row.get(0),
        )
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(stable_id)
}

pub fn status() -> AppResult<SyncStatus> {
    let conn = open_db()?;
    let count = |state: &str| -> u32 {
        conn.query_row(
            "SELECT COUNT(*) FROM sync_queue WHERE status=?1",
            params![state],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as u32
    };
    let last_error = conn
        .query_row(
            "SELECT last_error FROM sync_queue WHERE last_error IS NOT NULL ORDER BY updated_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None);
    let enabled = std::env::var("VIGILCUT_SUPABASE_SYNC")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    Ok(SyncStatus {
        enabled,
        connected: false,
        mode: if enabled {
            "configured_unverified"
        } else {
            "local_only"
        },
        pending: count("pending"),
        failed: count("failed"),
        succeeded: count("succeeded"),
        last_error,
    })
}

pub async fn health_check() -> AppResult<SyncStatus> {
    let Some(config) = SupabaseConfig::from_env()? else {
        return status();
    };
    let storage = SupabaseStorage::new(config)?;
    storage.health_check().await?;
    let mut value = status()?;
    value.connected = true;
    value.mode = "connected";
    Ok(value)
}

pub async fn process_once() -> AppResult<SyncStatus> {
    let Some(config) = SupabaseConfig::from_env()? else {
        return status();
    };
    let storage = SupabaseStorage::new(config)?;
    process_once_with(|asset| async move { storage.push_asset(&asset).await }).await?;
    status()
}

async fn process_once_with<F, Fut>(push: F) -> AppResult<bool>
where
    F: FnOnce(MediaAsset) -> Fut,
    Fut: Future<Output = AppResult<()>>,
{
    let Some(item) = claim_next()? else {
        return Ok(false);
    };
    let asset = get_asset_by_id(&item.entity_id)?;
    match push(asset).await {
        Ok(()) => {
            finish(&item.id, "succeeded", None, None)?;
            Ok(true)
        }
        Err(error) => {
            let delay = 2_i64.pow(item.attempt.min(6));
            let retry_at = (chrono::Utc::now() + chrono::Duration::minutes(delay)).to_rfc3339();
            finish(
                &item.id,
                "failed",
                Some(&error.to_string()),
                Some(&retry_at),
            )?;
            Err(error)
        }
    }
}

fn claim_next() -> AppResult<Option<SyncItem>> {
    let mut conn = open_db()?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|error| AppError::Message(error.to_string()))?;
    let item = tx
        .query_row(
            r#"SELECT id,entity_id,attempt FROM sync_queue
               WHERE status IN ('pending','failed')
                 AND (next_attempt_at IS NULL OR next_attempt_at<=?1)
               ORDER BY created_at LIMIT 1"#,
            params![chrono::Utc::now().to_rfc3339()],
            |row| {
                Ok(SyncItem {
                    id: row.get(0)?,
                    entity_id: row.get(1)?,
                    attempt: row.get::<_, i64>(2)? as u32 + 1,
                })
            },
        )
        .optional()
        .map_err(|error| AppError::Message(error.to_string()))?;
    if let Some(item) = &item {
        tx.execute(
            "UPDATE sync_queue SET status='syncing',attempt=?1,updated_at=?2 WHERE id=?3",
            params![
                item.attempt as i64,
                chrono::Utc::now().to_rfc3339(),
                item.id
            ],
        )
        .map_err(|error| AppError::Message(error.to_string()))?;
    }
    tx.commit()
        .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(item)
}

fn finish(
    id: &str,
    state: &str,
    error: Option<&str>,
    next_attempt_at: Option<&str>,
) -> AppResult<()> {
    let conn = open_db()?;
    conn.execute(
        "UPDATE sync_queue SET status=?1,last_error=?2,next_attempt_at=?3,updated_at=?4 WHERE id=?5",
        params![
            state,
            error,
            next_attempt_at,
            chrono::Utc::now().to_rfc3339(),
            id
        ],
    )
    .map_err(|error| AppError::Message(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual::LicenseStatus;
    use crate::pipeline::visual::library::{lock_library_for_test, set_library_root_override};
    use crate::visual_library::{AssetIngestionRequest, IngestionSource, LibraryService};

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn queue_resumes_and_deduplicates_without_remote_dependency() {
        let _lock = lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-sync-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let source = dir.join("sync.png");
        image::RgbImage::from_pixel(64, 64, image::Rgb([10, 20, 30]))
            .save(&source)
            .unwrap();
        let asset = LibraryService::new()
            .ingest_asset(AssetIngestionRequest {
                source_path: source,
                source: IngestionSource::ManualImport,
                title: Some("Sync fixture".into()),
                tags: vec![],
                concept_ids: vec![],
                concept_terms: vec![],
                provenance: Default::default(),
                license_status: LicenseStatus::Owned,
                commercial_use: Some(true),
                qa_status: crate::models::visual_intel::QaStatus::Approved,
                technical_score: Some(1.0),
                semantic_score: None,
            })
            .unwrap()
            .asset;
        let first = enqueue_asset(&asset.id).unwrap();
        let second = enqueue_asset(&asset.id).unwrap();
        assert_eq!(first, second);
        assert_eq!(status().unwrap().pending, 1);
        assert!(process_once_with(|_| async { Ok(()) }).await.unwrap());
        assert_eq!(status().unwrap().succeeded, 1);
        assert!(!process_once_with(|_| async { Ok(()) }).await.unwrap());
        assert_eq!(get_asset_by_id(&asset.id).unwrap().sha256, asset.sha256);
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
