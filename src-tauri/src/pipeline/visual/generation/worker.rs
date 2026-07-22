//! Generation job queue worker — can run without UI.

use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::models::visual::{LicenseStatus, MediaAsset};
use crate::models::visual_intel::{
    AssetProvenance, CandidateStatus, CostPolicy, GeneratedCandidate, JobStatus, NeedCoverage,
    QaStatus, VisualNeed,
};
use crate::pipeline::visual::generation::cost::{
    can_enqueue_generation, increment_generation_counter, CostGate,
};
use crate::pipeline::visual::generation::provider::{select_provider, GenerationRequest};
use crate::pipeline::visual::library::{import_image, open_db};
use crate::pipeline::visual::needs::{get_need, update_need};
use crate::pipeline::visual::qa::{persist_qa_check, review_image, QaThresholds, SemanticHints};

pub fn build_prompt(need: &VisualNeed) -> (String, String) {
    let mut prompt = format!(
        "Photorealistic stock photo, no logos, no readable brand text: {}. Aspect ratio {}.",
        need.label, need.desired_aspect
    );
    if !need.required_contexts.is_empty() {
        prompt.push_str(&format!(" Context: {}.", need.required_contexts.join(", ")));
    }
    let mut negative = need.hard_exclusions.join(", ");
    if !need.forbidden_contexts.is_empty() {
        if !negative.is_empty() {
            negative.push_str(", ");
        }
        negative.push_str(&need.forbidden_contexts.join(", "));
    }
    if negative.is_empty() {
        negative = "watermark, logo, text overlay, NSFW, celebrity".into();
    }
    (prompt, negative)
}

/// Enqueue generation for an uncovered need (idempotent v1 key).
pub fn queue_generation_for_need(
    need: &mut VisualNeed,
    opportunistic: bool,
) -> AppResult<Option<String>> {
    let idem = format!("need:{}:v1", need.id);
    queue_generation_with_key(need, opportunistic, &idem, "video_need")
}

/// Enqueue with explicit idempotency key (regenerate uses v2, v3, …).
pub fn queue_generation_with_key(
    need: &mut VisualNeed,
    opportunistic: bool,
    idem: &str,
    origin: &str,
) -> AppResult<Option<String>> {
    let policy = CostPolicy::from_env();
    let provider = select_provider(policy.paid_providers_enabled);
    let is_paid = !provider.is_free_tier();
    let cost_kind = if provider.name() == "mock" {
        "local"
    } else if is_paid {
        "paid"
    } else {
        "free_configured"
    };

    match can_enqueue_generation(&policy, &need.project_key, is_paid, opportunistic)? {
        CostGate::Deny { reason } => {
            need.coverage = NeedCoverage::Skipped;
            need.match_reasons.push(format!("policy:{reason}"));
            need.updated_at = chrono::Utc::now().to_rfc3339();
            update_need(need)?;
            return Ok(None);
        }
        CostGate::Allow { .. } => {}
    }

    let conn = open_db()?;
    if let Ok(existing) = conn.query_row(
        "SELECT id, status FROM generation_jobs WHERE idempotency_key = ?1",
        params![idem],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    ) {
        need.generation_job_id = Some(existing.0.clone());
        if existing.1 == "succeeded" {
            need.coverage = NeedCoverage::Covered;
        } else if existing.1 != "failed" && existing.1 != "cancelled" {
            need.coverage = NeedCoverage::Generating;
        }
        update_need(need)?;
        return Ok(Some(existing.0));
    }

    // Block second active job for same need
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM generation_jobs WHERE need_id=?1 AND status IN ('queued','running')",
            params![need.id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if active > 0 {
        return Err(AppError::Invalid(
            "Ya hay una generación en curso para esta necesidad.".into(),
        ));
    }

    let (prompt, negative) = build_prompt(need);
    let now = chrono::Utc::now().to_rfc3339();
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        r#"INSERT INTO generation_jobs (
            id, idempotency_key, need_id, concept_id, status, provider, model,
            prompt, negative_prompt, attempt, max_attempts, last_error, is_paid,
            opportunistic, created_at, updated_at, stage, cost_kind, free_verified, origin
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,0,?10,NULL,?11,?12,?13,?14,'queued',?15,0,?16)"#,
        params![
            id,
            idem,
            need.id,
            need.concept_id,
            JobStatus::Queued.as_str(),
            provider.name(),
            None::<String>,
            prompt,
            negative,
            policy.max_attempts_per_need as i64,
            is_paid as i64,
            opportunistic as i64,
            now,
            now,
            cost_kind,
            origin,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;

    need.generation_job_id = Some(id.clone());
    need.coverage = NeedCoverage::Generating;
    need.updated_at = now;
    update_need(need)?;
    Ok(Some(id))
}

/// Process one queued job. Returns job id if worked.
pub async fn process_next_job() -> AppResult<Option<String>> {
    let job = {
        let conn = open_db()?;
        conn.query_row(
            r#"SELECT id, need_id, prompt, negative_prompt, attempt, max_attempts, is_paid, status
               FROM generation_jobs
               WHERE status = 'queued'
               ORDER BY created_at ASC LIMIT 1"#,
            [],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)? as u32,
                    r.get::<_, i64>(5)? as u32,
                    r.get::<_, i64>(6)? != 0,
                    r.get::<_, String>(7)?,
                ))
            },
        )
        .ok()
    };
    let Some((id, need_id, prompt, negative, attempt, max_attempts, is_paid, _)) = job else {
        return Ok(None);
    };

    let policy = CostPolicy::from_env();
    if is_paid && !policy.paid_providers_enabled {
        mark_job(&id, JobStatus::BlockedPolicy, Some("paid disabled"))?;
        return Ok(Some(id));
    }

    {
        let conn = open_db()?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE generation_jobs SET status=?1, attempt=?2, stage='preparing', updated_at=?3 WHERE id=?4",
            params![JobStatus::Running.as_str(), (attempt + 1) as i64, now, id],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }

    if super::supervision::is_cancel_requested(&id) {
        mark_job(&id, JobStatus::Cancelled, Some("cancelled by user"))?;
        return Ok(Some(id));
    }

    let provider = select_provider(policy.paid_providers_enabled);
    let _ = super::supervision::set_job_stage(&id, "waiting_provider");
    let req = GenerationRequest {
        prompt: prompt.clone(),
        negative_prompt: negative.clone(),
        model: None,
        width: 1280,
        height: 720,
        seed: None,
        job_id: id.clone(),
    };

    let _ = super::supervision::set_job_stage(&id, "generating");
    if super::supervision::is_cancel_requested(&id) {
        mark_job(&id, JobStatus::Cancelled, Some("cancelled by user"))?;
        return Ok(Some(id));
    }

    match provider.generate(&req).await {
        Ok(result) => {
            if super::supervision::is_cancel_requested(&id) {
                // Drop incomplete if possible
                let _ = std::fs::remove_file(&result.local_path);
                mark_job(&id, JobStatus::Cancelled, Some("cancelled by user"))?;
                return Ok(Some(id));
            }
            increment_generation_counter()?;
            let _ = super::supervision::set_job_stage(&id, "file_review");
            let cand_id = uuid::Uuid::new_v4().to_string();
            let cand_now = chrono::Utc::now().to_rfc3339();
            {
                let conn = open_db()?;
                conn.execute(
                    r#"INSERT INTO generated_candidates (
                        id, job_id, need_id, local_path, sha256, perceptual_hash, status,
                        technical_score, semantic_score, qa_decision, qa_reason, approved_asset_id,
                        created_at, updated_at, origin, width, height, mime_type, cost_kind,
                        free_verified, provider, model
                    ) VALUES (?1,?2,?3,?4,NULL,NULL,?5,NULL,NULL,NULL,NULL,NULL,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)"#,
                    params![
                        cand_id,
                        id,
                        need_id,
                        result.local_path.to_string_lossy(),
                        CandidateStatus::AutomatedReview.as_str(),
                        cand_now,
                        cand_now,
                        "video_need",
                        result.width as i64,
                        result.height as i64,
                        result.mime_type,
                        result.cost_kind.as_str(),
                        result.free_verified as i64,
                        result.provider,
                        result.model,
                    ],
                )
                .map_err(|e| AppError::Message(e.to_string()))?;
                let _ = conn.execute(
                    "UPDATE generation_jobs SET cost_kind=?1, free_verified=?2, prompt_strategy=?3, model=?4, updated_at=?5 WHERE id=?6",
                    params![
                        result.cost_kind.as_str(),
                        result.free_verified as i64,
                        result.prompt_strategy,
                        result.model,
                        cand_now,
                        id
                    ],
                );
            }

            let _ = super::supervision::set_job_stage(&id, "evaluating");
            let hints = need_id
                .as_ref()
                .and_then(|nid| get_need(nid).ok())
                .map(|n| SemanticHints {
                    label: n.label,
                    meanings: n.terms,
                    hard_exclusions: n.hard_exclusions,
                    negative_contexts: n.forbidden_contexts,
                });
            let mut check =
                review_image(&result.local_path, hints.as_ref(), &QaThresholds::default())?;
            check.candidate_id = Some(cand_id.clone());
            persist_qa_check(&check)?;

            match check.decision.as_str() {
                "approve" => {
                    let asset = promote_candidate(
                        &cand_id,
                        &result.local_path,
                        &prompt,
                        &negative,
                        &result.provider,
                        &result.model,
                        need_id.as_deref(),
                    )?;
                    {
                        let conn = open_db()?;
                        conn.execute(
                            "UPDATE generated_candidates SET status=?1, technical_score=?2, semantic_score=?3,
                             qa_decision=?4, qa_reason=?5, approved_asset_id=?6, updated_at=?7 WHERE id=?8",
                            params![
                                CandidateStatus::Approved.as_str(),
                                check.technical_quality,
                                check.semantic_alignment,
                                check.decision,
                                check.reason,
                                asset.id,
                                chrono::Utc::now().to_rfc3339(),
                                cand_id,
                            ],
                        )
                        .map_err(|e| AppError::Message(e.to_string()))?;
                    }
                    mark_job(&id, JobStatus::Succeeded, None)?;
                    if let Some(nid) = need_id {
                        if let Ok(mut need) = get_need(&nid) {
                            need.matched_asset_id = Some(asset.id);
                            need.coverage = NeedCoverage::Covered;
                            need.generation_job_id = Some(id.clone());
                            need.match_reasons = vec!["generated+approved".into()];
                            need.updated_at = chrono::Utc::now().to_rfc3339();
                            update_need(&need)?;
                        }
                    }
                }
                "needs_human" => {
                    {
                        let conn = open_db()?;
                        conn.execute(
                            "UPDATE generated_candidates SET status=?1, technical_score=?2, semantic_score=?3,
                             qa_decision=?4, qa_reason=?5, updated_at=?6 WHERE id=?7",
                            params![
                                CandidateStatus::NeedsHumanReview.as_str(),
                                check.technical_quality,
                                check.semantic_alignment,
                                check.decision,
                                check.reason,
                                chrono::Utc::now().to_rfc3339(),
                                cand_id,
                            ],
                        )
                        .map_err(|e| AppError::Message(e.to_string()))?;
                    }
                    mark_job(&id, JobStatus::Succeeded, None)?;
                    if let Some(nid) = need_id {
                        if let Ok(mut need) = get_need(&nid) {
                            need.coverage = NeedCoverage::NeedsReview;
                            need.updated_at = chrono::Utc::now().to_rfc3339();
                            update_need(&need)?;
                        }
                    }
                }
                _ => {
                    {
                        let conn = open_db()?;
                        conn.execute(
                            "UPDATE generated_candidates SET status=?1, technical_score=?2, semantic_score=?3,
                             qa_decision=?4, qa_reason=?5, updated_at=?6 WHERE id=?7",
                            params![
                                CandidateStatus::Rejected.as_str(),
                                check.technical_quality,
                                check.semantic_alignment,
                                check.decision,
                                check.reason,
                                chrono::Utc::now().to_rfc3339(),
                                cand_id,
                            ],
                        )
                        .map_err(|e| AppError::Message(e.to_string()))?;
                    }
                    if attempt + 1 >= max_attempts {
                        mark_job(&id, JobStatus::Failed, Some(&check.reason))?;
                        if let Some(nid) = need_id {
                            if let Ok(mut need) = get_need(&nid) {
                                need.coverage = NeedCoverage::Failed;
                                need.updated_at = chrono::Utc::now().to_rfc3339();
                                update_need(&need)?;
                            }
                        }
                    } else {
                        let conn = open_db()?;
                        conn.execute(
                            "UPDATE generation_jobs SET status=?1, last_error=?2, updated_at=?3 WHERE id=?4",
                            params![
                                JobStatus::Queued.as_str(),
                                check.reason,
                                chrono::Utc::now().to_rfc3339(),
                                id
                            ],
                        )
                        .map_err(|e| AppError::Message(e.to_string()))?;
                    }
                }
            }
            Ok(Some(id))
        }
        Err(e) => {
            let msg = e.to_string();
            if attempt + 1 >= max_attempts {
                mark_job(&id, JobStatus::Failed, Some(&msg))?;
                if let Some(nid) = need_id {
                    if let Ok(mut need) = get_need(&nid) {
                        need.coverage = NeedCoverage::Failed;
                        need.updated_at = chrono::Utc::now().to_rfc3339();
                        update_need(&need)?;
                    }
                }
            } else {
                let conn = open_db()?;
                conn.execute(
                    "UPDATE generation_jobs SET status=?1, last_error=?2, updated_at=?3 WHERE id=?4",
                    params![
                        JobStatus::Queued.as_str(),
                        msg,
                        chrono::Utc::now().to_rfc3339(),
                        id
                    ],
                )
                .map_err(|e| AppError::Message(e.to_string()))?;
            }
            Ok(Some(id))
        }
    }
}

fn mark_job(id: &str, status: JobStatus, err: Option<&str>) -> AppResult<()> {
    let conn = open_db()?;
    conn.execute(
        "UPDATE generation_jobs SET status=?1, last_error=?2, updated_at=?3 WHERE id=?4",
        params![status.as_str(), err, chrono::Utc::now().to_rfc3339(), id],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

fn promote_candidate(
    _candidate_id: &str,
    path: &std::path::Path,
    prompt: &str,
    negative: &str,
    provider: &str,
    model: &str,
    need_id: Option<&str>,
) -> AppResult<MediaAsset> {
    let title = need_id
        .and_then(|id| get_need(id).ok())
        .map(|n| n.label)
        .unwrap_or_else(|| "generated".into());
    let concepts = need_id
        .and_then(|id| get_need(id).ok())
        .map(|n| n.terms)
        .unwrap_or_default();
    let mut asset = import_image(
        path,
        Some(title),
        concepts.clone(),
        concepts,
        LicenseStatus::Owned,
    )?;
    // Enrich provenance on row
    let conn = open_db()?;
    let prov = AssetProvenance {
        source: "ai_generated".into(),
        provider: Some(provider.into()),
        model: Some(model.into()),
        prompt: Some(prompt.into()),
        negative_prompt: Some(negative.into()),
        seed: None,
        generated_at: Some(chrono::Utc::now().to_rfc3339()),
    };
    let prov_s = serde_json::to_string(&prov).unwrap_or_else(|_| "{}".into());
    conn.execute(
        "UPDATE media_assets SET provenance_json=?1, qa_status=?2, commercial_use=1, source=?3, updated_at=?4 WHERE id=?5",
        params![
            prov_s,
            QaStatus::Approved.as_str(),
            format!("ai:{provider}"),
            chrono::Utc::now().to_rfc3339(),
            asset.id
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    asset.provenance = Some(prov);
    asset.qa_status = QaStatus::Approved;
    Ok(asset)
}

/// Process up to `max_jobs` queued items.
pub async fn worker_tick(max_jobs: u32) -> AppResult<u32> {
    let mut n = 0u32;
    for _ in 0..max_jobs {
        match process_next_job().await? {
            Some(_) => n += 1,
            None => break,
        }
    }
    Ok(n)
}

/// Human approve a candidate → library asset (idempotent: second call returns same asset).
pub fn human_approve_candidate(candidate_id: &str) -> AppResult<MediaAsset> {
    let conn = open_db()?;
    let (path, need_id, job_id, status, existing_asset): (
        String,
        Option<String>,
        String,
        String,
        Option<String>,
    ) = conn
        .query_row(
            "SELECT local_path, need_id, job_id, status, approved_asset_id FROM generated_candidates WHERE id = ?1",
            params![candidate_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|e| AppError::NotFound(e.to_string()))?;
    if let Some(aid) = existing_asset.filter(|s| !s.is_empty()) {
        return crate::pipeline::visual::library::get_asset_by_id(&aid);
    }
    if status == "rejected" || status == "discarded" {
        return Err(AppError::Invalid(
            "Este candidato fue rechazado y no se puede aprobar.".into(),
        ));
    }
    let path = std::path::PathBuf::from(path);
    if !path.is_file() {
        return Err(AppError::NotFound(path.display().to_string()));
    }
    let (prompt, neg, provider, model): (String, String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT prompt, negative_prompt, provider, model FROM generation_jobs WHERE id = ?1",
            params![job_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap_or_else(|_| (String::new(), String::new(), None, None));
    let asset = promote_candidate(
        candidate_id,
        &path,
        &prompt,
        &neg,
        provider.as_deref().unwrap_or("unknown"),
        model.as_deref().unwrap_or("unknown"),
        need_id.as_deref(),
    )?;
    conn.execute(
        "UPDATE generated_candidates SET status=?1, approved_asset_id=?2, updated_at=?3 WHERE id=?4",
        params![
            CandidateStatus::Approved.as_str(),
            asset.id,
            chrono::Utc::now().to_rfc3339(),
            candidate_id
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    if let Some(nid) = need_id {
        if let Ok(mut need) = get_need(&nid) {
            need.matched_asset_id = Some(asset.id.clone());
            need.coverage = NeedCoverage::Covered;
            need.updated_at = chrono::Utc::now().to_rfc3339();
            update_need(&need)?;
        }
    }
    Ok(asset)
}

pub fn human_reject_candidate(candidate_id: &str) -> AppResult<()> {
    human_reject_candidate_with_reason(candidate_id, None)
}

pub fn human_reject_candidate_with_reason(
    candidate_id: &str,
    reason: Option<&str>,
) -> AppResult<()> {
    let conn = open_db()?;
    conn.execute(
        "UPDATE generated_candidates SET status=?1, reject_reason=?2, updated_at=?3 WHERE id=?4",
        params![
            CandidateStatus::Rejected.as_str(),
            reason,
            chrono::Utc::now().to_rfc3339(),
            candidate_id
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    // Free the need for regenerate
    if let Ok(c) = super::supervision::get_candidate(candidate_id) {
        if let Some(nid) = c.need_id {
            if let Ok(mut n) = get_need(&nid) {
                n.coverage = NeedCoverage::Uncovered;
                n.updated_at = chrono::Utc::now().to_rfc3339();
                let _ = update_need(&n);
            }
        }
    }
    Ok(())
}

pub fn list_pending_review(limit: usize) -> AppResult<Vec<GeneratedCandidate>> {
    let conn = open_db()?;
    let limit = limit.clamp(1, 100) as i64;
    let mut stmt = conn
        .prepare(
            "SELECT id, job_id, need_id, local_path, sha256, perceptual_hash, status,
             technical_score, semantic_score, qa_decision, qa_reason, approved_asset_id,
             created_at, updated_at FROM generated_candidates
             WHERE status = 'needs_human_review' ORDER BY created_at DESC LIMIT ?1",
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map(params![limit], |r| {
            let st: String = r.get(6)?;
            Ok(GeneratedCandidate {
                id: r.get(0)?,
                job_id: r.get(1)?,
                need_id: r.get(2)?,
                local_path: r.get(3)?,
                sha256: r.get(4)?,
                perceptual_hash: r.get(5)?,
                status: CandidateStatus::parse(&st),
                technical_score: r.get(7)?,
                semantic_score: r.get(8)?,
                qa_decision: r.get(9)?,
                qa_reason: r.get(10)?,
                approved_asset_id: r.get(11)?,
                created_at: r.get(12)?,
                updated_at: r.get(13)?,
            })
        })
        .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(rows.flatten().collect())
}

/// Cover needs: match first, optionally queue generation for gaps.
pub async fn cover_project_needs(
    project_key: &str,
    generate_missing: bool,
    max_generate: u32,
) -> AppResult<serde_json::Value> {
    use crate::pipeline::visual::intelligent_match::{apply_best_match, MatchOptions};
    use crate::pipeline::visual::needs::list_needs;

    let mut needs = list_needs(project_key)?;
    let mut reused = 0u32;
    let mut queued = 0u32;
    let mut used = Vec::new();
    for n in &needs {
        if let Some(id) = &n.matched_asset_id {
            used.push(id.clone());
        }
    }

    for need in needs.iter_mut() {
        if matches!(
            need.coverage,
            NeedCoverage::Covered | NeedCoverage::Matched | NeedCoverage::Skipped
        ) {
            continue;
        }
        let opts = MatchOptions {
            used_in_project: used.clone(),
            ..Default::default()
        };
        if apply_best_match(need, &opts) {
            update_need(need)?;
            if let Some(id) = &need.matched_asset_id {
                used.push(id.clone());
            }
            reused += 1;
            continue;
        }
        if generate_missing
            && queued < max_generate
            && queue_generation_for_need(need, false)?.is_some()
        {
            queued += 1;
        }
    }

    let mut processed = 0u32;
    if generate_missing {
        processed = worker_tick(max_generate).await?;
    }

    let summary = crate::pipeline::visual::needs::coverage_for_project(project_key)?;
    Ok(serde_json::json!({
        "reused": reused,
        "queued": queued,
        "processed": processed,
        "coverage": summary,
        "needs": list_needs(project_key)?,
    }))
}
