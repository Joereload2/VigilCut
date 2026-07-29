//! Open and migrate `vnext.db` (separate from visual `library.db`).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const VNEXT_SCHEMA_VERSION: i32 = 1;
pub const VNEXT_DB_FILE: &str = "vnext.db";

static DB_ROOT_OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn override_lock() -> &'static Mutex<Option<PathBuf>> {
    DB_ROOT_OVERRIDE.get_or_init(|| Mutex::new(None))
}

/// Test/harness: force directory that will contain `vnext.db`.
pub fn set_vnext_root_override(path: Option<PathBuf>) {
    *override_lock().lock().unwrap_or_else(|e| e.into_inner()) = path;
}

pub fn vnext_root() -> AppResult<PathBuf> {
    if let Some(p) = override_lock()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
    {
        std::fs::create_dir_all(&p)?;
        return Ok(p);
    }
    let base = crate::state::AppState::app_data_dir()?;
    let root = base.join("vnext");
    std::fs::create_dir_all(&root)?;
    Ok(root)
}

pub fn vnext_db_path() -> AppResult<PathBuf> {
    Ok(vnext_root()?.join(VNEXT_DB_FILE))
}

pub fn open_vnext_db() -> AppResult<Connection> {
    let path = vnext_db_path()?;
    open_vnext_db_at(&path)
}

pub fn open_vnext_db_at(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path).map_err(|e| AppError::Message(e.to_string()))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| AppError::Message(e.to_string()))?;
    migrate(&conn)?;
    Ok(conn)
}

pub fn schema_version(conn: &Connection) -> AppResult<i32> {
    let ver: i32 = conn
        .query_row(
            "SELECT value FROM vnext_schema_meta WHERE key = 'version'",
            [],
            |r| {
                let s: String = r.get(0)?;
                Ok(s.parse::<i32>().unwrap_or(0))
            },
        )
        .unwrap_or(0);
    Ok(ver)
}

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS vnext_schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))?;

    let ver = schema_version(conn)?;
    if ver < 1 {
        migrate_v1(conn)?;
        conn.execute(
            "INSERT OR REPLACE INTO vnext_schema_meta(key,value) VALUES('version','1')",
            [],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }
    Ok(())
}

fn migrate_v1(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS content_projects (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            client_label TEXT,
            privacy_mode TEXT NOT NULL DEFAULT 'local_only',
            status TEXT NOT NULL,
            source_media_path TEXT NOT NULL,
            source_sha256 TEXT,
            source_duration_s REAL,
            source_width INTEGER,
            source_height INTEGER,
            source_has_audio INTEGER,
            source_has_video INTEGER,
            source_container TEXT,
            work_dir TEXT NOT NULL,
            active_recipe_id TEXT,
            legacy_project_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_content_projects_status ON content_projects(status);
        CREATE INDEX IF NOT EXISTS idx_content_projects_updated ON content_projects(updated_at);

        CREATE TABLE IF NOT EXISTS production_recipes (
            id TEXT PRIMARY KEY,
            content_project_id TEXT NOT NULL,
            label TEXT NOT NULL,
            recipe_json TEXT NOT NULL,
            supersedes_recipe_id TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(content_project_id) REFERENCES content_projects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_recipes_project ON production_recipes(content_project_id);

        CREATE TABLE IF NOT EXISTS short_candidates (
            id TEXT PRIMARY KEY,
            content_project_id TEXT NOT NULL,
            source_job_id TEXT NOT NULL,
            source_media_path TEXT NOT NULL,
            start_s REAL NOT NULL,
            end_s REAL NOT NULL,
            duration_s REAL NOT NULL,
            original_start_s REAL NOT NULL,
            original_end_s REAL NOT NULL,
            transcript_text TEXT NOT NULL,
            title TEXT NOT NULL,
            summary TEXT NOT NULL,
            score REAL NOT NULL,
            confidence REAL NOT NULL,
            score_breakdown_json TEXT NOT NULL,
            reasons_json TEXT NOT NULL,
            warnings_json TEXT NOT NULL,
            strengths_json TEXT NOT NULL,
            risks_json TEXT NOT NULL,
            workflow_status TEXT NOT NULL,
            variant_group_id TEXT NOT NULL,
            is_primary_variant INTEGER NOT NULL,
            framing_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(content_project_id) REFERENCES content_projects(id),
            CHECK(end_s > start_s)
        );
        CREATE INDEX IF NOT EXISTS idx_candidates_project_score ON short_candidates(content_project_id, score);
        CREATE INDEX IF NOT EXISTS idx_candidates_project_status ON short_candidates(content_project_id, workflow_status);
        CREATE INDEX IF NOT EXISTS idx_candidates_variant ON short_candidates(variant_group_id);

        CREATE TABLE IF NOT EXISTS render_plans (
            id TEXT PRIMARY KEY,
            content_project_id TEXT NOT NULL,
            recipe_id TEXT NOT NULL,
            candidate_id TEXT NOT NULL,
            plan_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            created_by TEXT NOT NULL,
            FOREIGN KEY(content_project_id) REFERENCES content_projects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_render_plans_project ON render_plans(content_project_id);

        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            content_project_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE,
            attempt INTEGER NOT NULL DEFAULT 0,
            max_attempts INTEGER NOT NULL,
            priority INTEGER NOT NULL DEFAULT 100,
            input_json TEXT NOT NULL,
            result_artifact_id TEXT,
            render_plan_id TEXT,
            error_json TEXT,
            stage TEXT NOT NULL DEFAULT '',
            progress_pct REAL NOT NULL DEFAULT 0,
            locked_by TEXT,
            lease_expires_at TEXT,
            cancel_requested INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            started_at TEXT,
            finished_at TEXT,
            FOREIGN KEY(content_project_id) REFERENCES content_projects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_jobs_project_status ON jobs(content_project_id, status);
        CREATE INDEX IF NOT EXISTS idx_jobs_status_priority ON jobs(status, priority, created_at);
        CREATE INDEX IF NOT EXISTS idx_jobs_lease ON jobs(status, lease_expires_at);

        CREATE TABLE IF NOT EXISTS artifacts (
            id TEXT PRIMARY KEY,
            content_project_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            role TEXT NOT NULL,
            path TEXT NOT NULL,
            sha256 TEXT NOT NULL,
            byte_size INTEGER NOT NULL,
            mime_type TEXT NOT NULL,
            created_by_job_id TEXT NOT NULL,
            probe_json TEXT,
            validation_status TEXT NOT NULL,
            validation_notes TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(content_project_id) REFERENCES content_projects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_artifacts_project ON artifacts(content_project_id);
        CREATE INDEX IF NOT EXISTS idx_artifacts_job ON artifacts(created_by_job_id);
        CREATE INDEX IF NOT EXISTS idx_artifacts_sha ON artifacts(sha256);

        CREATE TABLE IF NOT EXISTS artifact_parents (
            artifact_id TEXT NOT NULL,
            parent_artifact_id TEXT NOT NULL,
            PRIMARY KEY(artifact_id, parent_artifact_id),
            FOREIGN KEY(artifact_id) REFERENCES artifacts(id),
            FOREIGN KEY(parent_artifact_id) REFERENCES artifacts(id)
        );
        CREATE INDEX IF NOT EXISTS idx_artifact_parents_parent ON artifact_parents(parent_artifact_id);

        CREATE TABLE IF NOT EXISTS review_decisions (
            id TEXT PRIMARY KEY,
            content_project_id TEXT NOT NULL,
            target_kind TEXT NOT NULL,
            target_id TEXT NOT NULL,
            decision TEXT NOT NULL,
            reason TEXT,
            payload_json TEXT NOT NULL,
            actor TEXT NOT NULL,
            related_job_id TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY(content_project_id) REFERENCES content_projects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_decisions_target ON review_decisions(target_kind, target_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_decisions_project ON review_decisions(content_project_id, created_at);
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_root(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vc-vnext-{}-{}", label, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn migrate_empty_and_idempotent() {
        // Avoid path override races with application tests: open by absolute path.
        let root = temp_root("mig");
        let path = root.join("vnext.db");
        let conn = open_vnext_db_at(&path).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), VNEXT_SCHEMA_VERSION);
        migrate(&conn).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 1);

        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='jobs'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        assert!(!root.join("library.db").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn migrate_twice_on_fresh_file() {
        let root = temp_root("mig2");
        let path = root.join("vnext.db");
        {
            let c = open_vnext_db_at(&path).unwrap();
            assert_eq!(schema_version(&c).unwrap(), 1);
        }
        {
            let c = open_vnext_db_at(&path).unwrap();
            assert_eq!(schema_version(&c).unwrap(), 1);
        }
        let _ = std::fs::remove_dir_all(root);
    }
}
