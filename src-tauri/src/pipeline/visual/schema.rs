//! Versioned local SQLite migrations for the intelligent visual library.

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

pub const SCHEMA_VERSION: i32 = 5;

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))?;

    let ver: i32 = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| {
                let s: String = r.get(0)?;
                Ok(s.parse::<i32>().unwrap_or(0))
            },
        )
        .unwrap_or(0);

    if ver < 1 {
        // v1 base tables may already exist from library.rs CREATE IF NOT EXISTS
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(key,value) VALUES('version','1')",
            [],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }

    let ver: i32 = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| {
                let s: String = r.get(0)?;
                Ok(s.parse::<i32>().unwrap_or(0))
            },
        )
        .unwrap_or(1);

    if ver < 2 {
        migrate_v2(conn)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(key,value) VALUES('version','2')",
            [],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }

    let ver: i32 = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| {
                let s: String = r.get(0)?;
                Ok(s.parse::<i32>().unwrap_or(0))
            },
        )
        .unwrap_or(2);

    if ver < 3 {
        migrate_v3(conn)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(key,value) VALUES('version','3')",
            [],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }

    let ver: i32 = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| {
                let s: String = r.get(0)?;
                Ok(s.parse::<i32>().unwrap_or(0))
            },
        )
        .unwrap_or(3);

    if ver < 4 {
        migrate_v4(conn)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(key,value) VALUES('version','4')",
            [],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }

    let ver: i32 = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'version'",
            [],
            |r| {
                let s: String = r.get(0)?;
                Ok(s.parse::<i32>().unwrap_or(0))
            },
        )
        .unwrap_or(4);

    if ver < 5 {
        migrate_v5(conn)?;
        conn.execute(
            "INSERT OR REPLACE INTO schema_meta(key,value) VALUES('version','5')",
            [],
        )
        .map_err(|e| AppError::Message(e.to_string()))?;
    }

    Ok(())
}

fn migrate_v5(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS library_requests (
            id TEXT PRIMARY KEY,
            concept_id TEXT NOT NULL,
            title TEXT NOT NULL,
            target_count INTEGER NOT NULL,
            desired_format TEXT NOT NULL DEFAULT '16:9',
            positive_contexts TEXT NOT NULL DEFAULT '[]',
            negative_contexts TEXT NOT NULL DEFAULT '[]',
            hard_exclusions TEXT NOT NULL DEFAULT '[]',
            priority INTEGER NOT NULL DEFAULT 50,
            status TEXT NOT NULL DEFAULT 'draft',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_library_requests_status
            ON library_requests(status, created_at DESC);
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))
}

/// Lease/recovery columns for Codex CRIT-002 / HIGH-004.
fn migrate_v4(conn: &Connection) -> AppResult<()> {
    let alters = [
        "ALTER TABLE generation_jobs ADD COLUMN locked_by TEXT",
        "ALTER TABLE generation_jobs ADD COLUMN lease_expires_at TEXT",
        "ALTER TABLE generation_jobs ADD COLUMN attempt_version INTEGER NOT NULL DEFAULT 1",
        "ALTER TABLE generated_candidates ADD COLUMN concept_title TEXT",
        "ALTER TABLE generated_candidates ADD COLUMN need_label TEXT",
    ];
    for sql in alters {
        let _ = conn.execute(sql, []);
    }
    // Recover any stuck running jobs from previous process
    let _ = conn.execute(
        "UPDATE generation_jobs SET status='queued', stage='queued', locked_by=NULL, lease_expires_at=NULL,
         last_error=COALESCE(last_error,'') || ' [requeued after restart]',
         updated_at=datetime('now')
         WHERE status='running'",
        [],
    );
    Ok(())
}

fn migrate_v3(conn: &Connection) -> AppResult<()> {
    let alters = [
        "ALTER TABLE generation_jobs ADD COLUMN stage TEXT NOT NULL DEFAULT 'queued'",
        "ALTER TABLE generation_jobs ADD COLUMN cancel_requested INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE generation_jobs ADD COLUMN cost_kind TEXT NOT NULL DEFAULT 'unknown'",
        "ALTER TABLE generation_jobs ADD COLUMN free_verified INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE generation_jobs ADD COLUMN prompt_strategy TEXT",
        "ALTER TABLE generation_jobs ADD COLUMN origin TEXT NOT NULL DEFAULT 'video_need'",
        "ALTER TABLE generated_candidates ADD COLUMN origin TEXT NOT NULL DEFAULT 'video_need'",
        "ALTER TABLE generated_candidates ADD COLUMN reject_reason TEXT",
        "ALTER TABLE generated_candidates ADD COLUMN width INTEGER",
        "ALTER TABLE generated_candidates ADD COLUMN height INTEGER",
        "ALTER TABLE generated_candidates ADD COLUMN mime_type TEXT",
        "ALTER TABLE generated_candidates ADD COLUMN cost_kind TEXT",
        "ALTER TABLE generated_candidates ADD COLUMN free_verified INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE generated_candidates ADD COLUMN provider TEXT",
        "ALTER TABLE generated_candidates ADD COLUMN model TEXT",
    ];
    for sql in alters {
        let _ = conn.execute(sql, []);
    }
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS daily_feed_settings (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            enabled INTEGER NOT NULL DEFAULT 0,
            max_per_day INTEGER NOT NULL DEFAULT 5,
            interval_minutes INTEGER NOT NULL DEFAULT 30,
            last_cycle_at TEXT,
            consecutive_failures INTEGER NOT NULL DEFAULT 0,
            paused_until TEXT,
            updated_at TEXT NOT NULL
        );
        INSERT OR IGNORE INTO daily_feed_settings(id, enabled, max_per_day, interval_minutes, consecutive_failures, updated_at)
        VALUES (1, 0, 5, 30, 0, datetime('now'));

        CREATE TABLE IF NOT EXISTS daily_metrics (
            day TEXT PRIMARY KEY,
            checks INTEGER NOT NULL DEFAULT 0,
            free_routes INTEGER NOT NULL DEFAULT 0,
            attempts INTEGER NOT NULL DEFAULT 0,
            approved INTEGER NOT NULL DEFAULT 0,
            rejected INTEGER NOT NULL DEFAULT 0,
            needs_review INTEGER NOT NULL DEFAULT 0,
            rate_limits INTEGER NOT NULL DEFAULT 0,
            failures INTEGER NOT NULL DEFAULT 0,
            reused INTEGER NOT NULL DEFAULT 0,
            concepts_covered INTEGER NOT NULL DEFAULT 0,
            paid_spend REAL NOT NULL DEFAULT 0
        );
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

fn migrate_v2(conn: &Connection) -> AppResult<()> {
    // Extended columns on media_assets (additive, defaults for legacy rows)
    let alters = [
        "ALTER TABLE media_assets ADD COLUMN meanings TEXT NOT NULL DEFAULT '[]'",
        "ALTER TABLE media_assets ADD COLUMN positive_contexts TEXT NOT NULL DEFAULT '[]'",
        "ALTER TABLE media_assets ADD COLUMN negative_contexts TEXT NOT NULL DEFAULT '[]'",
        "ALTER TABLE media_assets ADD COLUMN hard_exclusions TEXT NOT NULL DEFAULT '[]'",
        "ALTER TABLE media_assets ADD COLUMN aspect_ratio TEXT",
        "ALTER TABLE media_assets ADD COLUMN safe_area TEXT DEFAULT 'center'",
        "ALTER TABLE media_assets ADD COLUMN perceptual_hash TEXT",
        "ALTER TABLE media_assets ADD COLUMN qa_status TEXT NOT NULL DEFAULT 'none'",
        "ALTER TABLE media_assets ADD COLUMN technical_score REAL",
        "ALTER TABLE media_assets ADD COLUMN semantic_score REAL",
        "ALTER TABLE media_assets ADD COLUMN provenance_json TEXT",
        "ALTER TABLE media_assets ADD COLUMN commercial_use INTEGER",
        "ALTER TABLE media_assets ADD COLUMN literal_description TEXT NOT NULL DEFAULT '[]'",
    ];
    for sql in alters {
        let _ = conn.execute(sql, []);
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS themes (
            id TEXT PRIMARY KEY,
            slug TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS visual_concepts (
            id TEXT PRIMARY KEY,
            canonical_key TEXT NOT NULL UNIQUE,
            theme_id TEXT,
            title TEXT NOT NULL,
            literal_description TEXT NOT NULL DEFAULT '[]',
            meanings TEXT NOT NULL DEFAULT '[]',
            positive_contexts TEXT NOT NULL DEFAULT '[]',
            negative_contexts TEXT NOT NULL DEFAULT '[]',
            hard_exclusions TEXT NOT NULL DEFAULT '[]',
            desired_formats TEXT NOT NULL DEFAULT '["16:9"]',
            priority INTEGER NOT NULL DEFAULT 50,
            request_count INTEGER NOT NULL DEFAULT 0,
            coverage_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_concepts_status ON visual_concepts(status);
        CREATE INDEX IF NOT EXISTS idx_concepts_priority ON visual_concepts(priority DESC);

        CREATE TABLE IF NOT EXISTS asset_concepts (
            asset_id TEXT NOT NULL,
            concept_id TEXT NOT NULL,
            weight REAL NOT NULL DEFAULT 1.0,
            PRIMARY KEY (asset_id, concept_id)
        );

        CREATE TABLE IF NOT EXISTS visual_needs (
            id TEXT PRIMARY KEY,
            project_key TEXT NOT NULL,
            media_path TEXT,
            semantic_event_id TEXT,
            concept_id TEXT,
            label TEXT NOT NULL,
            terms TEXT NOT NULL DEFAULT '[]',
            required_contexts TEXT NOT NULL DEFAULT '[]',
            forbidden_contexts TEXT NOT NULL DEFAULT '[]',
            hard_exclusions TEXT NOT NULL DEFAULT '[]',
            desired_aspect TEXT NOT NULL DEFAULT '16:9',
            approx_duration_secs REAL NOT NULL DEFAULT 5.0,
            source_start REAL,
            source_end REAL,
            output_start REAL,
            output_end REAL,
            priority INTEGER NOT NULL DEFAULT 50,
            coverage TEXT NOT NULL DEFAULT 'uncovered',
            matched_asset_id TEXT,
            match_score REAL,
            match_reasons TEXT NOT NULL DEFAULT '[]',
            generation_job_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_needs_project ON visual_needs(project_key);
        CREATE INDEX IF NOT EXISTS idx_needs_coverage ON visual_needs(coverage);

        CREATE TABLE IF NOT EXISTS generation_jobs (
            id TEXT PRIMARY KEY,
            idempotency_key TEXT NOT NULL UNIQUE,
            need_id TEXT,
            concept_id TEXT,
            status TEXT NOT NULL DEFAULT 'queued',
            provider TEXT,
            model TEXT,
            prompt TEXT NOT NULL DEFAULT '',
            negative_prompt TEXT NOT NULL DEFAULT '',
            attempt INTEGER NOT NULL DEFAULT 0,
            max_attempts INTEGER NOT NULL DEFAULT 2,
            last_error TEXT,
            is_paid INTEGER NOT NULL DEFAULT 0,
            opportunistic INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_jobs_status ON generation_jobs(status);

        CREATE TABLE IF NOT EXISTS generated_candidates (
            id TEXT PRIMARY KEY,
            job_id TEXT NOT NULL,
            need_id TEXT,
            local_path TEXT,
            sha256 TEXT,
            perceptual_hash TEXT,
            status TEXT NOT NULL DEFAULT 'generated',
            technical_score REAL,
            semantic_score REAL,
            qa_decision TEXT,
            qa_reason TEXT,
            approved_asset_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_candidates_job ON generated_candidates(job_id);
        CREATE INDEX IF NOT EXISTS idx_candidates_status ON generated_candidates(status);

        CREATE TABLE IF NOT EXISTS qa_checks (
            id TEXT PRIMARY KEY,
            candidate_id TEXT,
            asset_id TEXT,
            technical_quality REAL NOT NULL,
            semantic_alignment REAL NOT NULL,
            forbidden_detected TEXT NOT NULL DEFAULT '[]',
            text_detected INTEGER NOT NULL DEFAULT 0,
            watermark_detected INTEGER NOT NULL DEFAULT 0,
            decision TEXT NOT NULL,
            reason TEXT NOT NULL,
            details TEXT NOT NULL DEFAULT '{}',
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS provider_capabilities (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            supports_image INTEGER NOT NULL DEFAULT 0,
            free_tier INTEGER NOT NULL DEFAULT 1,
            last_probe_ok INTEGER NOT NULL DEFAULT 0,
            last_probe_at TEXT,
            last_error TEXT,
            latency_ms INTEGER,
            notes TEXT,
            UNIQUE(provider, model)
        );

        CREATE TABLE IF NOT EXISTS cost_counters (
            day TEXT PRIMARY KEY,
            generations INTEGER NOT NULL DEFAULT 0,
            paid_spend REAL NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS sync_queue (
            id TEXT PRIMARY KEY,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            attempt INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            next_attempt_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(entity_type, entity_id, operation)
        );
        CREATE INDEX IF NOT EXISTS idx_sync_queue_ready
          ON sync_queue(status, next_attempt_at, created_at);
        "#,
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrate_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        // minimal media_assets like production
        conn.execute_batch(
            "CREATE TABLE media_assets (
                id TEXT PRIMARY KEY, kind TEXT, managed_path TEXT, thumbnail_path TEXT,
                sha256 TEXT UNIQUE, title TEXT, description TEXT, tags TEXT, concepts TEXT,
                category TEXT, width INTEGER, height INTEGER, orientation TEXT, mime_type TEXT,
                file_size INTEGER, license_status TEXT, source TEXT, attribution TEXT,
                times_used INTEGER, last_used_at TEXT, allow_same_video_repeat INTEGER,
                minimum_videos_before_reuse INTEGER, quality_score REAL, status TEXT,
                original_path TEXT, created_at TEXT, updated_at TEXT
            );",
        )
        .unwrap();
        migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        let v: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key='version'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(v, "5");
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM visual_concepts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
