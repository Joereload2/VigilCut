# Visual Library — Implementation Report

**Date:** 2026-07-20 (updated second wave)  
**Branch:** `feat/intelligent-clipping`  
**Scope:** Vertical path — Transcript → Semantics → Library → Suggestions → Plan → Render  

---

## 1. Executive summary

Implemented a **working vertical slice** of visual enrichment without cloud, without NLE, and without modifying originals:

1. Canonical **Transcript** (JSON) + projections **SRT/TXT**  
2. **TimeMap** source↔output from EDL keep ranges (unit-tested)  
3. **Semantic events** (deterministic keywords/phrases/concepts)  
4. **SQLite local library** with managed copies, thumbs, SHA-256 dedupe  
5. **Folder import**, usage history, missing-file scan  
6. **Explainable matching** + human accept/reject  
7. **VisualPlan** separate from EDL, persisted as JSON  
8. **FFmpeg overlay render** of accepted placements (atomic) + manifest  
9. UI mode **Visual** + panel (import folder, export transcript, review)  
10. CLI `visual import|list|transcript|scan-missing`  
11. Tauri commands registered  

---

## 2. Initial state

- Hardening 1.1 already on branch (`f2cbbe5`).  
- Whisper only produced optional SRT paths for clipping.  
- No transcript domain, no image library, no visual plan.  
- Working tree clean before this epic.

---

## 3. Architecture implemented

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
                              VisualPlan (placements) ── JSON cache
                                    │
                                    ▼
                         FFmpeg overlay on cut MP4
                         + *.visual-manifest.json
                         + *.visual-plan.json
```

EDL = what survives of the source. VisualPlan = stills on the **output** timeline.

---

## 4. Decisions

| Decision | Rationale |
|----------|-----------|
| SQLite + managed files under APPDATA/library | Local, queryable, no cloud |
| Deterministic NLP only | Explainable, no LLM dependency |
| Session in-memory + plan/transcript JSON under cache | Simple; full DB for assets only |
| Overlay on **already-cut** longform | Audio stays in sync with output times |
| No auto-download of stock images | License/privacy mandate |
| `VIGILCUT_LIBRARY_ROOT` + override mutex for tests | Isolated library unit tests |
| EDL change clears placements + warning | Never keep silent stale timestamps |

---

## 5–7. Done / partial

### Done
- Transcript model + SRT/TXT/JSON export  
- TimeMap + expanded unit tests (boundaries, roundtrip, invalid spans)  
- Semantic extract + concept dictionary  
- Library import, folder import, list, update meta, usage after render  
- SHA-256 dedupe; original never deleted/modified  
- Missing managed-file scan  
- Suggestions rank + accept/reject/undo → plan  
- Plan JSON persist on generate and on human decision  
- EDL fingerprint invalidation  
- Render overlays + atomic output + manifest beside file  
- UI mode Visual: folder, export transcript, source/output labels  
- CLI visual subcommands  
- Design + this report  

### Partial
- Live preview of overlays inside the video player (render-only path works)  
- Headless enrich writes suggestions for human review; render is separate CLI step  
- Supervised re-map of placements when EDL changes (clear + warn; no auto-remap)  
- Word-level timestamps from Whisper (segment-level only)  
- Embeddings / local LLM matching (interface ready; not required)  

---

## 8. Main files

| Area | Paths |
|------|--------|
| Design | `docs/VISUAL_LIBRARY_DESIGN.md` |
| Models | `src-tauri/src/models/transcript.rs`, `visual.rs` |
| Engine | `pipeline/time_map.rs`, `transcript_engine.rs`, `semantic.rs` |
| Visual | `pipeline/visual/{library,match_rank,render,mod}.rs` |
| IPC | `commands/visual.rs` |
| UI | `src/lib/components/VisualPanel.svelte`, `ModeNav` |
| CLI | `src-tauri/src/bin/vigilcut_cli.rs` |

### Dependencies added
- `rusqlite` (bundled) — local library metadata  
- `sha2` / `hex` — content hash / fingerprints  
- `image` — decode + thumbnails  

---

## 9. Migrations

SQLite schema created on first open (`CREATE TABLE IF NOT EXISTS media_assets`, `asset_usage`). No remote migration service.

---

## 10. UX

Flow in mode **Visual**:

1. Analyze silences first (EDL).  
2. Import image(s) or folder with concepts.  
3. Generate suggestions.  
4. Accept / reject (human).  
5. Export cut longform (Silencios).  
6. Render plan → overlay on cut file.  

Transcript search, export TXT/SRT/JSON, and explicit source vs output timing labels are available.

---

## 11–13. Tests and exact results

| Suite | Result |
|-------|--------|
| `cargo test --lib` | **59 passed**, 0 failed |
| `cargo test --test smoke_visual` | **2 passed** (overlay + empty-plan refusal) |
| `npm run check` | **0 errors**, 0 warnings |
| `npm run build` | **OK** (vite) |
| CLI smoke `visual import` + `list` | **OK** (temp PNG + `VIGILCUT_LIBRARY_ROOT`) |
| `cargo build --bin vigilcut-cli` | **OK** |

New/expanded tests: time_map (11), match_rank (4), library (3), smoke_visual (2), transcript, semantic.

Third wave: unknown license excluded from auto-rank; CLI `visual enrich` / `visual render`; FFmpeg overlay smoke.

---

## 14–15. Loop QA

### Findings
- Private module re-exports (earlier wave)  
- Borrow checker on accept (earlier wave)  
- LicenseStatus misuse for asset status  
- Library tests raced on env override → fixed with mutex override  
- Old CLI binary still “v1.0” until rebuild  

### Corrections
- Serial library test lock + `set_library_root_override`  
- Folder import uses `ImportOutcome` for accurate duplicate counts  
- Manifest written only after successful finalize  

---

## 16–17. Loop CTO

### Findings
- Keep EDL and VisualPlan separate — OK  
- Session-only plan risk of loss on crash — mitigated with JSON under cache  
- Dual path complexity if auto-remap half-implemented  

### Simplifications
- No embeddings / no plugin system  
- Clear+warn on EDL change instead of partial remap  
- Overlay always on cut timeline (no dual-timeline FFmpeg graph)  

---

## 18–19. Loop UX/PM

### Findings
- Need folder bulk import for real libraries  
- Need export of transcript without hunting cache  
- Score must not look “scientific”  

### Corrections
- `+ Carpeta`, export TXT/SRT/JSON, match reasons shown as heuristic labels  
- Copy: “VisualPlan (separado del EDL)” and 6-step empty state  

---

## 20–21. Residual risks / limitations

1. Overlay filter graph may need tuning on odd resolutions / multi-image density.  
2. Without SRT/concepts, suggestions stay empty (by design).  
3. In-player live overlay preview not implemented.  
4. Paths outside Tauri `$HOME`/scoped dirs may fail open-dialog access depending on OS scope.  
5. Whisper word-level timestamps not parsed.  

---

## 22. Next steps (recommended)

1. FFmpeg smoke test: synthetic cut video + one placement → validate A/V sync.  
2. Optional in-player placement markers on the cut preview timeline.  
3. Single CLI: `visual enrich <video> --srt … --assets … --out …` for headless factory.  

---

## 23. Local commits

See `git log` on `feat/intelligent-clipping`. Typical:

- `f2cbbe5` — hardening 1.1  
- `441f21c` — visual MVP  
- *(plus this session’s commit for library/CLI/persist wave)*  

**No push** (mandate).

---

## 24. Final git state

Working tree expected clean after commit; branch ahead of `origin/feat/intelligent-clipping`.

---

## 25. How to test on Windows

```powershell
cd VigilCut
npm install
npm run setup:ffmpeg
npm run dev:win

# After analyze (Silencios) on a video with companion .srt
# Mode Visual → + Imagen or + Carpeta (tags: inflacion,alimentos)
# Generar sugerencias → Aceptar → Exportar video cortado → Render plan

# CLI
npm run cli -- visual import .\mis-imagenes --concepts inflacion,economia --recursive
npm run cli -- visual list inflacion
npm run cli -- visual transcript .\clip.mp4 .\out --srt .\clip.srt

# Unit
cd src-tauri
cargo test --lib
cd ..
npm run check
npm run build
```

Original media and user source images are never modified; library copies live under `%APPDATA%\VigilCut\library`.
