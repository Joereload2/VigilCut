//! Visual enrichment: library, matching, plan, render, intelligent library.

pub mod compose;
pub mod concepts;
pub mod generation;
pub mod intelligent_match;
pub mod layout;
pub mod library;
pub mod library_dashboard;
pub mod library_requests;
pub mod match_rank;
pub mod needs;
pub mod qa;
pub mod render;
pub mod schema;

#[cfg(test)]
mod intel_flow_tests;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::models::edl::Edl;
use crate::models::event::Span;
use crate::models::transcript::{Transcript, TranscriptStatus};
use crate::models::visual::{
    edl_fingerprint, LicenseStatus, PlacementLayout, PlacementMode, ProtectedRange, ReviewStatus,
    SuggestionStatus, VisualPlacement, VisualPlan, VisualSuggestion,
};
use crate::pipeline::semantic::extract_semantic_events;
use crate::pipeline::time_map::TimeMap;
use crate::pipeline::transcript_engine::write_transcript_artifacts;
use crate::pipeline::visual::compose::{
    evaluate_composition, restore_suggested, snap_placement_edges,
};
use crate::pipeline::visual::library::{get_asset_by_id, import_image, list_active_assets};
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

/// Create a manual placement on the **output** timeline. Transcript is optional.
/// Accepts either an existing `asset_id` or a new `image_path` to import.
pub fn create_manual_placement(
    state: &VisualState,
    edl: &Edl,
    media_path: &Path,
    asset_id: Option<&str>,
    image_path: Option<&Path>,
    output_start: f64,
    output_end: f64,
    display_mode: &str,
    position_x: Option<f64>,
    position_y: Option<f64>,
    size_w: Option<f64>,
    fit: Option<&str>,
    label: Option<String>,
) -> AppResult<serde_json::Value> {
    let mode = PlacementMode::from_user(display_mode);
    let mut layout = PlacementLayout::for_mode(mode);
    if let Some(x) = position_x {
        layout.x = x;
    }
    if let Some(y) = position_y {
        layout.y = y;
    }
    if let Some(w) = size_w {
        layout.w = w;
    }
    layout = layout.clamp();

    let asset = if let Some(id) = asset_id.filter(|s| !s.is_empty()) {
        get_asset_by_id(id)?
    } else if let Some(p) = image_path {
        import_image(p, label.clone(), vec![], vec![], LicenseStatus::Owned)?
    } else {
        return Err(AppError::Invalid(
            "Indica asset_id o image_path para el placement manual.".into(),
        ));
    };

    let time_map = TimeMap::from_edl(edl);
    let out_dur = time_map.output_duration.max(0.1);
    let start = output_start.clamp(0.0, out_dur);
    let mut end = output_end.clamp(0.0, out_dur);
    if end < start + 0.25 {
        end = (start + 3.5).min(out_dur);
    }
    if end <= start {
        return Err(AppError::Invalid(
            "Intervalo de salida inválido para el placement.".into(),
        ));
    }

    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let fp = edl_fingerprint(&edl.keep_ranges());
    if g.plan.is_none() {
        g.plan = Some(VisualPlan::new(
            g.edl_fp.clone().unwrap_or_else(|| fp.clone()),
            media_path.to_string_lossy(),
            fp.clone(),
        ));
    }
    let plan = g.plan.as_mut().unwrap();
    if plan.edl_fingerprint != fp {
        plan.edl_fingerprint = fp.clone();
        plan.placements.clear();
        plan.protected_ranges.clear();
        plan.warnings
            .push("EDL cambió: plan visual reiniciado.".into());
    }

    if plan.is_protected(start, end) {
        return Err(AppError::Invalid(
            "Ese intervalo está protegido (sin imágenes). Quita el rango protegido o elige otro momento.".into(),
        ));
    }

    // Soft-trim against protected ranges: reject hard overlap for MVP clarity
    for pr in &plan.protected_ranges {
        if pr.overlaps(start, end) {
            return Err(AppError::Invalid(format!(
                "Choca con zona protegida {:.1}–{:.1}s: {}",
                pr.output_start, pr.output_end, pr.reason
            )));
        }
    }

    let placement = VisualPlacement::manual(
        &asset.id,
        start,
        end,
        mode,
        layout,
        fit.unwrap_or("cover"),
        label.or_else(|| Some(asset.title.clone())),
    );
    plan.placements.push(placement.clone());
    evaluate_composition(plan);
    plan.warnings.retain(|w| !w.contains("Biblioteca vacía"));

    let plan_out = plan.clone();
    if let Ok(p) = save_visual_plan(&plan_out, None) {
        g.plan_path = Some(p);
    }
    g.edl_fp = Some(fp);

    Ok(serde_json::json!({
        "placement": plan_out.placements.iter().find(|p| p.id == placement.id).cloned().unwrap_or(placement),
        "asset": asset,
        "plan": plan_out,
        "transcript": g.transcript,
        "suggestions": g.suggestions,
        "timeMap": {
            "sourceDuration": time_map.source_duration,
            "outputDuration": time_map.output_duration,
        },
        "message": format!(
            "Placement manual {:.1}–{:.1}s · {:?}",
            start, end, mode
        ),
    }))
}

/// Single write path: assign need asset + create/replace one placement (PM-003).
pub fn use_asset_for_need(
    state: &VisualState,
    edl: &Edl,
    media_path: &Path,
    need_id: &str,
    asset_id: &str,
) -> AppResult<serde_json::Value> {
    use crate::models::visual_intel::NeedCoverage;
    use crate::pipeline::visual::needs::{get_need, update_need};

    let mut need = get_need(need_id)?;
    let asset = get_asset_by_id(asset_id)?;
    need.matched_asset_id = Some(asset.id.clone());
    need.coverage = NeedCoverage::Covered;
    need.match_reasons = vec!["user_selected".into()];
    need.updated_at = chrono::Utc::now().to_rfc3339();
    update_need(&need)?;

    let start = need.output_start.unwrap_or(0.0);
    let end = need
        .output_end
        .unwrap_or(start + need.approx_duration_secs.max(3.5));
    let need_tag = format!("need:{}", need.id);

    let time_map = TimeMap::from_edl(edl);
    let out_dur = time_map.output_duration.max(0.1);
    let start = start.clamp(0.0, out_dur);
    let mut end = end.clamp(0.0, out_dur);
    if end < start + 0.25 {
        end = (start + 3.5).min(out_dur);
    }

    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let fp = edl_fingerprint(&edl.keep_ranges());
    if g.plan.is_none() {
        g.plan = Some(VisualPlan::new(
            g.edl_fp.clone().unwrap_or_else(|| fp.clone()),
            media_path.to_string_lossy(),
            fp.clone(),
        ));
    }
    let plan = g.plan.as_mut().unwrap();
    if plan.edl_fingerprint != fp {
        plan.edl_fingerprint = fp.clone();
        plan.placements.clear();
        plan.protected_ranges.clear();
    }

    let existing = plan.placements.iter().position(|p| {
        p.related_text
            .as_ref()
            .is_some_and(|t| t == &need_tag || t == &need.label)
            || ((p.output_start - start).abs() < 0.2
                && p.related_text
                    .as_ref()
                    .is_some_and(|t| t.contains(&need.label)))
    });

    let placement = if let Some(idx) = existing {
        let pl = &mut plan.placements[idx];
        pl.asset_id = asset.id.clone();
        pl.output_start = start;
        pl.output_end = end;
        pl.related_text = Some(need_tag.clone());
        pl.label = Some(need.label.clone());
        pl.manual_override = true;
        pl.clone()
    } else {
        let mut pl = VisualPlacement::manual(
            &asset.id,
            start,
            end,
            PlacementMode::Fullframe,
            PlacementLayout::for_mode(PlacementMode::Fullframe),
            "cover",
            Some(need.label.clone()),
        );
        pl.related_text = Some(need_tag);
        pl.provenance = "library_match".into();
        plan.placements.push(pl.clone());
        pl
    };

    evaluate_composition(plan);
    let plan_out = plan.clone();
    save_visual_plan(&plan_out, None).map_err(|e| {
        AppError::Message(format!("No se pudo guardar el plan visual: {e}"))
    })?;
    g.edl_fp = Some(fp);

    Ok(serde_json::json!({
        "ok": true,
        "need": need,
        "asset": asset,
        "placement": placement,
        "plan": plan_out,
        "message": format!(
            "Imagen «{}» en {:.1}–{:.1}s",
            asset.title, start, end
        ),
    }))
}

pub fn update_placement(
    state: &VisualState,
    placement_id: &str,
    output_start: Option<f64>,
    output_end: Option<f64>,
    display_mode: Option<&str>,
    position_x: Option<f64>,
    position_y: Option<f64>,
    size_w: Option<f64>,
    size_h: Option<f64>,
    fit: Option<&str>,
    status: Option<&str>,
    review_status: Option<&str>,
    manual_override: Option<bool>,
    related_text: Option<String>,
    restore_ai: Option<bool>,
    opacity: Option<f64>,
) -> AppResult<VisualPlan> {
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let plan = g
        .plan
        .as_mut()
        .ok_or_else(|| AppError::Invalid("No hay VisualPlan en sesión.".into()))?;
    let (check_start, check_end, check_active, check_override) = {
        let pl = plan
            .placements
            .iter_mut()
            .find(|p| p.id == placement_id)
            .ok_or_else(|| AppError::NotFound(placement_id.into()))?;
        if restore_ai == Some(true) {
            restore_suggested(pl);
        }
        if let Some(s) = output_start {
            pl.output_start = s.max(0.0);
            pl.manual_override = true;
        }
        if let Some(e) = output_end {
            pl.output_end = e.max(pl.output_start + 0.25);
            pl.manual_override = true;
        }
        if let Some(m) = display_mode {
            pl.mode = PlacementMode::from_user(m);
            if position_x.is_none() && position_y.is_none() && size_w.is_none() {
                pl.layout = PlacementLayout::for_mode(pl.mode);
            }
            pl.manual_override = true;
        }
        if let Some(x) = position_x {
            pl.layout.x = x;
            pl.manual_override = true;
        }
        if let Some(y) = position_y {
            pl.layout.y = y;
            pl.manual_override = true;
        }
        if let Some(w) = size_w {
            pl.layout.w = w;
            pl.manual_override = true;
        }
        if let Some(h) = size_h {
            pl.layout.h = h;
            pl.manual_override = true;
        }
        if let Some(o) = opacity {
            pl.layout.opacity = o;
        }
        pl.layout = pl.layout.clone().clamp();
        if let Some(f) = fit {
            pl.fit = f.into();
            pl.manual_override = true;
        }
        if let Some(st) = status {
            pl.status = st.into();
        }
        if let Some(rs) = review_status {
            pl.review_status = match rs.to_lowercase().as_str() {
                "approved" | "aceptado" | "ok" => ReviewStatus::Approved,
                "conflict" | "conflicto" => ReviewStatus::Conflict,
                "rejected" | "rechazado" => ReviewStatus::Rejected,
                _ => ReviewStatus::Pending,
            };
        }
        if let Some(mo) = manual_override {
            pl.manual_override = mo;
        }
        if let Some(rt) = related_text {
            pl.related_text = Some(rt);
        }
        (
            pl.output_start,
            pl.output_end,
            pl.status == "active",
            pl.manual_override,
        )
    };
    if check_active && !check_override && plan.is_protected(check_start, check_end) {
        return Err(AppError::Invalid(
            "El placement cae en un rango protegido. Activa override o mueve el bloque.".into(),
        ));
    }
    evaluate_composition(plan);
    let out = plan.clone();
    let _ = save_visual_plan(&out, None);
    Ok(out)
}

/// Magnetic snap placement edges to transcript/cut anchors (output timeline).
pub fn snap_placement(
    state: &VisualState,
    placement_id: &str,
    output_start: f64,
    output_end: f64,
    anchors: Vec<f64>,
    threshold: Option<f64>,
) -> AppResult<VisualPlan> {
    let thr = threshold.unwrap_or(0.18);
    let (s, e) = snap_placement_edges(output_start, output_end, &anchors, thr);
    update_placement(
        state,
        placement_id,
        Some(s),
        Some(e),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(true),
        None,
        None,
        None,
    )
}

pub fn evaluate_plan(state: &VisualState) -> AppResult<VisualPlan> {
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let plan = g
        .plan
        .as_mut()
        .ok_or_else(|| AppError::Invalid("No hay VisualPlan en sesión.".into()))?;
    evaluate_composition(plan);
    let out = plan.clone();
    let _ = save_visual_plan(&out, None);
    Ok(out)
}

pub fn remove_placement(state: &VisualState, placement_id: &str) -> AppResult<VisualPlan> {
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let plan = g
        .plan
        .as_mut()
        .ok_or_else(|| AppError::Invalid("No hay VisualPlan en sesión.".into()))?;
    let before = plan.placements.len();
    plan.placements.retain(|p| p.id != placement_id);
    if plan.placements.len() == before {
        return Err(AppError::NotFound(placement_id.into()));
    }
    plan.touch();
    let out = plan.clone();
    let _ = save_visual_plan(&out, None);
    Ok(out)
}

pub fn add_protected_range(
    state: &VisualState,
    edl: &Edl,
    media_path: &Path,
    output_start: f64,
    output_end: f64,
    reason: Option<String>,
) -> AppResult<VisualPlan> {
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let fp = edl_fingerprint(&edl.keep_ranges());
    if g.plan.is_none() {
        g.plan = Some(VisualPlan::new(
            fp.clone(),
            media_path.to_string_lossy(),
            fp.clone(),
        ));
    }
    let plan = g.plan.as_mut().unwrap();
    if plan.edl_fingerprint != fp {
        plan.edl_fingerprint = fp;
    }
    let pr = ProtectedRange::new(
        output_start,
        output_end,
        reason.unwrap_or_else(|| "Sin B-roll".into()),
    );
    // Deactivate overlapping placements
    for pl in plan.placements.iter_mut() {
        if pr.overlaps(pl.output_start, pl.output_end) {
            pl.status = "blocked_protected".into();
        }
    }
    plan.protected_ranges.push(pr);
    plan.touch();
    let out = plan.clone();
    let _ = save_visual_plan(&out, None);
    Ok(out)
}

pub fn remove_protected_range(state: &VisualState, range_id: &str) -> AppResult<VisualPlan> {
    let mut g = state.lock().map_err(|e| AppError::Message(e.to_string()))?;
    let plan = g
        .plan
        .as_mut()
        .ok_or_else(|| AppError::Invalid("No hay VisualPlan en sesión.".into()))?;
    let before = plan.protected_ranges.len();
    plan.protected_ranges.retain(|r| r.id != range_id);
    if plan.protected_ranges.len() == before {
        return Err(AppError::NotFound(range_id.into()));
    }
    // Reactivate placements that were only blocked by protection (heuristic)
    for pl in plan.placements.iter_mut() {
        if pl.status == "blocked_protected" {
            let still = plan
                .protected_ranges
                .iter()
                .any(|r| r.overlaps(pl.output_start, pl.output_end));
            if !still {
                pl.status = "active".into();
            }
        }
    }
    plan.touch();
    let out = plan.clone();
    let _ = save_visual_plan(&out, None);
    Ok(out)
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
        plan.warnings
            .push("EDL cambió: VisualPlan invalidado. Vuelve a generar sugerencias.".into());
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
    let tr = g.transcript.as_ref().ok_or_else(|| {
        AppError::Invalid("No hay transcripción en sesión. Genera sugerencias primero.".into())
    })?;
    write_transcript_artifacts(tr, out_dir, stem)
}
