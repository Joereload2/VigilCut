# VigilCut — pirámide de tests

## Capas

| Capa | Comando | Qué valida |
|------|---------|------------|
| **Unit** | `npm run test:unit` | Policy, EDL, export filter, chapters map, fillers, parse (sin media) |
| **Smoke** | `npm run test:smoke` | FFmpeg real + `run_silence_analysis` sobre vídeo sintético (3s tono+silencio) |
| **E2E fábrica** | `npm run test:e2e` | Analyze → export EDL → MP4 + `chapters.txt` + `*-meta/` + `process_one_file` |
| **Todo** | `npm test` | Las tres capas en orden |

## Requisitos

- Rust / cargo
- Sidecar FFmpeg: `src-tauri/binaries/ffmpeg.exe` (`npm run setup:ffmpeg` si falta)
- No hace falta GUI ni WebView2 para unit/smoke/e2e de motor

## Qué no cubre (aún)

- E2E de la app Tauri/WebView (clics UI) — el producto de 5 años es el **motor**; la UI es cliente delgado
- Modelos Silero/Whisper en CI (smoke fuerza `prefer_silero: false` para determinismo)

## Fixtures

Los tests generan media en `src-tauri/target/test-workspace/*` con lavfi (color + sine + mute mid).
