# Changelog

## 1.1.0 — 2026-07-20 (hardening)

### Safety
- Export never writes the original: temp file → validate size → atomic rename
- No silent overwrite of destination (unique `stem-N.ext` if exists)
- Reject input path == output path
- Cleanup temp files on failed/cancelled export

### Exception modes (factory batch / watch / CLI)
- **Safe** (default): auto-cuts high confidence; pending exceptions kept in output
- **Supervised**: skip export while exceptions pending
- **Aggressive**: force-accept pending (explicit UI confirm / CLI `--aggressive`)
- Watch inbox and factory default are **Safe** (no longer force-cut)

### Security
- Tauri `fs:scope` and asset protocol no longer use bare `**`
- Scopes limited to APPDATA, TEMP, HOME, Documents, Downloads, Desktop, Video, etc.

### Models
- `setup:models` verifies HTTP, size, SHA-256, atomic install of Silero ONNX

### UX
- Batch panel mode selector + aggressive warning
- Summary states: “Listo” vs “Requiere revisión”
- Export success: “Tu video original no fue modificado”
- Heuristic score language (not “scientific confidence”)

### Tooling
- Version 1.1.0 across package.json / Cargo.toml / tauri.conf
- `npm run lint` → svelte-check (no phantom ESLint)
- GitHub Actions CI skeleton (Windows)
- Docs: `docs/HARDENING_1_1.md`

## Unreleased — feat/intelligent-clipping (merged into 1.1 workstream)

### Intelligent clipping
- Domain: `ClipCandidate`, scores, framing 9:16, duration/selection profiles
- Pipeline: SRT/VTT / sidecar / optional Whisper / speech fallback → candidates → score → dedupe → preselect
- UI: Clips workspace, classify/review, live 9:16 player, RightDock layout
- Export: individual + batch vertical MP4 + metadata/report
- CLI: `vigilcut-cli clips <video> [outdir]`
- Tests: unit + smoke (SRT/sidecar) + e2e vertical export
- Docs: `docs/CLIPPING.md`, `docs/BACKLOG_NEXT.md`

## 1.0.0 — 2026-07-18

### Factory engine
- Events → Policy → EDL → multi-artifact export
- Silero VAD (ONNX) with FFmpeg fallback
- Auto-approve high-confidence silences; exception queue for the rest
- Structure detectors: chapters, short candidates, breath/micro-pauses
- Optional Whisper CLI → SRT + filler (muletilla) cuts
- Feature cache (16 kHz WAV by media hash)

### Batch & automation
- Async batch worker (files or inbox folder)
- Inbox watch (poll) + process-now
- CLI: `analyze` / `export` / `batch` with `--policy`
- Policy packs: factory, youtube, podcast, gentle, shorts-first

### Artifacts per export
- `*-editado.mp4`, manifest JSON, events, EDL
- chapters JSON + YouTube TXT
- shorts JSON + real `*-shorts/short-NN.mp4` clips

### Desktop
- Supervisor UI (exceptions, not full-segment review by default)
- Cut preview, export success panel
- Factory batch panel with policy selector

## 0.1.x

- Initial Tauri + Svelte MVP (silence timeline)
