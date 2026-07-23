# VigilCut — Revisión arquitectónica de CTO (2026-07-18)

**Rol:** Arquitecto principal / CTO  
**Naturaleza del producto:** Herramienta interna de fábrica de contenido IA (no comercial)  
**Objetivo de negocio:** Minimizar tiempo entre grabación y materiales listos para publicar  
**Filosofía:** *La IA trabaja. El humano únicamente supervisa.*  
**Horizonte:** 5 años · comprensión de vídeo, no solo recorte de silencios  

Este documento es el resultado de **tres iteraciones completas de análisis**, cada una diseñada para destruir las conclusiones de la anterior. Conserva el diagnóstico histórico.

## Actualización 2026-07-20 (v1.1.0 hardening)

| Diagnóstico histórico (este doc) | Estado 1.1 |
|----------------------------------|------------|
| Silero “placeholder” / fake path | **Corregido** — ONNX real + SHA-256 en setup:models |
| Batch force-accept silencioso | **Corregido** — modo Safe por defecto; Aggressive explícito |
| Segment como verdad de export | **Parcial** — factory path usa EDL; UI K/X aún dual |
| Stubs sin honestidad | **Parcial** — audio enhance en export; Whisper opt-in |
| Scopes Tauri abiertos | **Mejorado** — sin `**` global; `$HOME/**` y dirs estándar |
| Docs desalineadas | **En progreso** — ver CHANGELOG 1.1 + HARDENING_1_1.md |

**Vigente aún:** dual Segment/EDL en UI, supervised batch resume UI, broker de features completo, face tracking.  
**Detalle:** `docs/HARDENING_1_1.md`.

---

## 0. Tres iteraciones internas (método)

### Iteración 1 — “Destruir el editor”

**Conclusión provisional:** VigilCut es un editor de timeline con un detector de silencios. Eso es el error fundacional. Hay que matar la metáfora NLE y construir un motor de eventos + políticas.

**Pruebas en el código actual:**

- El dominio central es `Segment { start, end, kind, decision }` con `kind ∈ {speech, silence, …}` y `decision ∈ {keep, cut, pending}`.
- El pipeline es `detect_silences → Vec<Segment> → export filter_complex`.
- El frontend es timeline + maintain/cut + export (UI de aprobación de cortes).
- Stubs de audio/color/subs/batch son comandos sueltos, no un bus de análisis.

**Fallo de la iteración 1 si se queda ahí:**

- “Solo eventos” no exporta vídeo solo: hace falta una proyección temporal continua (EDL / keep ranges).
- Un operario de fábrica aún necesita una UI de supervisión; el error no es tener UI, es que la UI **define** el modelo de dominio.

### Iteración 2 — “Destruir el monolitismo de segmentos”

**Conclusión provisional:** El segmento no es el átomo correcto. El átomo es el **evento de análisis** (Evidence). Los segmentos/EDL son **vistas derivadas** generadas por **políticas**.

**Contrataque a la iteración 1:**

- Con 15 detectores, un `SegmentKind` enum explota o se convierte en un cajón de sastre.
- Un mismo instante puede ser: silencio + muletilla + baja energía + no es CTA. Un segmento monovalente no puede expresar eso sin mentir.
- Keep/Cut en el segmento acopla **percepción** con **decisión de montaje**. Mezcla ontologías.

**Fallo de la iteración 2 si se queda ahí:**

- Sin un producto operativo mañana, la arquitectura ideal es vaporware.
- La fábrica necesita un **modo headless batch** más que un timeline bonito; la iteración 2 aún asume sesión interactiva.

### Iteración 3 — “Destruir el producto-app”

**Conclusión final (la que se defiende):**

1. **El producto no es la app Tauri.** El producto es un **Video Understanding Engine** (librería + CLI + jobs) con clientes delgados (desktop, futuro API local, scripts de fábrica).
2. **La verdad del sistema** es: Media → Features → Events → Policies → EDL → Renders/Artifacts.
3. **La UI de supervisión** solo muestra la **cola de excepciones** (baja confianza / conflicto de políticas), no “todos los tramos”.
4. **El segmento keep/cut del MVP actual es un atajo legítimo de v0.1**, pero es **deuda estructural** si se congela como modelo canónico.

Todo lo que sigue asume la iteración 3 como verdad operativa.

---

## 1. Diagnóstico general

### 1.1 Qué es hoy VigilCut (honesto)

Un **scaffold Tauri 2 + Svelte 5** (~4.3k LOC útiles) que:

1. Abre un vídeo local.
2. Detecta silencios (casi solo FFmpeg `silencedetect`; Silero es placeholder).
3. Materializa un timeline speech/silence con decisiones keep/cut.
4. Previsualiza saltando cortes en el cliente.
5. Exporta concatenando rangos KEEP vía `filter_complex`.

Es un **MVP de recorte supervisado de silencios** envuelto en promesas de editor extensible.

### 1.2 Qué debería ser (según el contexto real del dueño)

Un **orquestador de fábrica de contenido**:

```
carpeta de crudos → análisis multi-detector → políticas de marca →
cola de supervisión mínima → render multi-formato (largo, short, caps, capítulos, títulos)
```

El humano no “edita”. El humano **audita excepciones** y **aprueba lotes**.

### 1.3 Veredicto en una frase

> **Estás construyendo la UI de un editor alrededor de un detector, cuando deberías construir un motor de comprensión de vídeo con una UI de supervisión como cliente secundario.**

Eso no invalida el MVP: invalida **elevar el MVP a arquitectura de 5 años sin reescritura del dominio**.

---

## 2. Fortalezas (qué no tirar a la basura)

| Fortaleza | Por qué importa |
|-----------|-----------------|
| Stack local (Tauri + Rust + FFmpeg) | Alineado con fábrica offline / privacidad / coste |
| Separación `commands` / `ffmpeg` / `pipeline` / `models` | Esqueleto correcto para crecer *si* el dominio cambia |
| Filosofía human-in-the-loop ya documentada | Compatible con “supervisión”, no con “edición creativa libre” |
| Export basado en rangos keep | Es el embrión correcto de un EDL |
| Preview skip-cuts en cliente | Barato y útil para confianza sin re-encode |
| Presets como objeto de dominio | Semilla de **Policy packs** |
| Batch job types (aunque sin worker) | Semilla de cola de fábrica |
| Licencia MIT + repo limpio | Irrelevante para uso interno, útil si más adelante se comparte |

**Conclusión:** El **stack** y la **intención** son salvables. El **modelo de dominio** y el **centro de gravedad del producto (UI)** no lo son a 5 años.

---

## 3. Debilidades estructurales

### 3.1 El dominio miente

`Segment` une tres conceptos distintos:

1. **Observación** (`kind`, `confidence`, `energy_db`) — qué creemos que hay.
2. **Decisión de montaje** (`decision`) — qué hacemos en el cut final.
3. **UI state** (selección, touched, labels) — cómo se presenta.

Cuando añadas muletillas, CTA, shorts y capítulos, un segmento “es silencio y es corte y es no-short y es capítulo 3” colapsa.

### 3.2 Un solo pipeline concreto, N promesas abstractas

```
pipeline/
  silence.rs   ← real
  export.rs    ← real
```

Audio, color, whisper, batch: **comandos stub**, no pipeline. Eso enseña al equipo (tú) a añadir features como `commands/foo.rs` sueltos → monólito de handlers.

### 3.3 `SegmentKind` enum cerrado

```rust
Speech | Silence | Music | Noise | ClipCandidate | Manual
```

Cada nuevo detector fuerza:

- cambiar el enum,
- cambiar TS types,
- cambiar colores UI,
- cambiar lógica de export implícita.

Eso es el anti-patrón de plugin.

### 3.4 Frontend = dueño del workflow

`project.svelte.ts` orquesta open → detect → review → export. El backend no tiene un concepto de **Job** o **AnalysisRun**. Sin job graph, no hay fábrica batch seria ni reintentos ni cache por detector.

### 3.5 Silero “arquitectura lista” es ficción operativa

El código dice preferir Silero y luego **siempre** llama a FFmpeg. Eso es deuda de credibilidad: la arquitectura no está “lista”; está **anotada**.

### 3.6 Export y análisis no comparten un grafo de artefactos

No hay:

- hash de media,
- cache de wav 16 kHz,
- cache de transcript,
- versionado de detector,
- invalidación.

Cada detector futuro re-decodificará el mundo.

### 3.7 UX de editor vs job de fábrica

Incluso mejorada (Sprint A), la UX asume:

> un humano revisa tramo a tramo.

Eso es **O(n segmentos)**. Con 40 silencios/vídeo × 20 vídeos/día = muerte por clics. Viola “cada clic debe justificarse”.

---

## 4. Respuestas a las 10 preguntas (sin condescendencia)

### 1. ¿Editor o motor de análisis?

**Motor de análisis + proyecciones de montaje.**  
La app desktop es un **visor de supervisión y un lanzador de jobs**, no el núcleo.

Si en 12 meses la UI sigue siendo el 70% del esfuerzo, el producto habrá perdido.

### 2. ¿El modelo de segmentos aguanta decenas de detectores?

**No como fuente de verdad.**  
Sí como **vista materializada** (p.ej. “regiones de silencio propuestas para corte”) generada desde eventos.

### 3. ¿Eventos inteligentes en lugar de segmentos?

**Sí, con matiz:**

| Capa | Concepto |
|------|----------|
| L0 | Media + features (pcm, frames sample, ASR tokens) |
| L1 | **Events** (intervalos/puntos con type + score + payload) |
| L2 | **Policies** (reglas → acciones de montaje) |
| L3 | **EDL / Cutlist** (keep ranges + effects + tracks) |
| L4 | **Artifacts** (mp4, srt, chapters.json, shorts[], titles) |

Los “segmentos” del MVP son un L3 degenerado mezclado con L1.

### 4. ¿Pipeline para añadir detectores sin tocar el resto?

```
Ingest → FeatureExtractors → DetectorPlugins (parallel) → EventStore
       → PolicyEngine → EDLCompiler → Renderers → ArtifactStore
```

Contratos:

- `Detector: (MediaContext) -> Vec<Event>`
- `Policy: (EventStore, Profile) -> Vec<EditOp>`
- `Renderer: (EDL, Profile) -> Artifact`

Nadie importa el interior de otro detector.

### 5. ¿Qué limita el crecimiento hoy?

| Límite | Efecto |
|--------|--------|
| `Segment` canónico | Multi-label imposible sin hack |
| Detección síncrona en `openMedia` | UI bloqueada; no batch real |
| Estado solo en memoria + JSON plano | No query, no diff, no lineage |
| FFmpeg filter_complex monstruo | Frágil con muchos cortes / A-V drift |
| Preview solo client-side | No valida el mismo path que export |
| Un store Svelte dios | Frontend monólito |
| Commands flat en `lib.rs` | Superficie IPC sin bounded contexts |

### 6. ¿Responsabilidades mezcladas?

| Sitio | Mezcla |
|-------|--------|
| `Segment` | observación + decisión |
| `openMedia` (store) | IO + análisis + persistencia + UX focus |
| `export.rs` | EDL compile + ffmpeg args + color |
| `VideoPreview` | player + motor de timeline editado + seek policy |
| `ProcessingPreset` | silence + audio + color + export en un blob |
| `Project` | media + segmentos + preset + subs (god object light) |

### 7. ¿Qué debería ser plugin?

- Detectores (silence, filler, ASR, emotion, energy, topic, NER, CTA, short candidates…)
- Policies (YouTubeTalkingHead, PodcastAggressive, ShortsFactory)
- Renderers (longform mp4, vertical 9:16, srt burn-in, chapters youtube)
- Exporters de metadata (JSON-LD, description templates)
- Storage backends (fs local hoy; sqlite mañana)

### 8. ¿Detectores 100% independientes?

Cada detector:

```
plugins/<id>/
  manifest.toml   # name, version, inputs, outputs, cost
  run             # or Rust dyn trait loaded statically first
```

**Inputs declarados:** `need_audio_16k`, `need_transcript`, `need_rgb_1fps`…  
El orchestrator materializa features una vez y las reparte.

**Outputs:** solo `Event[]` + metrics. Nunca mutan el EDL directamente.

### 9. ¿Cómo no ser monólito en 2 años?

1. **Core crate** sin UI (`vigilcut-engine`).
2. **IPC estable** versionado (JSON schema de Event/EDL).
3. **Detectores sin acceso a Project UI types**.
4. **Prohibido** añadir `SegmentKind::Foo`.
5. **Jobs** con estado en sqlite (o carpeta de job con journal).
6. **ADR** por cada detector y cada policy.
7. Test de contrato: fixture de 30s → eventos golden.

### 10. ¿Qué haría distinto desde cero mañana?

1. Empezar por **CLI**: `vigilcut run ./inbox --profile youtube`.
2. Modelo Event/Policy/EDL desde el día 1.
3. Un detector (silence) + un policy + un renderer.
4. UI **solo** exception queue + play result + approve batch.
5. No timeline full-segment hasta que el volumen de excepciones lo exija.
6. Cache de features con content-hash.
7. Export por **concat demuxer** o segment files, no solo un filter_complex gigante.

---

## 5. Riesgos

### 5.1 Arquitectónicos

| Riesgo | Severidad | Señal de alarma |
|--------|-----------|-----------------|
| Segment como verdad | Crítica | Cada feature añade un campo a Segment |
| UI-driven domain | Crítica | Nuevas reglas de negocio en Svelte |
| Plugin theater (stubs) | Alta | 10 commands vacíos “para el futuro” |
| Acoplar ASR al timeline UI | Alta | Whisper escribe Segment[] directo |
| Un solo Project JSON | Media | Archivos de 50MB, merges imposibles |

### 5.2 Técnicos

| Riesgo | Severidad | Notas |
|--------|-----------|-------|
| A-V desync en muchos cortes | Alta | filter_complex concat es frágil |
| Memoria waveform/peaks en UI | Media | Vídeos largos |
| Sin progress events reales | Media | UX de fábrica rota |
| Modelos ML no versionados | Alta | Resultados no reproducibles |
| Windows path + asset protocol | Media | Ya dolió en preview |

### 5.3 Escalabilidad (fábrica)

| Riesgo | Severidad | Notas |
|--------|-----------|-------|
| Revisión O(n) tramos | Crítica | Mata el throughput diario |
| Un vídeo por sesión UI | Alta | Necesitas inbox → outbox |
| Sin paralelismo de detectores | Alta | 8 detectores en serie = lento |
| Sin cache de features | Alta | Re-análisis re-cobra todo |
| Export re-encode siempre | Media | Cuello de botella GPU/CPU |

### 5.4 UX (en clave fábrica, no “usuarios”)

| Riesgo | Severidad | Notas |
|--------|-----------|-------|
| Cada silencio pide atención | Crítica | Viola filosofía de supervisión |
| Preview cortado opt-in mental | Alta | Export a ciegas |
| No hay cola de excepciones | Crítica | Todo es excepción o nada |
| No hay “aprobar lote” | Alta | Un archivo ≠ fábrica |
| Post-export débil aún | Media | Sprint A ayuda, no cierra factory loop |

---

## 6. Componentes: eliminar / fusionar / crear

### 6.1 Eliminar (como conceptos de dominio o código muerto)

| Qué | Por qué |
|-----|---------|
| `SegmentKind` enum cerrado como API pública | Sustituir por `event_type: string` namespaced |
| `decision` dentro del “segmento de detección” | La decisión vive en EDL / EditOp |
| Components huérfanos (`Inspector`, `PresetPanel`, `SegmentList` si ya no se usan) | Basura cognitiva |
| Stubs que fingen features (whisper error, silero fake path) | Mentira operativa; mejor no exponer |
| `ProjectMode` ad-hoc sin engine | Sustituir por Policy profile id |
| Metáfora “editor de timeline completo” en roadmap | Distrae del job de fábrica |

### 6.2 Fusionar

| Fusionar | En |
|----------|----|
| `detect_silences` + futuros detectores | `AnalysisOrchestrator.run(profile)` |
| `preview_skip_cuts` + `estimate_export` + compile keep | `EdlCompiler` |
| `ProcessingPreset` monolítico | `PolicyPack` + `RenderProfile` separados |
| TopBar stats + ActionBar progress | **JobStatus** strip (fábrica) |
| create/load/save project ad-hoc | **Job** + **Run** persistence |

### 6.3 Crear (núcleo nuevo)

| Componente | Responsabilidad |
|------------|-----------------|
| `vigilcut-engine` (crate) | Orquestación sin UI |
| `MediaRegistry` | id, path, hash, duration, codecs |
| `FeatureStore` | wav16k, transcript, frames, embeddings paths |
| `EventStore` | append-only events por run |
| `Detector` trait + registry | plugins |
| `PolicyEngine` | events → EditOps |
| `EdlCompiler` | EditOps → Cutlist |
| `RenderPlanner` | Cutlist → ffmpeg/graph plan |
| `JobQueue` | batch inbox/outbox |
| `ExceptionQueue` | solo items low-confidence / conflicts |
| `ArtifactManifest` | qué se generó y de qué run |
| UI: **Supervisor** (no Editor) | excepciones + play result + approve |

---

## 7. Arquitectura ideal (5 años)

```
                    ┌─────────────────────────────┐
                    │  Clients                    │
                    │  - Supervisor UI (Tauri)    │
                    │  - CLI factory              │
                    │  - (opcional) local HTTP    │
                    └──────────────┬──────────────┘
                                   │ JSON-RPC / IPC
                    ┌──────────────▼──────────────┐
                    │  vigilcut-engine            │
                    │  JobController              │
                    └──────────────┬──────────────┘
           ┌───────────────────────┼───────────────────────┐
           ▼                       ▼                       ▼
    FeatureBroker           DetectorRuntime           PolicyEngine
           │                       │                       │
           ▼                       ▼                       ▼
    FeatureStore              EventStore               EditOps
                                                           │
                                                           ▼
                                                      EdlCompiler
                                                           │
                                                           ▼
                                                      Renderers
                                                           │
                                                           ▼
                                                      Artifacts/
```

### Principios no negociables

1. **UI never owns truth** — solo proyecta jobs/events/edl.
2. **Detectors never cut video** — solo emiten evidencia.
3. **Policies never decode media** — solo leen events.
4. **Renders are pure functions of EDL + media**.
5. **Every run is reproducible** (versions + hashes).
6. **Human time is the scarcest resource** — default auto, review exceptions.

---

## 8. Pipeline ideal

### 8.1 Job lifecycle

```
Queued → Ingesting → ExtractingFeatures → Detecting →
Policy → CompileEdl → (ExceptionReview?) → Rendering → Done | Failed
```

### 8.2 Parallelism

```
Features:
  audio_16k ──┬──► silence_detector
  transcript ─┼──► filler_detector
              ├──► topic_detector
  energy ─────┼──► energy_peaks
  vision ─────┴──► (future) scene/emotion

Barrier → PolicyEngine(profile) → EDL → Renders[]
```

### 8.3 Exception review (el único “loop humano”)

Un item de excepción:

```json
{
  "id": "...",
  "event_ids": ["..."],
  "reason": "low_confidence | policy_conflict | duration_edge",
  "suggested_ops": [{"op": "cut", "start": 12.1, "end": 13.4}],
  "confidence": 0.61,
  "preview": {"source_start": 11.5, "source_end": 14.0}
}
```

El humano: **Accept / Reject / Edit bounds** — no “recorrer 80 tramos verdes”.

### 8.4 Auto thresholds (fábrica)

| Confianza | Acción |
|-----------|--------|
| ≥ 0.9 | auto-apply policy |
| 0.7–0.9 | auto-apply + log |
| < 0.7 | exception queue |
| conflict | exception queue |

Los umbrales viven en el **PolicyPack**, no hardcodeados en UI.

---

## 9. Modelo de datos ideal

### 9.1 Event (L1) — fuente de verdad analítica

```json
{
  "id": "evt_01H...",
  "run_id": "run_01H...",
  "media_id": "med_01H...",
  "detector": "silence@1.2.0",
  "type": "audio.silence",
  "span": { "start": 12.04, "end": 13.88, "unit": "seconds" },
  "score": 0.93,
  "payload": { "noise_db": -42.1, "method": "silero" },
  "tags": ["removable_candidate"]
}
```

`type` es string namespaced (`audio.filler`, `speech.cta`, `structure.chapter`, `short.candidate`).

### 9.2 EditOp (L2 salida de policy)

```json
{
  "id": "op_...",
  "op": "remove_span | keep_span | force_cut | insert_marker | emit_short",
  "span": { "start": 12.04, "end": 13.88 },
  "priority": 100,
  "source_event_ids": ["evt_..."],
  "rationale": "silence > 0.4s with padding 0.12"
}
```

### 9.3 EDL / Cutlist (L3)

```json
{
  "media_id": "med_...",
  "video_track": [
    { "source_start": 0.0, "source_end": 12.04 },
    { "source_start": 13.88, "source_end": 64.2 }
  ],
  "markers": [
    { "at_output": 0.0, "type": "chapter", "label": "Intro" }
  ],
  "derived": {
    "output_duration": 62.36,
    "removed_duration": 1.84
  }
}
```

### 9.4 Artifact manifest (L4)

```json
{
  "job_id": "job_...",
  "artifacts": [
    { "kind": "longform_mp4", "path": "out/video.mp4" },
    { "kind": "shorts", "path": "out/shorts/01.mp4", "span_source": [40.1, 55.0] },
    { "kind": "chapters_json", "path": "out/chapters.json" },
    { "kind": "srt", "path": "out/captions.srt" }
  ]
}
```

### 9.5 Por qué esto escala a “comprender vídeo”

- Nuevos detectores = nuevos `type` + payload, **cero** cambio en EDL schema.
- Nuevas salidas (shorts, capítulos) = nuevos renderers leyendo events/markers.
- Entrenamiento futuro / LLM = otro detector que emite events.
- El humano nunca ve el raw event firehose salvo excepciones.

---

## 10. UX ideal (fábrica, no prosumer)

### Pantallas (solo 4)

1. **Inbox** — carpetas/archivos pendientes, perfiles, “Procesar lote”.
2. **Run progress** — features/detectores/policies con % y ETA.
3. **Exceptions** — 0–N items; si 0, skip a Done.
4. **Done** — artifacts, abrir carpeta, métricas (tiempo ahorrado).

### Anti-pantallas

- Timeline denso de 200 segmentos como vista principal.
- Inspector de codec en el camino feliz.
- Ajustes Silero en cada sesión (van al profile de fábrica).

### Métrica de éxito UX interna

> **Segundos de atención humana por minuto de vídeo crudo.**  
> Objetivo año 1: < 5 s/min. Año 3: < 1 s/min.

Si la UI invita a revisar cada silencio, la métrica falla por diseño.

---

## 11. Roadmap técnico

### Fase 0 — Congelar el daño (1 semana)

- Documentar que `Segment` es **legacy view**.
- Prohibir nuevos `SegmentKind`.
- Extraer `keep_ranges_from_decisions` como único puente a export.
- Eliminar components muertos.
- Añadir content-hash de media en project.

### Fase 1 — Engine extract (2–3 semanas)

- Crate `vigilcut-engine` usado por Tauri commands.
- Introducir `Event` + `AnalysisRun` aunque solo silence escriba events.
- Compilar EDL desde events+policy (policy v1 = “cut silence”).
- CLI: `vigilcut analyze x.mp4 -o run/`.

### Fase 2 — Job queue real (2 semanas)

- Sqlite o filesystem journal de jobs.
- Worker async con progress events al UI.
- Inbox folder watch (opcional).
- Batch export multi-artifact.

### Fase 3 — Detectors platform (continuo)

- Trait + registry.
- Feature broker (wav16k, whisper).
- Detector filler + energy peaks.
- Exception queue UI.

### Fase 4 — Comprehension layer

- Topic/chapter markers.
- Short candidates.
- CTA / memorable line via LLM local or rules on transcript.
- Multi-render profiles.

### Fase 5 — Factory hardening

- Reproducibility locks.
- Telemetry local (tiempos, tasas de excepción) — solo disco local.
- GPU optional paths.
- A/B de policies con métricas de retención internas.

---

## 12. Roadmap funcional (valor de fábrica)

| Prioridad | Capacidad | Valor |
|-----------|-----------|-------|
| P0 | Silence auto-cut + exception only | Tiempo base |
| P0 | Batch folder → outbox | Throughput |
| P0 | Preview = mismo EDL que export | Confianza |
| P1 | Filler/muletillas | Calidad habla |
| P1 | Captions (ASR) | Material de publicación |
| P1 | Chapters | SEO / UX viewer |
| P2 | Shorts candidates | Multi-formato |
| P2 | Breath / low energy trims | Densidad |
| P2 | Titles/description draft from transcript | Post-producción copy |
| P3 | Emotion / story beats | Editorial IA avanzada |

---

## 13. Plan de migración desde el estado actual

### Principio: strangler fig

No big-bang rewrite de Tauri/Svelte en un fin de semana. **Estrangular el dominio.**

```
Hoy:
  UI → detect_silences → Segment[] → export

Mañana:
  UI → engine.run → (Events, EDL) → UI proyecta SegmentView opcional → export(EDL)

Pasado:
  CLI/UI → engine jobs → ExceptionQueue → export multi-artifact
```

### Pasos concretos

1. **Introducir `Event` y `Edl` en Rust** sin borrar `Segment`.
2. **Silence detector** escribe `audio.silence` events y **también** genera `Segment[]` legacy para la UI actual.
3. **Export** cambia a consumir `Edl` (internamente igual que keep ranges).
4. **Policy v1** lee events silence → remove_span; UI toggles escriben **overrides** de policy, no mutan “kind”.
5. **UI**: modo “Supervisor” nuevo; el timeline legacy queda como “Advanced”.
6. **Cuando ExceptionQueue cubra el 95% de casos**, deprecar timeline full-list.
7. **Extraer CLI** compartiendo engine.
8. **Borrar** `Segment.decision` del modelo canónico (breaking interno OK: herramienta tuya).

### Qué NO migrar

- No rehacer Tailwind/Svelte “porque sí”.
- No microservicios.
- No cloud.
- No plugin WASM el día 1 (static registry en Rust basta años).

---

## 14. Cambios priorizados por impacto (para ti, operario diario)

| Rank | Cambio | Impacto en tiempo humano | Esfuerzo | Tipo |
|------|--------|---------------------------|----------|------|
| 1 | **Auto-approve high-confidence silence**; UI solo excepciones | Enorme | M | Producto+domain |
| 2 | **Batch inbox → outbox** | Enorme | L | Engine+jobs |
| 3 | **Event/EDL split** (strangler) | Enorme a 12 meses | L | Arquitectura |
| 4 | **Feature cache (wav/transcript)** | Alto | M | Perf |
| 5 | **Mismo EDL en preview y export** | Alto (confianza) | M | Correctness |
| 6 | **CLI factory** | Alto | M | Workflow |
| 7 | **Filler detector** | Alto | M | Detector plugin |
| 8 | **Captions artifact** | Alto | M–L | Detector+render |
| 9 | **Matar review O(n) como default** | Enorme | M | UX philosophy |
| 10 | Polish timeline/editor UX | Bajo para fábrica | S | Evitar como foco |

**Nota dura:** Seguir puliendo el timeline de tramos es **optimizar el cuello de botella equivocado**. El cuello es **atención humana por vídeo**, no la belleza del scrubber.

---

## 15. Desafío directo a tus decisiones actuales

### “Human-in-the-loop en cada segmento”

**Equívoco para fábrica.**  
HITL debe ser **human-on-the-exception-loop**. Si confías en el detector al 93%, revisar cada silencio es teatro de control, no control.

### “Un timeline unificado lo aguantará todo”

**No.**  
Un timeline unificado es una **visualización**, no un modelo. Si lo conviertes en base de datos, en 18 meses tendrás Photoshop del audiovisual sin su equipo.

### “Tauri app = el producto”

**No.**  
El producto es el **engine que transforma crudos en artefactos publicables**. La app es el mando de la fábrica. Si el engine no existe fuera de la UI, no tienes fábrica: tienes un pasatiempo de edición asistida.

### “Empezar por silencios valida la arquitectura”

**Solo valida FFmpeg + export.**  
No valida multi-detector, ni batch, ni exception UX, ni artifact matrix. El riesgo es **extrapolación falsa de éxito del MVP**.

---

## 16. Arquitectura de referencia (código mental)

```rust
// core ideas — not current code

trait Detector: Send + Sync {
    fn id(&self) -> &str;
    fn requires(&self) -> FeatureSet;
    fn detect(&self, ctx: &MediaContext) -> Result<Vec<Event>>;
}

trait Policy: Send + Sync {
    fn id(&self) -> &str;
    fn propose(&self, events: &EventStore) -> Vec<EditOp>;
}

trait Renderer: Send + Sync {
    fn id(&self) -> &str;
    fn render(&self, edl: &Edl, media: &MediaRef) -> Result<Artifact>;
}
```

UI:

```
SupervisorView {
  jobs: JobList,
  active: Option<ExceptionQueue>,
  result: Option<ArtifactGallery>,
}
```

---

## 17. Conclusión ejecutiva

| Pregunta | Respuesta |
|----------|-----------|
| ¿El proyecto actual es basura? | **No.** Es un MVP útil de silence-cut. |
| ¿Es la base correcta a 5 años? | **No, si Segment+UI-editor se congelan como verdad.** |
| ¿Hay que reescribir todo mañana? | **No big-bang.** Sí **strangler del dominio** y **engine-first**. |
| ¿La decisión más importante? | Dejar de optimizar un editor; construir **motor + cola de excepciones + batch**. |
| ¿Qué mataría el proyecto? | Añadir 10 detectores como `SegmentKind` y 10 paneles UI sin Event/Policy/EDL. |

### Mandato de arquitectura (permanente)

1. **Events in, policies decide, EDL cuts, renderers emit.**  
2. **Humans review exceptions, not timelines by default.**  
3. **Engine is the product; UI is a client.**  
4. **No new detector without a plugin contract.**  
5. **Measure human-seconds per media-minute.**

---

## 18. Próxima decisión que te pediría como CTO

Antes de otra feature de timeline, elegir **una**:

**A.** Implementar `Event` + `Edl` + silence como primer detector “de verdad” (strangler).  
**B.** Implementar batch inbox/outbox sobre el modelo actual (valor fábrica inmediato, deuda controlada).  
**C.** Implementar exception-only UX (auto-cut silence ≥ threshold, lista solo low-confidence).

Recomendación CTO para tu caso (fábrica diaria): **C + B en paralelo superficial, A como cimiento en la misma iteración de 2–3 semanas** — C/B dan oxígeno operativo; A evita la muerte por monólito.

---

*Documento vivo del proyecto. Cualquier PR que contradiga la sección 17 debe incluir un ADR que justifique la excepción.*
