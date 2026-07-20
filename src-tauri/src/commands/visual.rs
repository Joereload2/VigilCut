//! Tauri commands for transcript + visual library enrichment.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, State};

use crate::commands::analyze::AnalysisCache;
use crate::error::{AppError, AppResult};
use crate::models::edl::Edl;
use crate::models::progress;
use crate::models::visual::{AssetStatus, LicenseStatus, SuggestionStatus, VisualPlan};
use crate::pipeline::visual::library::{
    import_folder, list_assets, list_usage, scan_missing_assets, update_asset_meta,
};
use crate::pipeline::visual::render::render_visual_plan;
use crate::pipeline::visual::{
    attach_image_to_moment, export_session_transcript, import_library_image,
    invalidate_if_edl_changed, load_visual_plan, run_visual_enrichment,
    run_visual_enrichment_with_progress, save_visual_plan, set_suggestion_status, VisualSession,
};

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
    app: AppHandle,
    media_path: String,
    analysis_run_id: Option<String>,
    transcript_path: Option<String>,
    prefer_whisper: Option<bool>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let edl = edl_from_cache(&analysis, analysis_run_id.as_deref(), &media_path)?;
    let srt = transcript_path.as_ref().map(PathBuf::from);
    let prefer = prefer_whisper.unwrap_or(false);
    let app2 = app.clone();
    let mut on_prog = move |stage: &str, message: &str, percent: f64| {
        progress::emit(&app2, "visual", stage, message, percent);
    };
    if prefer {
        run_visual_enrichment_with_progress(
            PathBuf::from(&media_path).as_path(),
            &edl,
            srt.as_deref(),
            true,
            &visual,
            &mut on_prog,
        )
        .await
    } else {
        progress::emit(&app, "visual", "load", "Cargando…", 10.0);
        let r = run_visual_enrichment(
            PathBuf::from(&media_path).as_path(),
            &edl,
            srt.as_deref(),
            false,
            &visual,
        )
        .await;
        progress::emit(&app, "visual", "done", "Listo", 100.0);
        r
    }
}

/// Force transcription via Whisper (always prefer_whisper=true). Clear action for UI.
#[tauri::command]
pub async fn visual_transcribe_whisper(
    app: AppHandle,
    media_path: String,
    analysis_run_id: Option<String>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let st = crate::pipeline::detectors::whisper_cli::whisper_status();
    if !st.available {
        return Err(AppError::Invalid(format!(
            "Whisper no está disponible. {}\n{}",
            st.detail, st.install_hint
        )));
    }
    let edl = edl_from_cache(&analysis, analysis_run_id.as_deref(), &media_path)?;
    let app2 = app.clone();
    let mut on_prog = move |stage: &str, message: &str, percent: f64| {
        progress::emit(&app2, "visual", stage, message, percent);
    };
    progress::emit(&app, "visual", "start", "Iniciando Whisper…", 2.0);
    run_visual_enrichment_with_progress(
        PathBuf::from(&media_path).as_path(),
        &edl,
        None,
        true,
        &visual,
        &mut on_prog,
    )
    .await
}

#[tauri::command]
pub fn visual_whisper_status() -> AppResult<crate::pipeline::detectors::whisper_cli::WhisperStatus> {
    Ok(crate::pipeline::detectors::whisper_cli::whisper_status())
}

#[tauri::command]
pub async fn visual_install_whisper() -> AppResult<String> {
    crate::pipeline::detectors::whisper_cli::install_openai_whisper().await
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

/// Import image + attach as accepted placement at a transcript moment.
/// Does not rebuild or clear the session transcript.
#[tauri::command]
pub fn visual_attach_image(
    media_path: String,
    analysis_run_id: Option<String>,
    path: String,
    concept: String,
    source_start: f64,
    source_end: f64,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let edl = edl_from_cache(&analysis, analysis_run_id.as_deref(), &media_path)?;
    attach_image_to_moment(
        &visual,
        &edl,
        PathBuf::from(&media_path).as_path(),
        PathBuf::from(&path).as_path(),
        &concept,
        source_start,
        source_end,
    )
}

#[tauri::command]
pub fn visual_import_folder(
    path: String,
    tags: Option<Vec<String>>,
    concepts: Option<Vec<String>>,
    recursive: Option<bool>,
) -> AppResult<serde_json::Value> {
    let r = import_folder(
        PathBuf::from(&path).as_path(),
        tags.unwrap_or_default(),
        concepts.unwrap_or_default(),
        recursive.unwrap_or(false),
    )?;
    Ok(serde_json::to_value(r)?)
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
        "missing" => Some(AssetStatus::Missing),
        _ => None,
    });
    let a = update_asset_meta(&id, title, tags, concepts, lic, st)?;
    Ok(serde_json::to_value(a)?)
}

#[tauri::command]
pub fn visual_list_usage(
    asset_id: Option<String>,
    limit: Option<usize>,
) -> AppResult<serde_json::Value> {
    let rows = list_usage(asset_id.as_deref(), limit.unwrap_or(50))?;
    Ok(serde_json::to_value(rows)?)
}

#[tauri::command]
pub fn visual_scan_missing() -> AppResult<u32> {
    scan_missing_assets()
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
        "planPath": g.plan_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
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
pub fn visual_export_transcript(
    out_dir: String,
    stem: Option<String>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let stem = stem.unwrap_or_else(|| "transcript".into());
    let arts = export_session_transcript(&visual, PathBuf::from(&out_dir).as_path(), &stem)?;
    Ok(serde_json::json!({ "artifacts": arts }))
}

#[tauri::command]
pub fn visual_save_plan(
    path: Option<String>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<String> {
    let plan = {
        let g = visual.lock().map_err(|e| AppError::Message(e.to_string()))?;
        g.plan
            .clone()
            .ok_or_else(|| AppError::Invalid("No hay VisualPlan en sesión.".into()))?
    };
    let extra = path.as_ref().map(PathBuf::from);
    let p = save_visual_plan(&plan, extra.as_deref())?;
    Ok(p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn visual_load_plan(
    path: String,
    visual: State<'_, VisualSessionState>,
) -> AppResult<VisualPlan> {
    let plan = load_visual_plan(PathBuf::from(&path).as_path())?;
    let mut g = visual.lock().map_err(|e| AppError::Message(e.to_string()))?;
    g.plan = Some(plan.clone());
    g.edl_fp = Some(plan.edl_fingerprint.clone());
    g.plan_path = Some(PathBuf::from(&path));
    Ok(plan)
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
            .ok_or_else(|| {
                AppError::Invalid(
                    "No hay VisualPlan. Genera sugerencias y acepta alguna.".into(),
                )
            })?
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
