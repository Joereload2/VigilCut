use crate::error::AppResult;
use crate::visual_library::application::sync_service;

#[tauri::command]
pub fn library_sync_status() -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(sync_service::status()?)?)
}

#[tauri::command]
pub async fn library_sync_health_check() -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(sync_service::health_check().await?)?)
}

#[tauri::command]
pub fn library_sync_enqueue_asset(asset_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::json!({
        "queueId": sync_service::enqueue_asset(&asset_id)?,
    }))
}

#[tauri::command]
pub async fn library_sync_run_once() -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(sync_service::process_once().await?)?)
}
