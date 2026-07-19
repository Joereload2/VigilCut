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

## Transcripción (orden de resolución)

1. Ruta explícita (`transcriptPath` / botón + SRT).
2. Sidecar `{video}.srt` o `.vtt` junto al archivo.
3. Si `preferWhisper`: Whisper CLI en PATH (opcional).
4. Fallback: bloques de habla del análisis de silencios (heurístico).

## Encuadre 9:16

- `auto_center`: crop estático centrado (talking-head Y=0.42).
- `blurred_background` / `fit_with_bars` preparados.
- Preview **real** 9:16 (canvas muestreando el frame del video + zona segura).
- Ajuste manual del centro (←→↑↓) y modos de export.
- Reproducción del clip con auto-pausa al final.
- Export individual o por lote.
- `tracking_ready: false` — contrato listo para face-track futuro.

## Revisión (atajos)

| Tecla | Acción |
|-------|--------|
| A | Aprobar |
| R | Rechazar |
| I | Inicio = playhead |
| O | Final = playhead |
| E / Enter | Abrir editor de límites / encuadre |
| ↑↓ | Navegar candidatos |

## Export

```text
{video}-clips/
  clips/001_titulo.mp4
  metadata.json
  clipping-report.json
```

## Tests

- Unit: score, dedupe, framing modes, transcript parse
- Smoke: `cargo test --test smoke_clipping` (SRT + sidecar)
- E2E: `cargo test --test e2e_clipping`
- CI: `.github/workflows/ci.yml`

## CLI

```text
vigilcut-cli clips video.mp4 [outdir]
```

Aprueba preseleccionados (score≥55) y exporta 9:16 usando dimensiones reales del probe.

## Fuera de alcance MVP (deliberado)

- Face tracking en tiempo real (campo `trackingReady` reservado)
- Scoring con LLM
- E2E de clics en WebView/Tauri UI
- Subtítulos quemados en el vertical
