# Clipping inteligente — arquitectura y flujo

## Flujo

```text
Video → (SRT/VTT opcional | speech fallback) → unidades semánticas
     → candidatos → score explicable → dedupe → preselección
     → revisión humana (A/R) → export 9:16 (FFmpeg crop/pad/blur)
```

## Capas

| Capa | Ubicación |
|------|-----------|
| Dominio | `models/clipping.rs` |
| Pipeline | `pipeline/clipping/*` |
| Commands | `commands/clipping.rs` |
| UI | `ClippingPanel.svelte` (pestaña Clips) |

## Transcripción

1. Importar SRT/VTT (recomendado).
2. Si no hay: bloques de habla del análisis de silencios (heurístico).
3. Whisper CLI se puede enganchar vía `transcript_path` futuro / import.

## Encuadre 9:16

- `auto_center`: crop estático centrado (talking-head Y=0.42).
- `blurred_background` / `fit_with_bars` preparados.
- `tracking_ready: false` — contrato listo para face-track futuro.

## Export

```text
{video}-clips/
  clips/001_titulo.mp4
  metadata.json
  clipping-report.json
```

## Tests

- Unit: score, dedupe, framing, transcript parse
- E2E: `cargo test --test e2e_clipping`
