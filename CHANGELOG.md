# Changelog

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
