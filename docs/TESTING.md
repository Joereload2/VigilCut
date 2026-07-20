# VigilCut — pirámide de tests

## Capas

| Capa | Comando | Qué valida |
|------|---------|------------|
| **Unit** | `npm run test:unit` | Policy, EDL, export filter, chapters map, fillers, parse (sin media) |
| **Smoke** | `npm run test:smoke` | Silencios + clipping + overlay visual FFmpeg sobre vídeo sintético |
| **E2E fábrica** | `npm run test:e2e` | Export EDL longform + export clips 9:16 + batch |
| **Todo** | `npm test` | Unit + smoke (pipeline+clipping) + e2e (factory+clipping) |
| **CI** | GitHub Actions | `.github/workflows/ci.yml` en push/PR |

## Requisitos

- Rust / cargo
- Sidecar FFmpeg: `src-tauri/binaries/ffmpeg.exe` (`npm run setup:ffmpeg` si falta)
- No hace falta GUI ni WebView2 para unit/smoke/e2e de motor

## Visual library (unit)

Cubierto en `cargo test --lib`:

- `pipeline::time_map` — source↔output mapping  
- `pipeline::transcript_engine` / `models::transcript` — SRT/TXT projections  
- `pipeline::semantic` — deterministic concepts  
- `pipeline::visual::match_rank` — ranking + penalties  
- `pipeline::visual::library` — import, SHA dedupe, folder, usage, missing scan  

Override de tests: `set_library_root_override` / `VIGILCUT_LIBRARY_ROOT`.

## Smoke visual (FFmpeg)

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --test smoke_visual -- --nocapture
```

Valida: import PNG → VisualPlan placement → overlay encode → manifiesto → usage post-éxito → original intacto; y rechazo de plan vacío.

## Qué no cubre (aún)

- E2E de la app Tauri/WebView (clics UI) — el producto de 5 años es el **motor**; la UI es cliente delgado
- Modelos Silero/Whisper en CI (smoke fuerza `prefer_silero: false` para determinismo)

## Fixtures

Los tests generan media en `src-tauri/target/test-workspace/*` con lavfi (color + sine + mute mid).
