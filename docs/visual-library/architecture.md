# Intelligent Visual Library — Architecture (adapted to VigilCut)

**Date:** 2026-07-22  
**Branch:** `feat/intelligent-clipping`  
**Status:** Implementation (extends existing MVP)

## Phase 0 findings (what already exists)

| Area | Status |
|------|--------|
| `MediaAsset` + SQLite `library.db` | Done — managed copies, thumbs, SHA-256 dedupe, usage |
| `VisualPlan` / `VisualPlacement` | Done — composition supervision, FFmpeg bake |
| Matching | Done — explainable keyword/concept rank (`match_rank`) |
| Transcript + SemanticEvent | Done — deterministic NLP, no LLM required |
| TimeMap source↔output | Done |
| OmniRoute / Supabase / generation queue | **Missing** |
| VisualConcept / VisualNeed / QA pipeline | **Missing** (concepts only as free strings on assets) |
| Story Builder | Not built — contracts only |

**Reuse:** do not fork a second app. Extend `pipeline/visual/*`, `models/visual*`, Tauri commands, Visual mode UI.

## Loop 1 decisions (CTO / Architect / PM / Supabase / AI)

1. **Single app, domain boundaries** — library core, needs, matching, generation, QA, optional sync live under `pipeline/visual/` with clear modules; Story Builder only as `models/story_contracts.rs`.
2. **Asset ≠ Placement** — keep `MediaAsset` library-global; placements stay on `VisualPlan`.
3. **Local-first** — SQLite remains operational SoT for desktop offline; Supabase = optional central catalog + Storage (migrations + RLS ready, no hard runtime dependency).
4. **OmniRoute is replaceable** — `ImageGenerationProvider` trait; default `MockImageProvider` in tests; real HTTP only when env configured; **paid disabled by default**.
5. **Search before generate** — always; generation is never required to finish a video.
6. **Cost policy hard gates** — `paid_providers_enabled=false`, daily paid budget 0, caps per day/project/need.
7. **No Base64 in Postgres** — Storage paths only; local managed files under `library/`.
8. **Auth readiness** — Supabase tables include `owner_id`/`workspace_id` nullable; RLS uses `auth.uid()` when present; desktop single-user without fake auth.

## Loop 2 simplifications (UX / QA / ops)

| Avoided | Instead |
|---------|---------|
| 12 overlapping tables for MVP | Themes + concepts + assets + needs + jobs + candidates + qa_checks + usage (existing) + provider_capabilities |
| Embeddings mandatory | Text match + exclusions now; trait hook later |
| Semantic vision QA mandatory | Technical QA always; semantic QA as pluggable scorer (rule/heuristic stub) |
| Always-on opportunistic generation | Only priority uncovered concepts when free quota observed **and** policy allows |
| Separate desktop app | Sub-panel Biblioteca + coverage summary in Visual mode |
| Cartesian concept explosion | Seed curated theme packs; dedupe by `canonical_key` |

## Common representation (fundamental rule)

```
Edit existing video | Future text→video
        ↓
   VisualNeed[]  →  match/reuse or generate+QA
        ↓
   VisualPlan (placements → media_asset_id)
        ↓
   FFmpeg render (unchanged path)
```

## Cost priority order

1. Reuse existing asset  
2. Crop/variant of existing (future; layout already supports fit)  
3. Free OmniRoute provider with quota  
4. Local generator if configured  
5. Other free  
6. Paid **only** if explicitly enabled + budget  
7. Leave need uncovered  

## Secrets

Never hardcode. See `.env.example` and app env:

- `OMNIROUTE_BASE_URL`, `OMNIROUTE_API_KEY`, `OMNIROUTE_IMAGE_MODEL`
- `SUPABASE_URL`, `SUPABASE_ANON_KEY` (never service_role in client)
- `VIGILCUT_LIBRARY_ROOT` (tests / override)
- `VIGILCUT_PAID_PROVIDERS=0`, `VIGILCUT_MAX_DAILY_GENERATIONS`, …

## Module map (repo conventions)

```
src-tauri/src/
  models/visual.rs          # placements, plan, MediaAsset (extended)
  models/visual_intel.rs    # concept, need, job, candidate, QA, cost
  models/story_contracts.rs # future Story Builder DTOs
  pipeline/visual/
    library.rs              # SQLite CRUD + import
    schema.rs               # versioned local migrations
    concepts.rs
    needs.rs
    intelligent_match.rs
    qa.rs
    generation/{mod,provider,mock,omniroute,cost,worker}.rs
    match_rank.rs           # legacy enrich path (kept)
    render.rs / compose.rs / layout.rs
supabase/migrations/        # remote catalog + RLS + storage policies
docs/visual-library/
```

## Non-goals (this wave)

- Full Story Builder UI  
- Real Supabase cloud deploy from CI without secrets  
- Paid provider spend  
- Embedding index  
- Automatic bulk generation of entire theme packs  
