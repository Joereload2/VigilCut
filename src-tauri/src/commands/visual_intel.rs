//! Tauri commands for intelligent visual library (concepts, needs, generation, QA).

use tauri::State;

use crate::commands::analyze::AnalysisCache;
use crate::commands::visual::VisualSessionState;
use crate::error::{AppError, AppResult};
use crate::models::visual::{edl_fingerprint, PlacementLayout, PlacementMode, VisualPlacement};
use crate::models::visual_intel::{CostPolicy, CoverageSummary, NeedCoverage, VisualConcept};
use crate::pipeline::semantic::extract_semantic_events;
use crate::pipeline::time_map::TimeMap;
use crate::pipeline::visual::concepts::{
    insert_concept, list_concepts, seed_economy_theme, upsert_theme,
};
use crate::pipeline::visual::generation::daily_feed;
use crate::pipeline::visual::generation::provider::select_provider;
use crate::pipeline::visual::generation::supervision::{
    cancel_job, queue_regenerate, supervision_snapshot,
};
use crate::pipeline::visual::generation::worker::{
    cover_project_needs, human_approve_candidate, human_reject_candidate,
    human_reject_candidate_with_reason, list_pending_review, queue_generation_for_need,
    worker_tick,
};
use crate::pipeline::visual::intelligent_match::{apply_best_match, match_need, MatchOptions};
use crate::pipeline::visual::needs::{
    coverage_for_project, detect_needs_from_semantics, get_need, list_needs, merge_detected_needs,
    skip_need, update_need,
};
use crate::pipeline::visual::save_visual_plan;

fn edl_helper(
    analysis: &AnalysisCache,
    run_id: Option<&str>,
    media_path: &str,
) -> AppResult<crate::models::edl::Edl> {
    let map = analysis
        .runs
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    if let Some(id) = run_id {
        if let Some(run) = map.get(id) {
            return Ok(run.edl.clone());
        }
    }
    let mut best = None;
    for run in map.values() {
        if run.media_path.replace('\\', "/") == media_path.replace('\\', "/") {
            best = Some(run.edl.clone());
        }
    }
    best.ok_or_else(|| {
        AppError::Invalid("No hay EDL. Analiza el video en modo Silencios primero.".into())
    })
}

#[tauri::command]
pub fn visual_seed_theme_economy() -> AppResult<serde_json::Value> {
    let concepts = seed_economy_theme()?;
    Ok(serde_json::json!({ "count": concepts.len(), "concepts": concepts }))
}

#[tauri::command]
pub fn visual_list_concepts(
    theme_id: Option<String>,
    limit: Option<usize>,
) -> AppResult<serde_json::Value> {
    let list = list_concepts(theme_id.as_deref(), limit.unwrap_or(100))?;
    Ok(serde_json::to_value(list)?)
}

#[tauri::command]
pub fn visual_create_concept(
    title: String,
    theme_slug: Option<String>,
    meanings: Option<Vec<String>>,
    positive_contexts: Option<Vec<String>>,
    negative_contexts: Option<Vec<String>>,
    hard_exclusions: Option<Vec<String>>,
) -> AppResult<serde_json::Value> {
    let theme_id = if let Some(slug) = theme_slug {
        Some(upsert_theme(&slug, &slug, None)?.id)
    } else {
        None
    };
    let mut c = VisualConcept::new(title, theme_id);
    if let Some(m) = meanings {
        c.meanings = m;
    }
    if let Some(p) = positive_contexts {
        c.positive_contexts = p;
    }
    if let Some(n) = negative_contexts {
        c.negative_contexts = n;
    }
    if let Some(h) = hard_exclusions {
        c.hard_exclusions = h;
    }
    let c = insert_concept(c)?;
    Ok(serde_json::to_value(c)?)
}

#[tauri::command]
pub fn visual_detect_needs(
    media_path: String,
    analysis_run_id: Option<String>,
    max_needs: Option<usize>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let edl = edl_helper(&analysis, analysis_run_id.as_deref(), &media_path)?;
    let project_key = edl_fingerprint(&edl.keep_ranges());
    let time_map = TimeMap::from_edl(&edl);
    let g = visual
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    let tr = g.transcript.as_ref().ok_or_else(|| {
        AppError::Invalid("No hay transcripción. Genera sugerencias o usa Whisper primero.".into())
    })?;
    let semantics = extract_semantic_events(tr, &project_key, &time_map);
    drop(g);
    let detected = detect_needs_from_semantics(
        &project_key,
        Some(&media_path),
        &semantics,
        max_needs.unwrap_or(24),
    );
    // Non-destructive: keep needs with jobs/coverage in progress
    let needs = merge_detected_needs(&project_key, detected)?;
    let summary = CoverageSummary::from_needs(&needs);
    Ok(serde_json::json!({
        "projectKey": project_key,
        "needs": needs,
        "coverage": summary,
        "semanticCount": semantics.len(),
    }))
}

/// Search library only (no generation) for a need.
#[tauri::command]
pub fn visual_search_library_for_need(need_id: String) -> AppResult<serde_json::Value> {
    let mut need = get_need(&need_id)?;
    let ranked = match_need(&need, &MatchOptions::default());
    let matched = apply_best_match(&mut need, &MatchOptions::default());
    if matched {
        update_need(&need)?;
    }
    Ok(serde_json::json!({
        "matched": matched,
        "need": need,
        "candidates": ranked,
        "message": if matched {
            "Se encontró una imagen en la biblioteca"
        } else {
            "No hay imagen adecuada en la biblioteca"
        },
    }))
}

/// Human chose a specific library asset for a scene (picker "Usar esta imagen").
#[tauri::command]
pub fn visual_assign_need_asset(need_id: String, asset_id: String) -> AppResult<serde_json::Value> {
    let mut need = get_need(&need_id)?;
    need.matched_asset_id = Some(asset_id);
    need.coverage = NeedCoverage::Covered;
    need.match_reasons = vec!["user_selected".into()];
    need.updated_at = chrono::Utc::now().to_rfc3339();
    update_need(&need)?;
    Ok(serde_json::json!({ "need": need, "ok": true }))
}

/// Single orchestrator: assign need + one placement (create or replace). PM-003.
#[tauri::command]
pub fn visual_use_asset_for_need(
    need_id: String,
    asset_id: String,
    media_path: String,
    analysis_run_id: Option<String>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let edl = edl_helper(&analysis, analysis_run_id.as_deref(), &media_path)?;
    crate::pipeline::visual::use_asset_for_need(
        &visual,
        &edl,
        std::path::Path::new(&media_path),
        &need_id,
        &asset_id,
    )
}

#[tauri::command]
pub fn visual_list_needs(project_key: String) -> AppResult<serde_json::Value> {
    let needs = list_needs(&project_key)?;
    let coverage = coverage_for_project(&project_key)?;
    Ok(serde_json::json!({ "needs": needs, "coverage": coverage }))
}

#[tauri::command]
pub fn visual_coverage(project_key: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(coverage_for_project(&project_key)?)?)
}

#[tauri::command]
pub fn visual_skip_need(need_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(skip_need(&need_id)?)?)
}

#[tauri::command]
pub async fn visual_cover_needs(
    project_key: String,
    generate_missing: Option<bool>,
    max_generate: Option<u32>,
) -> AppResult<serde_json::Value> {
    cover_project_needs(
        &project_key,
        generate_missing.unwrap_or(false),
        max_generate.unwrap_or(5),
    )
    .await
}

#[tauri::command]
pub async fn visual_worker_tick(max_jobs: Option<u32>) -> AppResult<serde_json::Value> {
    let n = worker_tick(max_jobs.unwrap_or(3)).await?;
    Ok(serde_json::json!({ "processed": n }))
}

#[tauri::command]
pub fn visual_list_review_queue(limit: Option<usize>) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(list_pending_review(
        limit.unwrap_or(50),
    )?)?)
}

#[tauri::command]
pub fn visual_approve_candidate(candidate_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(human_approve_candidate(
        &candidate_id,
    )?)?)
}

#[tauri::command]
pub fn visual_reject_candidate(
    candidate_id: String,
    reason: Option<String>,
) -> AppResult<serde_json::Value> {
    if let Some(r) = reason {
        human_reject_candidate_with_reason(&candidate_id, Some(&r))?;
    } else {
        human_reject_candidate(&candidate_id)?;
    }
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn visual_supervision(project_key: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(supervision_snapshot(&project_key)?)?)
}

/// Global daily/pending snapshot without a video project (Codex HIGH-008).
#[tauri::command]
pub fn visual_supervision_global() -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::generation::supervision::supervision_snapshot_global()?,
    )?)
}

/// Enqueue-only: returns after commit; supervisor processes the queue (Codex CRIT-001).
#[tauri::command]
pub async fn visual_generate_need(need_id: String) -> AppResult<serde_json::Value> {
    let mut need = get_need(&need_id)?;
    // Search library first
    if apply_best_match(&mut need, &MatchOptions::default()) {
        update_need(&need)?;
        return Ok(serde_json::json!({
            "action": "reused",
            "need": need,
            "message": "Se reutilizó una imagen de la biblioteca",
        }));
    }
    let job_id = queue_generation_for_need(&mut need, false)?;
    Ok(serde_json::json!({
        "action": "queued",
        "jobId": job_id,
        "need": get_need(&need_id)?,
        "snapshot": supervision_snapshot(&need.project_key)?,
        "message": "En cola — el supervisor generará la imagen en segundo plano",
    }))
}

#[tauri::command]
pub fn visual_cancel_job(job_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(cancel_job(&job_id)?)?)
}

/// Enqueue-only regenerate (Codex CRIT-001 / HIGH-007).
#[tauri::command]
pub async fn visual_regenerate_need(need_id: String) -> AppResult<serde_json::Value> {
    let job_id = queue_regenerate(&need_id)?;
    let need = get_need(&need_id)?;
    Ok(serde_json::json!({
        "jobId": job_id,
        "need": need,
        "snapshot": supervision_snapshot(&need.project_key)?,
        "message": "Regeneración en cola",
    }))
}

#[tauri::command]
pub async fn visual_approve_and_use(
    candidate_id: String,
    media_path: Option<String>,
    analysis_run_id: Option<String>,
    place: Option<bool>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let asset = human_approve_candidate(&candidate_id)?;
    let place = place.unwrap_or(true);
    let mut placement_added = false;
    if place {
        if let Some(mp) = media_path {
            // Attach as placement if we have a need with times
            let cand =
                crate::pipeline::visual::generation::supervision::get_candidate(&candidate_id)?;
            if let Some(nid) = cand.need_id {
                let need = get_need(&nid)?;
                if let (Some(s), Some(e)) = (need.output_start, need.output_end) {
                    // Codex HIGH-005: never invent a 60s EDL — require real analysis
                    match edl_helper(&analysis, analysis_run_id.as_deref(), &mp) {
                        Ok(edl) => {
                            let mut g = visual
                                .lock()
                                .map_err(|err| AppError::Message(err.to_string()))?;
                            let fp = edl_fingerprint(&edl.keep_ranges());
                            if g.plan.is_none() {
                                g.plan = Some(crate::models::visual::VisualPlan::new(
                                    &fp,
                                    &mp,
                                    fp.clone(),
                                ));
                            }
                            if let Some(plan) = g.plan.as_mut() {
                                let exists = plan.placements.iter().any(|p| {
                                    p.asset_id == asset.id && (p.output_start - s).abs() < 0.2
                                });
                                if !exists {
                                    let mut pl = VisualPlacement::manual(
                                        &asset.id,
                                        s,
                                        e,
                                        PlacementMode::Fullframe,
                                        PlacementLayout::for_mode(PlacementMode::Fullframe),
                                        "cover",
                                        Some(need.label.clone()),
                                    );
                                    pl.provenance = "library_generated".into();
                                    plan.placements.push(pl);
                                    plan.touch();
                                    save_visual_plan(plan, None).map_err(|err| {
                                        AppError::Message(format!(
                                            "Aprobada en biblioteca pero no se pudo guardar el plan: {err}"
                                        ))
                                    })?;
                                    placement_added = true;
                                }
                            }
                        }
                        Err(edl_err) => {
                            // Asset already approved; report place failure explicitly
                            return Ok(serde_json::json!({
                                "asset": asset,
                                "placementAdded": false,
                                "placeError": edl_err.to_string(),
                                "message": "Imagen aprobada en la biblioteca. No se pudo colocar: analiza el video (EDL) primero.",
                            }));
                        }
                    }
                }
            }
        }
    }
    Ok(serde_json::json!({
        "asset": asset,
        "placementAdded": placement_added,
        "message": if placement_added {
            "Imagen aprobada y en el plan"
        } else {
            "Imagen aprobada y en la biblioteca"
        },
    }))
}

#[tauri::command]
pub fn visual_daily_feed_settings() -> AppResult<serde_json::Value> {
    daily_feed::settings_json()
}

#[tauri::command]
pub fn visual_daily_feed_set_enabled(enabled: bool) -> AppResult<serde_json::Value> {
    let s = daily_feed::set_enabled(enabled)?;
    if enabled {
        crate::pipeline::visual::generation::supervisor::request_wake();
    }
    Ok(serde_json::to_value(s)?)
}

/// Manual/debug cycle; production path is the resident supervisor.
#[tauri::command]
pub async fn visual_daily_feed_cycle() -> AppResult<serde_json::Value> {
    daily_feed::run_daily_cycle_forced().await
}

#[tauri::command]
pub fn visual_daily_week_summary() -> AppResult<serde_json::Value> {
    daily_feed::week_summary()
}

#[tauri::command]
pub fn visual_apply_needs_to_plan(
    media_path: String,
    analysis_run_id: Option<String>,
    project_key: Option<String>,
    analysis: State<'_, AnalysisCache>,
    visual: State<'_, VisualSessionState>,
) -> AppResult<serde_json::Value> {
    let edl = edl_helper(&analysis, analysis_run_id.as_deref(), &media_path)?;
    let fp = edl_fingerprint(&edl.keep_ranges());
    let key = project_key.unwrap_or_else(|| fp.clone());
    let needs = list_needs(&key)?;
    let mut g = visual
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?;
    if g.plan.is_none() {
        g.plan = Some(crate::models::visual::VisualPlan::new(
            &fp,
            &media_path,
            fp.clone(),
        ));
    }
    let plan = g.plan.as_mut().unwrap();
    let mut added = 0u32;
    for n in &needs {
        if !matches!(n.coverage, NeedCoverage::Matched | NeedCoverage::Covered) {
            continue;
        }
        let Some(asset_id) = &n.matched_asset_id else {
            continue;
        };
        let start = n.output_start.unwrap_or(0.0);
        let end = n
            .output_end
            .unwrap_or(start + n.approx_duration_secs.max(3.5));
        if plan
            .placements
            .iter()
            .any(|p| p.asset_id == *asset_id && (p.output_start - start).abs() < 0.2)
        {
            continue;
        }
        let mut pl = VisualPlacement::manual(
            asset_id,
            start,
            end,
            PlacementMode::Fullframe,
            PlacementLayout::for_mode(PlacementMode::Fullframe),
            "cover",
            Some(n.label.clone()),
        );
        pl.provenance = if n.generation_job_id.is_some() {
            "library_generated".into()
        } else {
            "library_match".into()
        };
        pl.confidence = n.match_score.unwrap_or(0.7);
        pl.semantic_event_id = n.semantic_event_id.clone();
        pl.related_text = Some(n.label.clone());
        plan.placements.push(pl);
        added += 1;
    }
    plan.touch();
    let out = plan.clone();
    let _ = save_visual_plan(&out, None);
    Ok(serde_json::json!({
        "added": added,
        "plan": out,
        "coverage": coverage_for_project(&key)?,
    }))
}

#[tauri::command]
pub async fn visual_probe_image_provider() -> AppResult<serde_json::Value> {
    let policy = CostPolicy::from_env();
    let p = select_provider(policy.paid_providers_enabled);
    let probe = p
        .probe()
        .await
        .map_err(|e| AppError::Message(e.to_string()))?;
    if let Ok(conn) = crate::pipeline::visual::library::open_db() {
        let _ = conn.execute(
            r#"INSERT INTO provider_capabilities (
                id, provider, model, supports_image, free_tier, last_probe_ok, last_probe_at,
                last_error, latency_ms, notes
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
            ON CONFLICT(provider, model) DO UPDATE SET
                last_probe_ok=excluded.last_probe_ok,
                last_probe_at=excluded.last_probe_at,
                last_error=excluded.last_error,
                latency_ms=excluded.latency_ms,
                notes=excluded.notes"#,
            rusqlite::params![
                uuid::Uuid::new_v4().to_string(),
                probe.provider,
                probe.model,
                probe.supports_image as i64,
                probe.free_tier as i64,
                probe.ok as i64,
                chrono::Utc::now().to_rfc3339(),
                probe.error,
                probe.latency_ms as i64,
                probe.notes,
            ],
        );
    }
    Ok(serde_json::json!({
        "probe": probe,
        "policy": policy,
        "provider": p.name(),
    }))
}

#[tauri::command]
pub fn visual_cost_policy() -> AppResult<serde_json::Value> {
    let p = CostPolicy::from_env();
    let daily = crate::pipeline::visual::generation::cost::daily_generation_count().unwrap_or(0);
    Ok(serde_json::json!({
        "policy": p,
        "dailyGenerations": daily,
    }))
}

#[tauri::command]
pub fn visual_library_dashboard() -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::library_dashboard::dashboard()?,
    )?)
}

#[tauri::command]
pub fn visual_library_concept_coverage(limit: Option<usize>) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::concept_coverage::list(limit.unwrap_or(100))?,
    )?)
}

#[tauri::command]
pub fn visual_library_create_request(
    input: crate::pipeline::visual::library_requests::CreateLibraryRequest,
) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::library_requests::create_request(input)?,
    )?)
}

#[tauri::command]
pub fn visual_library_confirm_request(request_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::library_requests::confirm(&request_id)?,
    )?)
}

#[tauri::command]
pub fn visual_library_preview_request(request_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::library_requests::preview(&request_id)?,
    )?)
}

#[tauri::command]
pub fn visual_library_list_requests(limit: Option<usize>) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::library_requests::list_requests(limit.unwrap_or(30))?,
    )?)
}

#[tauri::command]
pub fn visual_library_cancel_request(request_id: String) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(
        crate::pipeline::visual::library_requests::cancel(&request_id)?,
    )?)
}

#[tauri::command]
pub fn visual_match_need(need_id: String) -> AppResult<serde_json::Value> {
    let mut need = get_need(&need_id)?;
    let ranked = match_need(&need, &MatchOptions::default());
    let matched = apply_best_match(&mut need, &MatchOptions::default());
    if matched {
        update_need(&need)?;
    }
    Ok(serde_json::json!({
        "need": need,
        "candidates": ranked,
        "matched": matched,
    }))
}
