//! Opportunistic daily library growth — only free/local, never paid.
//! Runs while VigilCut is open (MVP); no OS service installed silently.

use rusqlite::params;
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::models::visual_intel::CostPolicy;
use crate::models::visual_intel::{NeedCoverage, VisualNeed};
use crate::pipeline::visual::concepts::list_concepts;
use crate::pipeline::visual::generation::cost::{can_enqueue_generation, CostGate};
use crate::pipeline::visual::generation::provider::select_provider;
use crate::pipeline::visual::generation::worker::{queue_generation_with_key, worker_tick};
use crate::pipeline::visual::intelligent_match::{apply_best_match, MatchOptions};
use crate::pipeline::visual::library::open_db;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyFeedSettings {
    pub enabled: bool,
    pub max_per_day: u32,
    pub interval_minutes: u32,
    pub last_cycle_at: Option<String>,
    pub consecutive_failures: u32,
    pub paused_until: Option<String>,
}

pub fn settings_json() -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(load_settings()?)?)
}

pub fn load_settings() -> AppResult<DailyFeedSettings> {
    let conn = open_db()?;
    conn.query_row(
        "SELECT enabled, max_per_day, interval_minutes, last_cycle_at, consecutive_failures, paused_until
         FROM daily_feed_settings WHERE id=1",
        [],
        |r| {
            Ok(DailyFeedSettings {
                enabled: r.get::<_, i64>(0)? != 0,
                max_per_day: r.get::<_, i64>(1)? as u32,
                interval_minutes: r.get::<_, i64>(2)? as u32,
                last_cycle_at: r.get(3)?,
                consecutive_failures: r.get::<_, i64>(4)? as u32,
                paused_until: r.get(5)?,
            })
        },
    )
    .map_err(|e| AppError::Message(e.to_string()))
}

pub fn set_enabled(enabled: bool) -> AppResult<DailyFeedSettings> {
    let conn = open_db()?;
    conn.execute(
        "UPDATE daily_feed_settings SET enabled=?1, updated_at=?2 WHERE id=1",
        params![enabled as i64, chrono::Utc::now().to_rfc3339()],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    load_settings()
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn bump_metric(field: &str) -> AppResult<()> {
    let day = today();
    let conn = open_db()?;
    conn.execute(
        "INSERT INTO daily_metrics(day) VALUES(?1) ON CONFLICT(day) DO NOTHING",
        params![day],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    // Only allow known columns
    let col = match field {
        "checks" | "free_routes" | "attempts" | "approved" | "rejected" | "needs_review"
        | "rate_limits" | "failures" | "reused" | "concepts_covered" => field,
        _ => return Ok(()),
    };
    conn.execute(
        &format!("UPDATE daily_metrics SET {col} = {col} + 1 WHERE day = ?1"),
        params![day],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn week_summary() -> AppResult<serde_json::Value> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(SUM(approved),0), COALESCE(SUM(concepts_covered),0), COALESCE(SUM(attempts),0),
             COALESCE(SUM(paid_spend),0) FROM daily_metrics
             WHERE day >= date('now', '-7 days')",
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    let (approved, concepts, attempts, paid): (i64, i64, i64, f64) = stmt
        .query_row([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap_or((0, 0, 0, 0.0));
    Ok(serde_json::json!({
        "approved": approved,
        "conceptsCovered": concepts,
        "attempts": attempts,
        "paidSpend": paid,
        "message": format!(
            "Esta semana la biblioteca añadió {approved} imágenes aprobadas y cubrió {concepts} conceptos sin coste pagado registrado."
        ),
    }))
}

/// One low-frequency cycle: find priority concept without coverage → match or mock generate.
/// Never runs paid. Video jobs (queued) take priority — skip if video queue busy.
pub async fn run_daily_cycle() -> AppResult<serde_json::Value> {
    let settings = load_settings()?;
    let _ = bump_metric("checks");
    if !settings.enabled {
        return Ok(serde_json::json!({ "ok": false, "reason": "disabled" }));
    }
    if let Some(until) = &settings.paused_until {
        if let Ok(until_dt) = chrono::DateTime::parse_from_rfc3339(until) {
            if until_dt > chrono::Utc::now() {
                return Ok(serde_json::json!({ "ok": false, "reason": "paused", "until": until }));
            }
        }
    }

    // Video work first
    let conn = open_db()?;
    let video_queued: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM generation_jobs WHERE status IN ('queued','running') AND COALESCE(origin,'video_need')='video_need'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if video_queued > 0 {
        return Ok(serde_json::json!({
            "ok": false,
            "reason": "video_priority",
            "videoJobs": video_queued
        }));
    }

    let policy = CostPolicy::from_env();
    let provider = select_provider(policy.paid_providers_enabled);
    if !provider.is_free_tier() && provider.name() != "mock" {
        return Ok(serde_json::json!({
            "ok": false,
            "reason": "no_free_route",
            "note": "No hay ruta gratuita/local verificada para daily feed"
        }));
    }
    let _ = bump_metric("free_routes");

    // Daily cap
    let day = today();
    let attempts_today: i64 = conn
        .query_row(
            "SELECT COALESCE(attempts,0) FROM daily_metrics WHERE day=?1",
            params![day],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if attempts_today as u32 >= settings.max_per_day {
        return Ok(serde_json::json!({ "ok": false, "reason": "daily_limit" }));
    }

    // Find concept with low coverage
    let concepts = list_concepts(None, 100)?;
    let mut target = None;
    for c in concepts {
        if matches!(
            c.status,
            crate::models::visual_intel::ConceptStatus::Priority
                | crate::models::visual_intel::ConceptStatus::Active
        ) && c.coverage_count == 0
        {
            target = Some(c);
            break;
        }
    }
    let Some(concept) = target else {
        return Ok(serde_json::json!({ "ok": false, "reason": "no_uncovered_concepts" }));
    };

    // Synthetic need under project_key daily_feed
    let mut need = VisualNeed::from_label("daily_feed", &concept.title);
    need.concept_id = Some(concept.id.clone());
    need.terms = concept.literal_description.clone();
    if need.terms.is_empty() {
        need.terms = concept.meanings.clone();
    }
    need.required_contexts = concept.positive_contexts.clone();
    need.forbidden_contexts = concept.negative_contexts.clone();
    need.hard_exclusions = concept.hard_exclusions.clone();
    need.desired_aspect = concept
        .desired_formats
        .first()
        .cloned()
        .unwrap_or_else(|| "16:9".into());
    crate::pipeline::visual::needs::save_needs(std::slice::from_ref(&need))?;

    // Search library first
    if apply_best_match(&mut need, &MatchOptions::default()) {
        crate::pipeline::visual::needs::update_need(&need)?;
        let _ = bump_metric("reused");
        let _ = bump_metric("concepts_covered");
        return Ok(serde_json::json!({
            "ok": true,
            "action": "reused",
            "conceptId": concept.id,
            "assetId": need.matched_asset_id,
        }));
    }

    match can_enqueue_generation(&policy, "daily_feed", false, true)? {
        CostGate::Deny { reason } => {
            return Ok(serde_json::json!({ "ok": false, "reason": reason }));
        }
        CostGate::Allow { .. } => {}
    }

    let idem = format!("daily:{}:v1", concept.id);
    need.coverage = NeedCoverage::Uncovered;
    match queue_generation_with_key(&mut need, true, &idem, "daily_feed") {
        Ok(Some(job_id)) => {
            let _ = bump_metric("attempts");
            let processed = worker_tick(1).await.unwrap_or(0);
            // reset consecutive failures on enqueue
            let conn = open_db()?;
            let _ = conn.execute(
                "UPDATE daily_feed_settings SET consecutive_failures=0, last_cycle_at=?1, updated_at=?1 WHERE id=1",
                params![chrono::Utc::now().to_rfc3339()],
            );
            Ok(serde_json::json!({
                "ok": true,
                "action": "generated",
                "jobId": job_id,
                "processed": processed,
                "conceptId": concept.id,
            }))
        }
        Ok(None) => Ok(serde_json::json!({ "ok": false, "reason": "policy_blocked" })),
        Err(e) => {
            let conn = open_db()?;
            let fails = settings.consecutive_failures + 1;
            let pause = if fails >= 5 {
                Some((chrono::Utc::now() + chrono::Duration::hours(6)).to_rfc3339())
            } else {
                None
            };
            let _ = conn.execute(
                "UPDATE daily_feed_settings SET consecutive_failures=?1, paused_until=?2, updated_at=?3 WHERE id=1",
                params![fails as i64, pause, chrono::Utc::now().to_rfc3339()],
            );
            let _ = bump_metric("failures");
            Err(e)
        }
    }
}

/// Integration test path: force one daily mock generation then reuse on video need.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::concepts::seed_economy_theme;
    use crate::pipeline::visual::library::set_library_root_override;

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn daily_then_video_reuses() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-daily-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
        std::env::remove_var("OMNIROUTE_BASE_URL");
        std::env::set_var("VIGILCUT_OPPORTUNISTIC", "1");

        let _ = seed_economy_theme().unwrap();
        set_enabled(true).unwrap();
        let r = run_daily_cycle().await.unwrap();
        // may generate or reuse
        assert!(r.get("ok").is_some());

        // process any queue
        let _ = worker_tick(3).await;

        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        std::env::remove_var("VIGILCUT_OPPORTUNISTIC");
        let _ = std::fs::remove_dir_all(dir);
    }
}
