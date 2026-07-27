# VigilCut Architecture / Arquitectura

## Philosophy — AI works, human supervises exceptions

See also [ARCHITECTURAL_REVIEW.md](./ARCHITECTURAL_REVIEW.md) (CTO mandate).

Pipeline (engine, 2026-07):

1. **Detect** — emit `Event[]` (e.g. `audio.silence`)  
2. **Policy** — high-confidence auto `EditOp` (remove); low-confidence → `ExceptionItem`  
3. **EDL** — compile keep ranges from ops + resolved exceptions  
4. **Supervise** — human only on exception queue (not every segment)  
5. **Preview / Export** — same keep ranges via skip-cuts / FFmpeg  

Legacy `Segment[]` is a **UI projection** of events + policy, not the source of truth.

### Factory batch (2026-07)

```
queue_batch_job / queue_inbox_batch
  → async worker per file: run_silence_analysis → accept exceptions → export
  → outbox/*.mp4 + *.json manifest
  → events: batch://progress, batch://done
```

CLI (no UI):

```
cargo run --bin vigilcut-cli -- analyze video.mp4
cargo run --bin vigilcut-cli -- batch ./inbox ./outbox
cargo run --bin vigilcut-cli -- export video.mp4
cargo run --bin vigilcut-cli -- visual import ./images --concepts topic
cargo run --bin vigilcut-cli -- visual transcript video.mp4 ./out
```

App data: `%APPDATA%/VigilCut/inbox`, `outbox`, and `library` (visual assets).

### Visual enrichment (local)

```
Transcript (canonical) → SemanticEvent[] → match library → VisualSuggestion[]
  → human accept/reject → VisualPlan (placements on OUTPUT timeline)
  → FFmpeg overlay on already-cut longform (EDL unchanged)
```

- **EDL** = what survives of the source  
- **VisualPlan** = stills overlaid on the cut output  
- Original media is never modified; library stores managed copies only  
- Design: [VISUAL_LIBRARY_DESIGN.md](./VISUAL_LIBRARY_DESIGN.md)

---

## Stack

| Layer | Tech |
|--------|------|
| Shell | Tauri 2 |
| Backend | Rust (`src-tauri/`) |
| Frontend | Svelte 5 + TypeScript + Tailwind |
| Media | FFmpeg / FFprobe as `externalBin` sidecars |
| VAD | Silero (ONNX) when model present; FFmpeg `silencedetect` fallback |
| ASR (planned) | Local Whisper models under app `models/` |

---

## Module map

```
src-tauri/src/
  commands/     # Tauri IPC surface (incl. visual_*)
  ffmpeg/       # Sidecar resolve + probe + filters
  pipeline/     # silence, export, time_map, transcript, semantic, visual/*
  models/       # Domain types (transcript, visual, edl, …)
  state.rs      # App data dirs, in-memory project/batch maps

src/lib/
  components/   # Timeline, Preview, Inspector, …
  stores/       # project.svelte.ts (runes)
  types/        # TS mirrors of Rust JSON
  utils/tauri.ts
```

---

## MVP scope (v0.1)

- [x] Open media, probe with ffprobe  
- [x] Silence detection → speech/silence segments  
- [x] Timeline with toggle Keep/Cut, split, zoom  
- [x] Preview skip-cuts plan  
- [x] Export Keep ranges via `filter_complex` concat  
- [x] Built-in presets + user preset save  
- [x] Batch job queue (status plumbing; worker next)  
- [x] Subtitle SRT/VTT import  
- [ ] Silero ONNX inference (hook + model path ready)  
- [ ] Whisper local transcription  
- [ ] Full audio enhance at export  
- [ ] Batch worker pool + progress events  

---

## Feature loops (designed)

### Loop A — Silence cut
`detect_silences` → segments → user review → `export_video`

### Loop B — Audio enhance
`enhance_audio_preview` builds filter graph (`afftdn`, `loudnorm`, highpass) → applied on export

### Loop C — Color / lighting
`analyze_color_stats` / `eq` filter → optional auto-levels later

### Loop D — Subtitles
`import_subtitles` (SRT/VTT) + `generate_subtitles_whisper` (stub)

### Loop E — Clip preselection
`analyze_speech_segments` with `auto_cut_silence: false` + clip_candidate kind (extensible)

---

## Data: Segment

```json
{
  "id": "uuid",
  "start": 1.2,
  "end": 3.4,
  "kind": "speech|silence|manual|…",
  "decision": "keep|cut|pending",
  "confidence": 0.92
}
```

Projects are stored as JSON under `%APPDATA%/VigilCut/projects/<id>/project.json` (Windows) or platform equivalent.

---

## FFmpeg sidecars

Tauri `bundle.externalBin`: `binaries/ffmpeg`, `binaries/ffprobe`.

Setup: `npm run setup:ffmpeg` copies system binaries into `src-tauri/binaries/` with target-triple names.

---

## Silero VAD integration plan

1. Ship or download `silero_vad.onnx` into `AppState::models_dir()`  
2. Load via `ort` / `tract` crate  
3. Feed 16 kHz mono PCM from `Ffmpeg::extract_audio_wav`  
4. Post-process speech probs → same `silences_to_segments` helper  

Until the model file exists, the pipeline uses FFmpeg `silencedetect` and sets `method` accordingly.

---

## Security

- Fully offline; no telemetry  
- Filesystem access scoped via Tauri capabilities (dev is permissive for UX; tighten for release)  
- User must confirm export path  

---

## License

MIT — see `/LICENSE`
