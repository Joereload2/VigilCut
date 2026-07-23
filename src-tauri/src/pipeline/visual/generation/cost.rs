//! Hard cost gates — never spend paid APIs silently.

use rusqlite::params;

use crate::error::{AppError, AppResult};
use crate::models::visual_intel::CostPolicy;
use crate::pipeline::visual::library::open_db;

#[derive(Debug, Clone)]
pub enum CostGate {
    Allow { free: bool },
    Deny { reason: String },
}

pub fn can_enqueue_generation(
    policy: &CostPolicy,
    project_key: &str,
    is_paid: bool,
    opportunistic: bool,
) -> AppResult<CostGate> {
    if opportunistic && !policy.opportunistic_enabled {
        return Ok(CostGate::Deny {
            reason: "generación oportunista desactivada".into(),
        });
    }
    if is_paid {
        if !policy.paid_providers_enabled {
            return Ok(CostGate::Deny {
                reason: "proveedores de pago deshabilitados (VIGILCUT_PAID_PROVIDERS)".into(),
            });
        }
        if policy.daily_paid_budget <= 0.0 {
            return Ok(CostGate::Deny {
                reason: "presupuesto diario de pago es 0".into(),
            });
        }
    }

    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let conn = open_db()?;
    let gens: i64 = conn
        .query_row(
            "SELECT generations FROM cost_counters WHERE day = ?1",
            params![day],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if gens as u32 >= policy.max_daily_generations {
        return Ok(CostGate::Deny {
            reason: format!(
                "límite diario de generaciones ({})",
                policy.max_daily_generations
            ),
        });
    }

    // Per-project: count jobs for project via needs
    let proj_jobs: i64 = conn
        .query_row(
            r#"SELECT COUNT(*) FROM generation_jobs j
               LEFT JOIN visual_needs n ON n.id = j.need_id
               WHERE n.project_key = ?1 AND j.status IN ('queued','running','succeeded')"#,
            params![project_key],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if proj_jobs as u32 >= policy.max_generations_per_project {
        return Ok(CostGate::Deny {
            reason: format!(
                "límite de generaciones del proyecto ({})",
                policy.max_generations_per_project
            ),
        });
    }

    Ok(CostGate::Allow { free: !is_paid })
}

pub fn increment_generation_counter() -> AppResult<()> {
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let conn = open_db()?;
    conn.execute(
        r#"INSERT INTO cost_counters(day, generations, paid_spend) VALUES(?1, 1, 0)
           ON CONFLICT(day) DO UPDATE SET generations = generations + 1"#,
        params![day],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

pub fn daily_generation_count() -> AppResult<u32> {
    let day = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let conn = open_db()?;
    let n: i64 = conn
        .query_row(
            "SELECT generations FROM cost_counters WHERE day = ?1",
            params![day],
            |r| r.get(0),
        )
        .unwrap_or(0);
    Ok(n as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::library::set_library_root_override;

    #[test]
    fn paid_blocked_by_default() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-cost-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let policy = CostPolicy::default();
        assert!(!policy.paid_providers_enabled);
        let g = can_enqueue_generation(&policy, "proj", true, false).unwrap();
        assert!(matches!(g, CostGate::Deny { .. }));
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
