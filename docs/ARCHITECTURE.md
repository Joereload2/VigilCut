# VigilCut Architecture / Arquitectura

## Philosophy — Human in the loop

VigilCut never exports irreversible decisions without review. The pipeline:

1. **Analyze** (automated) — VAD, silence, optional audio/color stats  
2. **Propose** (segments with Keep/Cut/Pending)  
3. **Supervise** (user toggles, splits, padding, presets)  
4. **Preview** (playback that jumps cut regions)  
5. **Approve & export** (FFmpeg concat of Keep ranges)

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
  commands/     # Tauri IPC surface
  ffmpeg/       # Sidecar resolve + probe + filters
  pipeline/     # silence → segments, export filter graphs
  models/       # Serializable domain types
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
