//! Tauri commands for transcript + visual library enrichment.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::State;

use crate::commands::analyze::AnalysisCache;
use crate::error::{AppError, AppResult};
use crate::models::edl::Edl;
use crate::models::visual::{SuggestionStatus, VisualPlan};
use crate::pipeline::visual::{
    import_library_image, invalidate_if_edl_changed, run_visual_enrichment, set_suggestion_status,
    VisualSession,
};
use crate::pipeline::visual::library::{list_assets, update_asset_meta};
use crate::pipeline::visual::render::render_visual_plan;
use crate::models::visual::{AssetStatus, LicenseStatus};

pub type VisualSessionState = Mutex<VisualSession>;

fn edl_from_cache(cache: &AnalysisCache, run_id: Option<&str>, media_path: &str) -> AppResult<Edl> {
    let map = cache
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    if let Some(id) = run_id {
        if let Some(run) = map.get(id) {
            return Ok(run.edl.clone());
        }
    }
    // Latest run for media
    let mut best: Option<Edl> = None;
    for run in map.values() {
        if run.media_path.replace('\\', "/") == media_path.replace('\\', "/") {
            best = Some(run.edl.clone());
        }
    }
    best.ok_or_else(|| {
        AppError::Invalid(
            "No hay EDL de análisis. Abre el video y deja que se analice primero (modo Silencios)."
                .into(),
        )
    })
}

#[tauri::command]
pub async fn visual_run_enrichment(
    media_path: String,
    analysis_run_id: Option<String>,
    transcript_path: Option<String>,
    prefer_whisper: Option<bool>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let edl = edl_from_cache(&analysis, analysis_run_id.as_deref(), &media_path)?;
    let srt = transcript_path.as_ref().map(PathBuf::from);
    run_visual_enrichment(
        PathBuf::from(&media_path).as_path(),
        &edl,
        srt.as_deref(),
        prefer_whisper.unwrap_or(false),
        &visual,
    )
    .await
}

#[tauri::command]
pub fn visual_list_assets(query: Option<String>, limit: Option<usize>) -> AppResult<serde_json::Value> {
    let list = list_assets(query.as_deref(), limit.unwrap_or(100))?;
    Ok(serde_json::to_value(list)?)
}

#[tauri::command]
pub fn visual_import_image(
    path: String,
    title: Option<String>,
    tags: Option<Vec<String>>,
    concepts: Option<Vec<String>>,
) -> AppResult<serde_json::Value> {
    let a = import_library_image(
        PathBuf::from(&path).as_path(),
        title,
        tags.unwrap_or_default(),
        concepts.unwrap_or_default(),
    )?;
    Ok(serde_json::to_value(a)?)
}

#[tauri::command]
pub fn visual_update_asset(
    id: String,
    title: Option<String>,
    tags: Option<Vec<String>>,
    concepts: Option<Vec<String>>,
    license: Option<String>,
    status: Option<String>,
) -> AppResult<serde_json::Value> {
    let lic = license.and_then(|s| match s.as_str() {
        "owned" => Some(LicenseStatus::Owned),
        "licensed" => Some(LicenseStatus::Licensed),
        "public_domain" => Some(LicenseStatus::PublicDomain),
        "attribution_required" => Some(LicenseStatus::AttributionRequired),
        "unknown" => Some(LicenseStatus::Unknown),
        _ => None,
    });
    let st = status.and_then(|s| match s.as_str() {
        "active" => Some(AssetStatus::Active),
        "archived" => Some(AssetStatus::Archived),
        "blocked" => Some(AssetStatus::Blocked),
        _ => None,
    });
    let a = update_asset_meta(&id, title, tags, concepts, lic, st)?;
    Ok(serde_json::to_value(a)?)
}

#[tauri::command]
pub fn visual_set_suggestion_status(
    suggestion_id: String,
    status: String,
    visual: State<'_, VisualSessionState>,
) -> AppResult<VisualPlan> {
    let st = match status.as_str() {
        "accepted" => SuggestionStatus::Accepted,
        "rejected" => SuggestionStatus::Rejected,
        "replaced" => SuggestionStatus::Replaced,
        "suggested" => SuggestionStatus::Suggested,
        _ => return Err(AppError::Invalid(format!("status desconocido: {status}"))),
    };
    set_suggestion_status(&visual, &suggestion_id, st)
}

#[tauri::command]
pub fn visual_get_session(visual: State<'_, VisualSessionState>) -> AppResult<serde_json::Value> {
    let g = visual.lock().map_err(|e| AppError::Message(e.to_string()))?;
    Ok(serde_json::json!({
        "transcript": g.transcript,
        "suggestions": g.suggestions,
        "plan": g.plan,
        "edlFingerprint": g.edl_fp,
    }))
}

#[tauri::command]
pub fn visual_check_edl(
    media_path: String,
    analysis_run_id: Option<String>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<bool> {
    let edl = edl_from_cache(&analysis, analysis_run_id.as_deref(), &media_path)?;
    invalidate_if_edl_changed(&visual, &edl)
}

#[tauri::command]
pub async fn visual_render_plan(
    cut_video_path: String,
    output_path: String,
    media_path: String,
    visual: State<'_, VisualSessionState>,
) -> AppResult<String> {
    let plan = {
        let g = visual.lock().map_err(|e| AppError::Message(e.to_string()))?;
        g.plan
            .clone()
            .ok_or_else(|| AppError::Invalid("No hay VisualPlan. Genera sugerencias y acepta alguna.".into()))?
    };
    let out = render_visual_plan(
        PathBuf::from(&cut_video_path).as_path(),
        &plan,
        PathBuf::from(&output_path).as_path(),
        &media_path,
    )
    .await?;
    Ok(out.to_string_lossy().into_owned())
}
