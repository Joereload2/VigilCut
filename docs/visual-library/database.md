# Database — local SQLite (runtime)

## Local (`%APPDATA%/VigilCut/library/library.db`)

- Migrated by `pipeline/visual/schema.rs` (`SCHEMA_VERSION = 4`)
- Tables: `media_assets` (+ intel columns), `asset_usage`, `themes`, `visual_concepts`, `asset_concepts`, `visual_needs`, `generation_jobs` (lease/stage/origin), `generated_candidates`, `qa_checks`, `provider_capabilities`, `cost_counters`, `daily_feed_settings`, `daily_metrics`
- Files: `library/assets/`, `library/thumbs/`, `library/candidates/`
- Worker lease columns (v4): `locked_by`, `lease_expires_at`, `attempt_version`

## Supabase / remote — **out of scope for this release**

> **Status:** design-only. There is **no** Supabase client, Storage sync, or runtime remote path in the desktop app.

- SQL under `supabase/migrations/` is a **future** sketch for multi-device catalog sync.
- RLS/Storage policies in that migration are **incomplete** and must not be applied to production as-is (see Codex CRIT-007 / HIGH-006).
- Delivery criteria for visual library **do not** require remote sync.
- When product prioritizes cloud: implement a single `SyncService` + outbox, complete RLS/Storage isolation by `owner_id`/`workspace_id`, and add local integration tests. Until then, treat Supabase as non-shipping design.

## Sync strategy (future)

1. Desktop works offline on SQLite always  
2. Optional future client would push approved assets + concepts; pull catalog  
3. Dedup by `sha256` / `idempotency_key`  
4. No Base64 in Postgres  

## Rollback

- Local: keep previous DB file; schema only ADDs columns/tables  
- Remote (if ever deployed): new migration to drop new objects; never rewrite storage.objects internals  
