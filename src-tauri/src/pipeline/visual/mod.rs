//! Visual enrichment: library, matching, plan, render.

pub mod library;
pub mod match_rank;
pub mod render;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::models::edl::Edl;
use crate::models::event::Span;
use crate::models::transcript::{Transcript, TranscriptStatus};
use crate::models::visual::{
    edl_fingerprint, LicenseStatus, SuggestionStatus, VisualPlan, VisualPlacement, VisualSuggestion,
};
use crate::pipeline::semantic::extract_semantic_events;
use crate::pipeline::time_map::TimeMap;
use crate::pipeline::transcript_engine::write_transcript_artifacts;
use crate::pipeline::visual::library::{import_image, list_active_assets};
use crate::pipeline::visual::match_rank::{rank_suggestions, MatchConfig};
use uuid::Uuid;

/// In-memory session state for visual runs (also persisted as JSON under cache).
#[derive(Default)]
pub struct VisualSession {
    pub transcript: Option<Transcript>,
    pub suggestions: Vec<VisualSuggestion>,
    pub plan: Option<VisualPlan>,
    pub edl_fp: Option<String>,
    pub plan_path: Option<PathBuf>,
}

impl VisualSession {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

pub type VisualState = Mutex<VisualSession>;

fn visual_plans_dir() -> AppResult<PathBuf> {
    let d = crate::state::AppState::cache_dir()?.join("visual_plans");
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

/// Persist VisualPlan JSON under cache (and optional explicit path).
pub fn save_visual_plan(plan: &VisualPlan, extra_path: Option<&Path>) -> AppResult<PathBuf> {
    let primary = visual_plans_dir()?.join(format!("{}.visual-plan.json", plan.id));
    let json = serde_json::to_string_pretty(plan)?;
    std::fs::write(&primary, &json)?;
    if let Some(p) = extra_path {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, &json)?;
        return Ok(p.to_path_buf());
    }
    Ok(primary)
}

pub fn load_visual_plan(path: &Path) -> AppResult<VisualPlan> {
    let raw = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&raw)?)
}

pub async fn run_visual_enrichment(
    media_path: &Path,
    edl: &Edl,
    explicit_srt: Option<&Path>,
    prefer_whisper: bool,
    state: &VisualState,
) -> AppResult<serde_json::Value> {
    run_visual_enrichment_with_progress(
        media_path,
        edl,
        explicit_srt,
        prefer_whisper,
        state,
        &mut |_, _, _| {},
    )
    .await
}

pub async fn run_visual_enrichment_with_progress(
    media_path: &Path,
    edl: &Edl,
    explicit_srt: Option<&Path>,
    prefer_whisper: bool,
    state: &VisualState,
    on_progress: &mut crate::pipeline::detectors::whisper_cli::WhisperProgressFn<'_>,
) -> AppResult<serde_json::Value> {
    let run_id = edl_fingerprint(&edl.keep_ranges());
    let time_map = TimeMap::from_edl(edl);
    on_progress("visual", "Preparando transcripción…", 3.0);

    // Preserve an already-ready transcript unless caller forces Whisper or provides a new SRT.
    let existing = {
        let g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
        g.transcript.clone()
    };
    let reuse = existing
        .as_ref()
        .map(|t| {
            matches!(t.status, TranscriptStatus::Ready)
                && !t.segments.is_empty()
                && !prefer_whisper
                && explicit_srt.is_none()
        })
        .unwrap_or(false);

    let tr = if reuse {
        on_progress("visual", "Reutilizando transcripción existente…", 40.0);
        existing.expect("reuse checked")
    } else {
        crate::pipeline::transcript_engine::build_transcript_with_progress(
            media_path,
            explicit_srt,
            prefer_whisper,
            Some(run_id.clone()),
            on_progress,
        )
        .await?
    };

    on_progress("visual", "Extrayendo conceptos…", 93.0);
    let semantics = extract_semantic_events(&tr, &run_id, &time_map);
    let assets = list_active_assets().unwrap_or_default();
    let suggestions = rank_suggestions(
        &semantics,
        &assets,
        time_map.output_duration,
        &MatchConfig::default(),
    );

    // Keep human-accepted placements when re-ranking (same EDL fingerprint).
    let prev_plan = {
        let g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
        g.plan.clone()
    };
    let mut plan = VisualPlan::new(
        &run_id,
        media_path.to_string_lossy(),
        edl_fingerprint(&edl.keep_ranges()),
    );
    if let Some(prev) = prev_plan {
        if prev.edl_fingerprint == plan.edl_fingerprint && !prev.placements.is_empty() {
            plan.placements = prev.placements;
            plan.version = prev.version.max(1);
            plan.id = prev.id;
            plan.created_at = prev.created_at;
        }
    }
    if assets.is_empty() {
        plan.warnings
            .push("Biblioteca vacía: importa imágenes y asigna conceptos/tags.".into());
    }
    if matches!(tr.status, TranscriptStatus::Empty) {
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
    on_progress("visual", "Guardando artefactos…", 97.0);
    let arts = write_transcript_artifacts(&tr, &cache, stem)?;
    let plan_path = save_visual_plan(&plan, None)?;

    {
        let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
        g.transcript = Some(tr.clone());
        g.suggestions = suggestions.clone();
        g.plan = Some(plan.clone());
        g.edl_fp = Some(edl_fingerprint(&edl.keep_ranges()));
        g.plan_path = Some(plan_path.clone());
    }

    on_progress("visual", "Listo", 100.0);
    Ok(serde_json::json!({
        "transcript": tr,
        "semanticEvents": semantics,
        "suggestions": suggestions,
        "plan": plan,
        "planPath": plan_path.to_string_lossy(),
        "transcriptArtifacts": arts,
        "timeMap": {
            "sourceDuration": time_map.source_duration,
            "outputDuration": time_map.output_duration,
        }
    }))
}

/// Import an image for a concept and attach it as an accepted placement on the
/// output timeline at the selected speech moment. Never clears the transcript.
pub fn attach_image_to_moment(
    state: &VisualState,
    edl: &Edl,
    media_path: &Path,
    image_path: &Path,
    concept: &str,
    source_start: f64,
    source_end: f64,
) -> AppResult<serde_json::Value> {
    let concept = concept.trim().to_lowercase();
    if concept.is_empty() {
        return Err(AppError::Invalid("Concepto vacío".into()));
    }
    let asset = import_image(
        image_path,
        Some(concept.clone()),
        vec![concept.clone()],
        vec![concept.clone()],
        LicenseStatus::Owned,
    )?;

    let time_map = TimeMap::from_edl(edl);
    let source_span = Span::new(source_start, source_end.max(source_start + 0.5));
    let mut output = time_map
        .primary_output_span(source_span)
        .unwrap_or_else(|| {
            // Fallback: treat source times as output if fully cut (should be rare)
            Span::new(source_start.min(time_map.output_duration), source_end)
        });
    // Suggest 3.5–5.5s overlay duration on output timeline
    let min_dur = 3.5_f64;
    if output.duration() < min_dur {
        output.end = (output.start + min_dur).min(time_map.output_duration.max(output.start + 1.0));
    }
    if output.end <= output.start {
        return Err(AppError::Invalid(
            "Ese momento del texto cae en un tramo cortado del video. Elige otra frase o exporta sin cortar esa zona.".into(),
        ));
    }

    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let run_id = g
        .edl_fp
        .clone()
        .unwrap_or_else(|| edl_fingerprint(&edl.keep_ranges()));

    if g.plan.is_none() {
        g.plan = Some(VisualPlan::new(
            &run_id,
            media_path.to_string_lossy(),
            edl_fingerprint(&edl.keep_ranges()),
        ));
    }
    // Ensure plan matches current EDL
    if let Some(plan) = g.plan.as_mut() {
        let fp = edl_fingerprint(&edl.keep_ranges());
        if plan.edl_fingerprint != fp {
            plan.edl_fingerprint = fp;
            plan.placements.clear();
            plan.warnings
                .push("EDL cambió: placements previos se reiniciaron.".into());
        }
    }

    let sug = VisualSuggestion {
        id: Uuid::new_v4().to_string(),
        semantic_event_id: format!("manual:{concept}"),
        asset_id: asset.id.clone(),
        source_span,
        output_span: output,
        match_reasons: vec![
            format!("manual:{concept}"),
            "human_attached".into(),
            format!("concept:{concept}"),
        ],
        match_score: 1.0,
        alternatives: Vec::new(),
        status: SuggestionStatus::Accepted,
        asset_title: Some(asset.title.clone()),
        thumbnail_path: asset.thumbnail_path.clone(),
    };

    // Replace prior placement for same suggestion id pattern isn't needed — append
    g.suggestions.retain(|s| {
        !(s.asset_id == asset.id
            && (s.output_span.start - output.start).abs() < 0.05
            && (s.output_span.end - output.end).abs() < 0.05)
    });
    g.suggestions.push(sug.clone());

    let plan = g
        .plan
        .as_mut()
        .ok_or_else(|| AppError::Invalid("No visual plan".into()))?;
    // Avoid exact-time overlaps of same asset
    plan.placements.retain(|p| {
        !(p.asset_id == asset.id
            && (p.output_start - output.start).abs() < 0.05
            && (p.output_end - output.end).abs() < 0.05)
    });
    let mut placement = VisualPlacement::from_accepted(&sug);
    placement.provenance = "human_attached".into();
    plan.placements.push(placement);
    plan.updated_at = chrono::Utc::now().to_rfc3339();
    plan.version += 1;
    plan.warnings.retain(|w| !w.contains("Biblioteca vacía"));

    let plan_out = plan.clone();
    if let Ok(p) = save_visual_plan(&plan_out, None) {
        g.plan_path = Some(p);
    }
    g.edl_fp = Some(edl_fingerprint(&edl.keep_ranges()));

    Ok(serde_json::json!({
        "asset": asset,
        "suggestion": sug,
        "plan": plan_out,
        "transcript": g.transcript,
        "suggestions": g.suggestions,
        "timeMap": {
            "sourceDuration": time_map.source_duration,
            "outputDuration": time_map.output_duration,
        },
        "message": format!(
            "Imagen «{}» adherida al video en {:.1}–{:.1}s (salida)",
            concept, output.start, output.end
        ),
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
    let plan_out = plan.clone();
    // Persist after human decision
    if let Ok(p) = save_visual_plan(&plan_out, None) {
        g.plan_path = Some(p);
    }
    Ok(plan_out)
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
        let _ = save_visual_plan(plan, None);
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

/// Export session transcript projections to a user-chosen directory.
pub fn export_session_transcript(
    state: &VisualState,
    out_dir: &Path,
    stem: &str,
) -> AppResult<Vec<(String, String)>> {
    let g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let tr = g
        .transcript
        .as_ref()
        .ok_or_else(|| AppError::Invalid("No hay transcripción en sesión. Genera sugerencias primero.".into()))?;
    write_transcript_artifacts(tr, out_dir, stem)
}
