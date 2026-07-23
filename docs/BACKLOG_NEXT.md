# Backlog siguiente — feat/intelligent-clipping

Checklist operativo post-MVP. Orden = ROI (calidad + velocidad de fábrica).  
No merge a `main` sin autorización.

---

## Hecho en esta rama (referencia)

- [x] Silencios: Events → Policy → EDL → export + excepciones  
- [x] Silero VAD + batch/inbox/CLI + multi-artefacto  
- [x] Shorts: sacar, score ≥ 50, clasificar, 9:16 live, export  
- [x] Marco 9:16 arrastrable + títulos Clip NN / frase  
- [x] Layout 3 zonas + RightDock (Resumen / Supervisar / Timeline / Lote / Ajustes)  
- [x] Progreso % + cancel básico + Whisper off al abrir  
- [x] Audio enhance real en export + UI visible  
- [x] Reuso parcial de análisis silence → clips  

---

## Sprint A — Velocidad y no mentir (P0)

| ID | Tarea | Criterio de hecho | PR sugerido |
|----|--------|-------------------|-------------|
| A1 | Reuso **garantizado** silence → clipping (run id + cache path) | “Sacar clips” no re-corre VAD si hay run del mismo media | `perf/reuse-analysis-clips` |
| A2 | Sesión ONNX Silero **reutilizada** (batch + multi-open) | 2º archivo del lote no paga init completo de sesión | `perf/silero-session-pool` |
| A3 | Cancel **durante** etapas largas de VAD (check periódico) | Cancelar a mitad de vídeo largo detiene en &lt;2s razonables | `feat/cancel-vad-checkpoints` |
| A4 | Una sola verdad **Segment ↔ EDL** (o bloquear edits que no recompilan EDL) | Preview cortado y export usan los mismos keep ranges tras K/X | `fix/edl-segment-sync` |
| A5 | Progreso export con parse de `time=` FFmpeg (opcional) | Barra % avanza durante encode, no solo fases | `feat/export-ffmpeg-progress` |

---

## Sprint B — Calidad shorts (P1)

| ID | Tarea | Criterio de hecho | PR sugerido |
|----|--------|-------------------|-------------|
| B1 | SRT/VTT **first-class**: import en Welcome/Shorts + sidecar auto | Con .srt al lado, títulos y scores dejan de ser “habla Ns” | `feat/srt-first-class` |
| B2 | Whisper **explícito** (botón “Generar subtítulos”) no checkbox escondido | Un clic lanza Whisper; progreso; SRT queda en cache | `feat/whisper-on-demand-ui` |
| B3 | Unidades semánticas más estables (frases / gaps) | Menos clips basura; menos solapes | `feat/semantic-units-v2` |
| B4 | Diversidad / anti-cluster en preselect | No 5 clips del mismo minuto en “Por revisar” | `feat/clip-diversity` |
| B5 | Auto-framing 9:16 (centro rostro o ROI estático mejorado) | Export sin arrastrar marco en talking-head típico | `feat/auto-framing-v1` |

---

## Sprint C — Fábrica / lote (P1)

| ID | Tarea | Criterio de hecho | PR sugerido |
|----|--------|-------------------|-------------|
| C1 | Preset encode **rápido** vs **calidad** en lote | Batch usa veryfast/crf alto; export manual calidad | `feat/export-quality-presets` |
| C2 | Encode de clips en **pool 2** | 10 shorts no son 10× secuencial puro | `perf/parallel-clip-export` |
| C3 | Unificar copy **artefact shorts** vs **panel clipping** | Usuario entiende dos salidas distintas o se unifican | `docs+ux/two-shorts-systems` |
| C4 | Reabrir proyecto con run/EDL restaurado | Recientes no solo path: estado de corte | `feat/restore-project-run` |

---

## Sprint D — Plataforma (P2)

| ID | Tarea | Criterio de hecho |
|----|--------|-------------------|
| D1 | whisper.cpp embebido (sin CLI PATH) | Setup un comando; sin depender del PATH del usuario |
| D2 | LLM local solo títulos/hooks (no cortes ciegos) | Título legible con/sin SRT pobre |
| D3 | Editor policy packs en UI | CRUD sin editar JSON a mano |
| D4 | Tests frontend keep-ranges + reloj cortado | CI atrapa regresiones de preview |
| D5 | UI E2E Tauri (smoke open → export) | 1 smoke verde en CI |
| D6 | CI workflow con scope correcto + lint honesto | Push de `.github/workflows` funciona |
| D7 | Scope FS más estricto en release | No `**` en producción |
| D8 | Docs ARCHITECTURE/REVIEW al día | No contradicen ROADMAP |

---

## Orden de ejecución recomendado

```
A1 → A2 → B1 → B2 → A4 → B5 → C1 → C2 → (resto)
```

1. **A1** y **A2** → velocidad percibida en vídeos largos y lote.  
2. **B1/B2** → calidad de shorts (mayor techo).  
3. **A4** → confianza export = preview.  
4. **B5** → menos fricción 9:16.  
5. **C1/C2** → fábrica a escala.

---

## Explicitamente fuera de alcance inmediato

- NLE completo (multi-track, keyframes manuales de video).  
- Cloud / cuentas / sync.  
- Subtítulos quemados en vertical (salvo que se pida como epic).  
- Face tracking “broadcast” multi-persona (solo talking-head v1).

---

## Cómo usar este doc

- Marcar `[x]` al mergear el PR de cada fila.  
- Un PR ≈ un ID (A1, B1…).  
- No force-push; no merge a `main` sin ok del owner.

*Actualizado: 2026-07-20 — rama feat/intelligent-clipping.*
