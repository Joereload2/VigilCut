# Database — local SQLite + optional Supabase

## Local (`%APPDATA%/VigilCut/library/library.db`)

- Migrated by `pipeline/visual/schema.rs` (`SCHEMA_VERSION = 2`)
- Tables: `media_assets` (+ intel columns), `asset_usage`, `themes`, `visual_concepts`, `asset_concepts`, `visual_needs`, `generation_jobs`, `generated_candidates`, `qa_checks`, `provider_capabilities`, `cost_counters`
- Files: `library/assets/`, `library/thumbs/`, `library/candidates/`

## Remote (`supabase/migrations/20260722000000_visual_library.sql`)

- Catalog + queue + assignments + RLS
- Storage bucket `visual-library` (private)
- Path pattern: `themes/{slug}/concepts/{concept_id}/assets/{asset_id}/original.webp`
- `owner_id` / `workspace_id` nullable until product auth exists; policies use `auth.uid()` membership, not `user_metadata`

## Sync strategy

1. Desktop works offline on SQLite always  
2. When Supabase env is set (future client), push approved assets + concepts; pull catalog  
3. Dedup by `sha256` / `idempotency_key`  
4. No Base64 in Postgres  

## Rollback

- Local: keep previous DB file; schema only ADDs columns/tables  
- Remote: new migration to drop new objects; never rewrite storage.objects internals  
