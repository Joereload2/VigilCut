//! Aggregated operational snapshot for the global visual library.

use serde::Serialize;

use crate::error::AppResult;
use crate::models::visual_intel::CostPolicy;
use crate::pipeline::visual::generation::cost::daily_generation_count;
use crate::pipeline::visual::generation::provider::select_provider;
use crate::pipeline::visual::library::open_db;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryInventory {
    pub total_assets: u32,
    pub active_assets: u32,
    pub missing_assets: u32,
    pub pending_candidates: u32,
    pub managed_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryCoverage {
    pub total_concepts: u32,
    pub covered_concepts: u32,
    pub partial_concepts: u32,
    pub uncovered_concepts: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryActivity {
    pub queued: u32,
    pub running: u32,
    pub cancelling: u32,
    pub failed_today: u32,
    pub last_cycle_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryProviderStatus {
    pub name: String,
    pub model: Option<String>,
    pub configured: bool,
    pub reachable: bool,
    pub supports_image: bool,
    pub free_verified: bool,
    pub cost_kind: String,
    pub last_checked_at: Option<String>,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryLimits {
    pub local_daily_limit: u32,
    pub local_used_today: u32,
    pub local_remaining_today: u32,
    pub max_attempts_per_concept: u32,
    pub paid_providers_enabled: bool,
    pub provider_limit: Option<u32>,
    pub provider_used: Option<u32>,
    pub provider_remaining: Option<u32>,
    pub provider_reset_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryDashboard {
    pub inventory: LibraryInventory,
    pub coverage: LibraryCoverage,
    pub activity: LibraryActivity,
    pub provider: LibraryProviderStatus,
    pub limits: LibraryLimits,
    pub can_work: bool,
    pub blocked_reason: Option<String>,
}

fn count(conn: &rusqlite::Connection, sql: &str) -> u32 {
    conn.query_row(sql, [], |r| r.get::<_, i64>(0))
        .unwrap_or(0)
        .max(0) as u32
}

pub fn dashboard() -> AppResult<LibraryDashboard> {
    let conn = open_db()?;
    let inventory = LibraryInventory {
        total_assets: count(
            &conn,
            "SELECT COUNT(*) FROM media_assets WHERE COALESCE(status,'active') != 'invalid'",
        ),
        active_assets: count(
            &conn,
            "SELECT COUNT(*) FROM media_assets WHERE status='active'",
        ),
        missing_assets: count(
            &conn,
            "SELECT COUNT(*) FROM media_assets WHERE status='missing'",
        ),
        pending_candidates: count(
            &conn,
            "SELECT COUNT(*) FROM generated_candidates
             WHERE status IN ('generated','automated_review','needs_human_review')",
        ),
        managed_bytes: conn
            .query_row(
                "SELECT COALESCE(SUM(file_size),0) FROM media_assets
                 WHERE COALESCE(status,'active') != 'invalid'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            .max(0) as u64,
    };

    let total_concepts = count(
        &conn,
        "SELECT COUNT(*) FROM visual_concepts WHERE status IN ('active','priority')",
    );
    let covered_concepts = count(
        &conn,
        "SELECT COUNT(DISTINCT c.id)
         FROM visual_concepts c
         JOIN asset_concepts ac ON ac.concept_id=c.id
         JOIN media_assets a ON a.id=ac.asset_id
         WHERE c.status IN ('active','priority')
           AND a.status='active'
           AND COALESCE(a.qa_status,'none') != 'rejected'",
    );
    let coverage = LibraryCoverage {
        total_concepts,
        covered_concepts,
        partial_concepts: 0,
        uncovered_concepts: total_concepts.saturating_sub(covered_concepts),
    };

    let activity = LibraryActivity {
        queued: count(
            &conn,
            "SELECT COUNT(*) FROM generation_jobs WHERE status='queued'",
        ),
        running: count(
            &conn,
            "SELECT COUNT(*) FROM generation_jobs WHERE status='running'",
        ),
        cancelling: count(
            &conn,
            "SELECT COUNT(*) FROM generation_jobs
             WHERE status='running' AND COALESCE(cancel_requested,0)=1",
        ),
        failed_today: count(
            &conn,
            "SELECT COUNT(*) FROM generation_jobs
             WHERE status='failed' AND date(updated_at)=date('now')",
        ),
        last_cycle_at: conn
            .query_row(
                "SELECT last_cycle_at FROM daily_feed_settings WHERE id=1",
                [],
                |r| r.get(0),
            )
            .ok(),
    };

    let policy = CostPolicy::from_env();
    let used = daily_generation_count().unwrap_or(0);
    let limits = LibraryLimits {
        local_daily_limit: policy.max_daily_generations,
        local_used_today: used,
        local_remaining_today: policy.max_daily_generations.saturating_sub(used),
        max_attempts_per_concept: policy.max_attempts_per_need,
        paid_providers_enabled: policy.paid_providers_enabled,
        provider_limit: None,
        provider_used: None,
        provider_remaining: None,
        provider_reset_at: None,
    };

    let selected = select_provider(policy.paid_providers_enabled);
    let provider_name = selected.name().to_string();
    let configured = provider_name == "mock"
        || std::env::var("OMNIROUTE_BASE_URL")
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
    type ProbeRow = (
        String,
        bool,
        bool,
        bool,
        Option<String>,
        Option<u64>,
        Option<String>,
    );
    let latest: Option<ProbeRow> = conn
        .query_row(
            "SELECT model, COALESCE(last_probe_ok,0), COALESCE(supports_image,0),
                    COALESCE(free_tier,0), last_probe_at, latency_ms, last_error
             FROM provider_capabilities WHERE provider=?1
             ORDER BY last_probe_at DESC LIMIT 1",
            rusqlite::params![provider_name],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get::<_, i64>(1)? != 0,
                    r.get::<_, i64>(2)? != 0,
                    r.get::<_, i64>(3)? != 0,
                    r.get(4)?,
                    r.get::<_, Option<i64>>(5)?.map(|n| n.max(0) as u64),
                    r.get(6)?,
                ))
            },
        )
        .ok();

    let provider = if provider_name == "mock" {
        LibraryProviderStatus {
            name: provider_name,
            model: Some("fixture".into()),
            configured: true,
            reachable: true,
            supports_image: true,
            free_verified: true,
            cost_kind: "local".into(),
            last_checked_at: latest.as_ref().and_then(|v| v.4.clone()),
            latency_ms: latest.as_ref().and_then(|v| v.5),
            error: None,
        }
    } else {
        let (model, reachable, supports_image, free_tier, checked, latency, error) = latest
            .unwrap_or_else(|| {
                (
                    String::new(),
                    false,
                    false,
                    false,
                    None,
                    None,
                    Some("Proveedor todavía no comprobado".into()),
                )
            });
        LibraryProviderStatus {
            name: provider_name,
            model: (!model.is_empty()).then_some(model),
            configured,
            reachable,
            supports_image,
            free_verified: false,
            cost_kind: if free_tier {
                "free_configured".into()
            } else {
                "unknown".into()
            },
            last_checked_at: checked,
            latency_ms: latency,
            error,
        }
    };

    let (can_work, blocked_reason) = if limits.local_remaining_today == 0 {
        (false, Some("Límite local diario alcanzado".into()))
    } else if provider.name == "mock" {
        (true, None)
    } else if !provider.configured {
        (false, Some("OmniRoute no está configurado".into()))
    } else if !provider.reachable {
        (false, Some("Proveedor no disponible".into()))
    } else if !provider.supports_image {
        (
            false,
            Some("La capacidad de generación de imágenes no está verificada".into()),
        )
    } else if !provider.free_verified && !policy.paid_providers_enabled {
        (
            false,
            Some("El coste gratuito de OmniRoute no está verificado".into()),
        )
    } else {
        (true, None)
    };

    Ok(LibraryDashboard {
        inventory,
        coverage,
        activity,
        provider,
        limits,
        can_work,
        blocked_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::library::{lock_library_for_test, set_library_root_override};

    #[test]
    fn empty_dashboard_is_safe_and_mock_can_work() {
        let _lock = lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-dashboard-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
        let d = dashboard().unwrap();
        assert_eq!(d.inventory.total_assets, 0);
        assert_eq!(d.coverage.total_concepts, 0);
        assert!(d.provider.free_verified);
        assert!(d.can_work);
        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pending_candidate_does_not_count_as_asset() {
        let _lock = lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-dashboard-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let conn = open_db().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO generation_jobs(
                id,idempotency_key,status,prompt,negative_prompt,max_attempts,
                is_paid,opportunistic,created_at,updated_at
             ) VALUES('j','dash:j','succeeded','','',1,0,0,?1,?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO generated_candidates(id,job_id,status,created_at,updated_at)
             VALUES('c','j','needs_human_review',?1,?1)",
            rusqlite::params![now],
        )
        .unwrap();
        let d = dashboard().unwrap();
        assert_eq!(d.inventory.total_assets, 0);
        assert_eq!(d.inventory.pending_candidates, 1);
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
