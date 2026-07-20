# Visual Library — Implementation Report

**Date:** 2026-07-20  
**Branch:** `feat/intelligent-clipping`  
**Scope:** Vertical MVP — Transcript → Semantics → Library → Suggestions → Plan → Render  

---

## 1. Executive summary

Implemented a **working vertical slice** of visual enrichment without cloud, without NLE, and without modifying originals:

1. Canonical **Transcript** (JSON) + projections **SRT/TXT**  
2. **TimeMap** source↔output from EDL keep ranges (unit-tested)  
3. **Semantic events** (deterministic keywords/phrases/concepts)  
4. **SQLite local library** with managed copies, thumbs, SHA-256 dedupe  
5. **Explainable matching** + human accept/reject  
6. **VisualPlan** separate from EDL  
7. **FFmpeg overlay render** of accepted placements (atomic finalize)  
8. UI mode **Visual** + panel  
9. Tauri commands registered  

---

## 2. Initial state

- Hardening 1.1 already on branch (`f2cbbe5`).  
- Whisper only produced optional SRT paths for clipping.  
- No transcript domain, no image library, no visual plan.  
- Working tree clean before this epic.

---

## 3. Architecture

```
AnalysisRun.edl (keep ranges)
        │
        ├─ TimeMap (source ↔ output)
        │
Transcript (source times) ──► SemanticEvent[]
        │                            │
        │                            ▼
   Library (SQLite) ──────► rank_suggestions → VisualSuggestion[]
                                    │ human
                                    ▼
                              VisualPlan (placements)
                                    │
                                    ▼
                         FFmpeg overlay on cut MP4
```

EDL = what survives of the source. VisualPlan = stills on the **output** timeline.

---

## 4. Decisions

| Decision | Rationale |
|----------|-----------|
| SQLite + managed files under APPDATA/library | Local, queryable, no cloud |
| Deterministic NLP only | Explainable, no LLM dependency |
| Session in-memory + transcript files in cache | Simple; full DB for assets only |
| Overlay on **already-cut** longform | Audio stays in sync with output times |
| No auto-download of stock images | License/privacy mandate |

---

## 5–7. Done / partial

### Done
- Transcript model + SRT/TXT/JSON export  
- TimeMap + exhaustive unit tests  
- Semantic extract + concept dictionary  
- Library import, list, update meta, usage after render  
- Suggestions rank + accept/reject → plan  
- EDL fingerprint invalidation hook  
- Render overlays + atomic output  
- UI mode Visual + VisualPanel  
- Design + this report  

### Partial
- Live preview of overlays inside player (render-only)  
- Full CLI `visual` subcommand  
- Supervised re-map of placements when EDL changes (clear + warn)  
- Word-level timestamps from Whisper (segment-level only)  
- Folder bulk import UI (API can loop)  

---

## 8. Main files

- `docs/VISUAL_LIBRARY_DESIGN.md`  
- `src-tauri/src/models/transcript.rs`, `visual.rs`  
- `src-tauri/src/pipeline/time_map.rs`, `transcript_engine.rs`, `semantic.rs`  
- `src-tauri/src/pipeline/visual/*`  
- `src-tauri/src/commands/visual.rs`  
- `src/lib/components/VisualPanel.svelte`  
- Dependencies: `rusqlite`, `sha2`, `hex`, `image`  

---

## 9–13. Tests

`cargo test --lib` → **48 passed**  
`npm run build` → OK  
`npm run check` → 0 errors  

New tests: time_map (6), semantic, transcript, match_rank, transcript_engine SRT fixture.

Not run this session: full Tauri E2E, FFmpeg visual render smoke (needs real encode + assets).

---

## 14–19. Review loops

**QA:** Fixed private module exports, borrow checker on accept, LicenseStatus misuse.  
**CTO:** Kept EDL separate; no cloud; atomic render. Residual: dual session vs disk for plans.  
**UX/PM:** Mode “Visual” with clear 5-step copy; no technical EDL jargon on main path.

---

## 20–22. Risks / next

1. Overlay filter graph may need tuning on odd resolutions.  
2. Without SRT/concepts, suggestions stay empty (by design).  
3. Next: in-player preview; bulk folder import; persist VisualPlan JSON next to export.  

---

## 23–25. Git / how to test (Windows)

```powershell
# After analyze (Silencios) on a video with companion .srt
# Mode Visual → + Imagen (tags: inflacion,alimentos) → Generar sugerencias
# Aceptar → Exportar video cortado → Render plan (elige el *-editado.mp4)
```

Commits: see local history after this session. **No push.**
