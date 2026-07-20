//! Visual enrichment: library, matching, plan, render.

pub mod library;
pub mod match_rank;
pub mod render;

use std::path::Path;
use std::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::models::edl::Edl;
use crate::models::transcript::Transcript;
use crate::models::visual::{
    edl_fingerprint, SuggestionStatus, VisualPlan, VisualPlacement, VisualSuggestion,
};
use crate::pipeline::semantic::extract_semantic_events;
use crate::pipeline::time_map::TimeMap;
use crate::pipeline::transcript_engine::{build_transcript, write_transcript_artifacts};
use crate::pipeline::visual::library::{import_image, list_active_assets};
use crate::pipeline::visual::match_rank::{rank_suggestions, MatchConfig};
use crate::models::visual::LicenseStatus;

/// In-memory session state for visual runs (also persisted as JSON under cache).
#[derive(Default)]
pub struct VisualSession {
    pub transcript: Option<Transcript>,
    pub suggestions: Vec<VisualSuggestion>,
    pub plan: Option<VisualPlan>,
    pub edl_fp: Option<String>,
}

impl VisualSession {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

pub type VisualState = Mutex<VisualSession>;

pub async fn run_visual_enrichment(
    media_path: &Path,
    edl: &Edl,
    explicit_srt: Option<&Path>,
    prefer_whisper: bool,
    state: &VisualState,
) -> AppResult<serde_json::Value> {
    let run_id = edl_fingerprint(&edl.keep_ranges());
    let time_map = TimeMap::from_edl(edl);
    let tr = build_transcript(
        media_path,
        explicit_srt,
        prefer_whisper,
        Some(run_id.clone()),
    )
    .await?;

    let semantics = extract_semantic_events(&tr, &run_id, &time_map);
    let assets = list_active_assets().unwrap_or_default();
    let suggestions = rank_suggestions(
        &semantics,
        &assets,
        time_map.output_duration,
        &MatchConfig::default(),
    );

    let mut plan = VisualPlan::new(&run_id, media_path.to_string_lossy(), edl_fingerprint(&edl.keep_ranges()));
    if assets.is_empty() {
        plan.warnings
            .push("Biblioteca vacía: importa imágenes y asigna conceptos/tags.".into());
    }
    if matches!(tr.status, crate::models::transcript::TranscriptStatus::Empty) {
        plan.warnings.push(
            "Sin transcripción: no hay conceptos del habla. Importa SRT o activa Whisper.".into(),
        );
    }

    // Persist transcript artifacts next to cache
    let cache = crate::state::AppState::cache_dir()?.join("transcripts");
    let stem = media_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("media");
    let arts = write_transcript_artifacts(&tr, &cache, stem)?;

    {
        let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
        g.transcript = Some(tr.clone());
        g.suggestions = suggestions.clone();
        g.plan = Some(plan.clone());
        g.edl_fp = Some(edl_fingerprint(&edl.keep_ranges()));
    }

    Ok(serde_json::json!({
        "transcript": tr,
        "semanticEvents": semantics,
        "suggestions": suggestions,
        "plan": plan,
        "transcriptArtifacts": arts,
        "timeMap": {
            "sourceDuration": time_map.source_duration,
            "outputDuration": time_map.output_duration,
        }
    }))
}

pub fn set_suggestion_status(
    state: &VisualState,
    suggestion_id: &str,
    status: SuggestionStatus,
) -> AppResult<VisualPlan> {
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let idx = g
        .suggestions
        .iter()
        .position(|s| s.id == suggestion_id)
        .ok_or_else(|| AppError::NotFound(suggestion_id.into()))?;
    g.suggestions[idx].status = status;
    let sug_clone = g.suggestions[idx].clone();

    let plan = g
        .plan
        .as_mut()
        .ok_or_else(|| AppError::Invalid("No visual plan in session".into()))?;

    match status {
        SuggestionStatus::Accepted => {
            plan.placements
                .retain(|p| p.suggestion_id.as_deref() != Some(suggestion_id));
            plan.placements
                .push(VisualPlacement::from_accepted(&sug_clone));
        }
        SuggestionStatus::Rejected | SuggestionStatus::Replaced => {
            plan.placements
                .retain(|p| p.suggestion_id.as_deref() != Some(suggestion_id));
        }
        _ => {}
    }
    plan.updated_at = chrono::Utc::now().to_rfc3339();
    plan.version += 1;
    Ok(plan.clone())
}

pub fn invalidate_if_edl_changed(state: &VisualState, edl: &Edl) -> AppResult<bool> {
    let fp = edl_fingerprint(&edl.keep_ranges());
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    if g.edl_fp.as_deref() == Some(fp.as_str()) {
        return Ok(false);
    }
    if let Some(plan) = g.plan.as_mut() {
        plan.warnings.push(
            "EDL cambió: VisualPlan invalidado. Vuelve a generar sugerencias.".into(),
        );
        plan.placements.clear();
        plan.version += 1;
        plan.updated_at = chrono::Utc::now().to_rfc3339();
    }
    g.suggestions.clear();
    g.edl_fp = Some(fp);
    Ok(true)
}

pub fn import_library_image(
    path: &Path,
    title: Option<String>,
    tags: Vec<String>,
    concepts: Vec<String>,
) -> AppResult<crate::models::visual::MediaAsset> {
    import_image(path, title, tags, concepts, LicenseStatus::Owned)
}
