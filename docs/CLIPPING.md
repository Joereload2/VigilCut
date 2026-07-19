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
- Preview de zona segura + centro en el panel (editor E).
- Ajuste manual del centro (←→↑↓) y modos de export.
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

- Unit: score, dedupe, framing, transcript parse
- E2E: `cargo test --test e2e_clipping`
