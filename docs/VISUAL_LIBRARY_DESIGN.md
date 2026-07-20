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
     → VisualPlan (output times) → Preview/render overlays
     → Artifacts + usage history
```

Separate from EDL (what survives of the source). VisualPlan answers **what image appears when on the final timeline**.

## Out of scope (MVP)

- Cloud, Supabase, remote image download  
- Embeddings / mandatory LLM  
- Full NLE multi-track editor  
- Permanent delete of assets from UI  
- Dynamic plugin system  

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
