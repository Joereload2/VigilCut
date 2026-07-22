# Visual Library & Transcript Enrichment — Design

**Date:** 2026-07-20  
**Status:** Implemented (MVP vertical slice)  
**Branch:** feat/intelligent-clipping  

## Problem

Operators cut silences well, but still need B-roll / illustrative stills for talking-head content. Doing this in a separate NLE is slow and disconnected from the factory pipeline.

## Goal

A **local** path:

```
Video → Transcript (source times) → Semantic events
     → Library match → Suggestions → Human accept/reject
     → VisualPlan (composition ops on output timeline)
     → Live preview + FFmpeg bake (same model)
     → Artifacts + usage history
```

Separate from EDL (what survives of the source). VisualPlan answers **what image appears when on the final timeline**.

### Supervision UI (not a full NLE)

Philosophy: **AI proposes, human only reviews exceptions.**

| Surface | Responsibility |
|---------|----------------|
| **Timeline** (output clock) | When: video/cuts, transcript, B-roll blocks, waveform — shared playhead/zoom |
| **Preview** | Spatial: fullscreen vs overlay, drag, resize, fit, protected zones |
| **Inspector** | Props, warnings, accept / restore AI / remove |

B-roll is **composition** state (`VisualPlacement` + issues + spatial zones), not Segment legacy.

## Out of scope (MVP)

- Full Story Builder UI (contracts only — see `models/story_contracts.rs`)  
- Embeddings / mandatory LLM vision  
- Full NLE multi-track editor  
- Permanent delete of assets from UI  
- Dynamic plugin system  
- Silent paid generation  

## Intelligent library (2026-07-22)

See `docs/visual-library/architecture.md`. Adds: VisualConcept, VisualNeed, generation queue (OmniRoute replaceable), QA, cost policy, optional Supabase migrations. Generation is never required; search-before-generate is mandatory.

## Architecture

| Layer | Responsibility |
|-------|----------------|
| Transcript | Canonical JSON + SRT/TXT projections |
| TimeMap | Source ↔ output mapping from EDL keep ranges |
| Semantic | Deterministic keyword/phrase/concept extraction |
| Library | SQLite metadata + managed copies + thumbnails |
| Match | Explainable ranking + penalties |
| VisualPlan | Accepted placements only |
| Render | FFmpeg overlay on already-cut longform (or keep ranges) |

Canonical chain unchanged: **Media → Features → Events → Policies → EDL → Renders**.  
Visual is a **parallel enrichment** after EDL exists.

## Data model (summary)

- `Transcript`, `TranscriptSegment`  
- `TimeMap` (keep ranges)  
- `SemanticEvent`  
- `MediaAsset` (SQLite)  
- `VisualSuggestion`, `VisualPlacement`, `VisualPlan`  
- `AssetUsage` history  

## Matching (MVP)

Positive: concept/tag/title token overlap.  
Penalties: used recently, same video, blocked, unknown license, orientation.  
Max ~3.5 images/min, min gap 8s, duration 3–6s.

## Security

- Original video never modified  
- Import **copies** into managed storage; originals untouched  
- No silent overwrite on render (temp → validate → rename)  
- License `unknown` warned / not auto-final without flag  

## Acceptance (vertical)

Transcript export + time map tests + import image + one suggestion + accept → plan → overlay render + unit tests.
