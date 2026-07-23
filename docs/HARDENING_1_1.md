# VigilCut 1.1 — Hardening report

**Date:** 2026-07-20  
**Branch:** `feat/intelligent-clipping`  
**Version decision:** **1.1.0** (behavioral defaults + safety; not a full rewrite)

---

## 1. Executive summary

VigilCut v1 was a working factory MVP with a dangerous default: **batch/inbox auto-accepted all pending exceptions** (force-cut). Export could write directly to the destination without strong collision/input-output guards. Tauri FS/asset scopes used unrestricted `**`. Silero download tracked a floating URL without checksum.

This hardening pass implements **safe-by-default exception modes**, **atomic export**, **path safety**, **tighter capabilities**, **verified Silero install**, clearer **heuristic scores**, CI skeleton, and documentation alignment — without changing the canonical architecture Media → Events → Policy → EDL → Artifacts.

---

## 2. Loop 1 — Diagnosis (condensed)

| # | Severity | Problem | Evidence |
|---|----------|---------|----------|
| 1 | **Critical** | Batch default `auto_accept_exceptions: true` force-cuts doubts | `batch.rs` unwrap_or(true); `batch_worker` accept_all |
| 2 | **Critical** | Export can overwrite destination; no atomic finalize | `export_keep_ranges_with_audio` wrote final path directly |
| 3 | **High** | Same path input/output possible in principle | No validate before encode |
| 4 | **High** | Tauri `fs:scope` + assetProtocol `**` | `capabilities/default.json`, `tauri.conf.json` |
| 5 | **High** | Silero from `master` without SHA-256 | `scripts/setup-models.mjs` |
| 6 | **Medium** | Scores presented as “conf” / probability | policy strings, Event.score docs |
| 7 | **Medium** | `lint` script calls missing ESLint | package.json |
| 8 | **Medium** | Docs/repo URL/version drift | Cargo.toml github/vigilcut; Silero done vs pending in ARCHITECTURE |
| 9 | **Medium** | Watch/inbox used aggressive path | watch process_one_file(true) |
| 10 | **Low** | No formal metrics schema for human_seconds_per_media_minute | n/a |

---

## 3. Loop 2 — Simplification

**Kept:** Exception mode enum (Safe/Supervised/Aggressive), atomic export helpers, unique path, safe paths tests, checksum download, scope to well-known dirs (not bare `**`), manifest fields, batch UI selector with aggressive confirm, CI basic, metrics struct stub.

**Dropped from this pass (over-scope):** Full feature broker plugin system, FFmpeg time= progress parsing, streaming Silero, face tracking, ESLint full stack, signed installer, full human-wait timing instrumentation.

**Assumption:** `$HOME/**` + Documents/Downloads/Desktop/Video is enough for real media paths without global `**`. Network drives outside home may need future dynamic scope.

---

## 4. Plan implemented

1. Atomic export + original protection  
2. Exception modes + safe default batch/watch  
3. Manifest fields for mode / conservative export  
4. Whisper remains opt-in (`prefer_whisper` default false)  
5. Silero download checksum  
6. Tauri capability tightening  
7. Heuristic score language  
8. EDL edge normalization + tests  
9. UX: summary status, batch modes, export “original intact”  
10. CI + lint alias  
11. Docs + version 1.1.0  

---

## 5. Behavioral changes (user-visible)

| Before | After |
|--------|--------|
| Lote cortaba dudas por defecto | Lote **Seguro** por defecto |
| Watch inbox agresivo | Watch **Safe** |
| Export escribía destino final directo | Temp → validar → rename; no overwrite silencioso |
| Sin selector de modo en lote | Seguro / Supervisado / Agresivo (+ confirm agresivo) |
| “conf %” en textos | “score heurístico / estimación operativa” |
| Export success genérico | **“Tu video original no fue modificado”** |

---

## 6. Architecture notes

- No change to Events → Policy → EDL pipeline.  
- `ExceptionHandlingMode` is factory/job layer, not a new detector.  
- EDL still ignores pending exceptions as removes (`effective_removes`) — Safe mode relies on that.  
- Supervised mode skips export when pending &gt; 0.  

---

## 7. Security changes

- Removed bare `**` from fs:scope and assetProtocol.  
- Retained `$HOME/**` and standard user media locations so open/preview still work.  
- Backend validates export paths (exists, readable, ≠ input, unique dest).  

---

## 8. Tests

- `pipeline::safe_paths::*` — same file, collision, tiny output, missing input  
- `models::exception_mode::*`  
- `models::edl::edl_edge_tests::*`  
- Existing unit suite: **38 passed** (lib)

---

## 9. Residual risks

- Media on paths outside `$HOME` / special roots may need expanded scope.  
- Silero URL still points at `master` raw; hash pins content but upstream move requires hash update.  
- Supervised multi-file batch marks needs_review but no full UI queue of “resume after review” yet.  
- Segment manual K/X vs EDL dual truth not fully unified in this pass.  
- CI smoke continue-on-error if FFmpeg missing on runner.  

---

## 10. Next three steps

1. Resume-export UI for Supervised batch files.  
2. Pin Silero to a release tag URL (not master).  
3. Segment edits recompile EDL (single source of truth).  

---

## 11. Git

Commits to create locally (no push without authorization).  
Version packages: **1.1.0**.
