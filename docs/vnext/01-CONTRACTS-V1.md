# VigilCut vNext — Contratos V1

**Fase:** 1 (solo diseño)  
**Fecha:** 2026-07-29  
**Fuente de producto/arquitectura:** `docs/VIGILCUT_DESIGN_REVIEW.md`  
**Estado:** contratos de diseño; **no implementados** en Rust/TypeScript en esta fase.

---

## 0. Decisiones transversales (justificadas)

| Decisión | Elección | Justificación |
|---|---|---|
| Formato de IDs | UUID v4 en string canónico (36 chars, minúsculas) | Ya es el patrón de `Project`, `ClippingRun`, `generation_jobs`, candidatos. Evita secuencias y colisiones multi-máquina. |
| Timestamps de estado | ISO-8601 / RFC 3339 en UTC, serializados como `string` | Coherente con jobs visuales (`created_at` TEXT) y con `chrono` en `Project`/`BatchJob`. Fácil de leer en SQLite. |
| Tiempo de media (spans, duración) | **Segundos** como `number` IEEE-754 (`f64`), no milisegundos enteros | `ClipCandidate.start/end`, `Span`, EDL y export FFmpeg usan segundos. Cambiar a ms rompería equivalencia con clipping legacy. La UI puede redondear a 1 ms en display. |
| Rutas de archivos | **Absolutas** en contratos de snapshot y manifests MVP | Windows es target principal; paths relativos dependen del CWD del proceso Tauri y son frágiles. Opcionalmente se registra `work_dir` del proyecto para futuros paths relativos *dentro* del work tree. |
| Hashes de archivo | SHA-256 hex minúscula (64 chars) | Ya usado en `media_assets.sha256` y candidatos visuales. |
| Serialización API/TS | JSON **camelCase** | Patrón actual `#[serde(rename_all = "camelCase")]` y `src/lib/types/index.ts`. |
| Campos anidados variables | JSON embebido en columnas SQLite + tipos tipados en contratos | Evita explosión de tablas en MVP sin perder tipado en aplicación. |
| Versionado de contrato | Campo `contractVersion: "v1"` en cada snapshot/manifest | Permite deserializar con rechazo explícito de versiones desconocidas. |
| Compatibilidad | Lectura: rechazar `contractVersion` ≠ v1 hasta haber adaptador. Escritura: solo v1. | No se hace dual-write legacy↔vNext en Fase 1. |
| Errores | `ApplicationErrorV1` tipado (código + mensaje + contexto) | Sustituye `unknown` y strings sueltos en la API vNext. |
| Progreso | `JobProgressV1` con `jobId` obligatorio | Corrige el progreso global actual (`JobProgress.job` sin id de fila durable). |
| ReviewDecision | **Append-only** (INSERT only; nunca UPDATE/DELETE de filas de decisión) | Una decisión humana no desaparece al cambiar estado en memoria. |
| RenderPlan / Artifact final | **Inmutables** tras creación/finalización | Nueva versión = nuevo id. |
| Completar Job | Prohibido sin `resultArtifactId` validado | Invariante de dominio y de persistencia. |
| Idempotency de Job | `idempotencyKey` UNIQUE por scope de aplicación | Reutiliza el patrón de `generation_jobs.idempotency_key`. |
| Fuente de verdad | SQLite metadata/estado; filesystem bytes | UI y eventos solo reflejan. |

### Relación con legacy (no eliminar)

| Legacy | Relación con v1 |
|---|---|
| `Project` (`project.json`) | No es `ContentProject`. Puede coexistir; un ContentProject puede referenciar `legacyProjectId` opcional. |
| `ClippingRun` (memoria) | Fuente de campos para `ShortCandidateV1` + recipe de clipping; se persistirá en fases 2–3, no en Fase 1. |
| `ClipCandidate` | Mapeo 1:1 conceptual → `ShortCandidateV1` (renombre de producto). |
| `AnalysisRun` (JSON disco) | Input posible de un job `transcribe`/`analyze`; no es contrato vNext de UI. |
| `BatchJob` (memoria) | Fuera del MVP de un Short; no se modela como Job v1 de producto todavía. |
| `generation_jobs` / library.db | Dominio **Biblioteca Visual**, no mezclar con ContentProject del cliente. Patrón lease/idempotencia se **copia conceptualmente**, no se reutiliza la misma tabla. |
| `ArtifactRef` | Precursor pobre; `ArtifactManifestV1` lo reemplaza con hash, probe y linaje. |
| `JobProgress` TS (`job`/`stage`) | Reemplazado por `JobProgressV1` con `jobId`. |

### Fuera de contratos activos del MVP

Idea, ScriptVersion editorial, Scene, VisualRequirement, HorizontalArtifact — solo evolución futura; **sin campos** en estos contratos v1.

---

## 1. ContentProjectSnapshotV1

### Propósito
Raíz de una producción de cliente en vNext: un video fuente (o conjunto mínimo de entradas), política de privacidad local, y punteros a la próxima acción — no un “workspace de herramienta”.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | Versión del contrato. |
| `id` | string (UUID) | sí | Id del content project. |
| `title` | string | sí | Nombre humano. |
| `clientLabel` | string \| null | no | Etiqueta de cliente (no CRM). |
| `privacyMode` | enum | sí | `local_only` (único valor MVP). |
| `sourceMedia` | SourceMediaRefV1 | sí | Entrada principal. |
| `workDir` | string (path abs.) | sí | Directorio de trabajo del proyecto (caches, temps de jobs, exports). |
| `status` | enum | sí | `active` \| `archived`. |
| `nextAction` | enum \| null | no | Sugerencia UI: `ingest` \| `transcribe` \| `generate_candidates` \| `review` \| `render` \| `done` \| `resolve_failed_job`. Derivada, no segunda verdad. |
| `activeRecipeId` | string \| null | no | Recipe vigente. |
| `legacyProjectId` | string \| null | no | Puente opcional a `Project.id` JSON. |
| `createdAt` | string RFC3339 | sí | |
| `updatedAt` | string RFC3339 | sí | |

#### SourceMediaRefV1 (embebido)

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `path` | string abs. | sí | Ruta del archivo fuente del cliente. **No se borra automáticamente.** |
| `sha256` | string \| null | no | Tras ingest/probe. |
| `durationS` | number \| null | no | Segundos de duración (ffprobe). |
| `width` | number \| null | no | px |
| `height` | number \| null | no | px |
| `hasAudio` | boolean \| null | no | |
| `hasVideo` | boolean \| null | no | |
| `mimeOrContainer` | string \| null | no | p.ej. `mp4` |

### Enums
- `privacyMode`: `local_only`
- `status`: `active`, `archived`
- `nextAction`: ver arriba

### Unidades
- Duración/probe: segundos (`durationS`).
- Dimensiones: píxeles enteros.

### Invariantes
1. `workDir` ≠ `sourceMedia.path` como archivo (workDir es directorio).
2. `privacyMode` en MVP es siempre `local_only`.
3. No almacena transcript completo en este snapshot (va a artifacts o tablas de job result).
4. No es dueño de jobs: lista de jobs se consulta aparte.

### Serialización
JSON camelCase. TypeScript: interface `ContentProjectSnapshotV1` sin `unknown`.

### Relación legacy
Coexiste con `Project` / `project.json`. No reemplaza silence-cut en Fase 1–3.

---

## 2. ProductionRecipeV1

### Propósito
Snapshot **versionado e inmutable** de decisiones repetibles de producción (perfiles de clipping, preset de subtítulos, codec/target de render). Una edición de recipe crea un **nuevo** id.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `id` | string UUID | sí | |
| `contentProjectId` | string UUID | sí | FK lógica. |
| `label` | string | sí | p.ej. `default-short-v1`. |
| `clipping` | ClippingRecipeV1 | sí | Perfiles y pads. |
| `subtitles` | SubtitlesRecipeV1 | sí | Preset cerrado MVP. |
| `render` | VerticalRenderRecipeV1 | sí | Target 1080×1920, codecs. |
| `createdAt` | string RFC3339 | sí | |
| `supersedesRecipeId` | string \| null | no | Recipe anterior si es revisión. |

#### ClippingRecipeV1

| Campo | Tipo | Oblig. | Notas |
|---|---|---|---|
| `durationProfile` | enum | sí | `micro` \| `short` \| `standard` \| `extended` \| `custom` (mirrors `DurationProfile`) |
| `selectionProfile` | enum | sí | `conservative` \| `balanced` \| `broad` \| `exploratory` |
| `minDurationS` | number \| null | no | Override segundos |
| `idealDurationS` | number \| null | no | |
| `maxDurationS` | number \| null | no | |
| `padBeforeS` | number | sí | default 0.25 |
| `padAfterS` | number | sí | default 0.35 |
| `maxCandidates` | number | sí | entero ≥ 1 |
| `preferWhisper` | boolean | sí | |

#### SubtitlesRecipeV1

| Campo | Tipo | Oblig. | Notas |
|---|---|---|---|
| `presetId` | string | sí | MVP: un solo preset aprobado, p.ej. `safe_center_bottom_v1` |
| `burnIn` | boolean | sí | MVP true para entregable cliente |
| `maxLines` | number | sí | |
| `safeMarginPct` | number | sí | 0–1, zona segura |

#### VerticalRenderRecipeV1

| Campo | Tipo | Oblig. | Notas |
|---|---|---|---|
| `width` | number | sí | **1080** |
| `height` | number | sí | **1920** |
| `videoCodec` | string | sí | `libx264` |
| `audioCodec` | string | sí | `aac` |
| `audioBitrateKbps` | number | sí | p.ej. 160 |
| `crf` | number | sí | p.ej. 20 |
| `pixelFormat` | string | sí | `yuv420p` |
| `faststart` | boolean | sí | true |

### Invariantes
1. Recipe **inmutable** tras insert: no UPDATE de campos de contenido.
2. `width`×`height` del MVP vertical = 1080×1920.
3. No contiene paths de output ni comandos FFmpeg.

### Relación legacy
Mapea `ClippingOptions` + `ClipFraming` defaults + parámetros de `export_clips.rs`.

---

## 3. JobSnapshotV1

### Propósito
Unidad durable de trabajo. Se **registra antes de ejecutar**. La UI no es dueña del ciclo de vida.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `id` | string UUID | sí | |
| `contentProjectId` | string UUID | sí | |
| `kind` | enum | sí | Ver kinds MVP. |
| `status` | JobStatusV1 | sí | |
| `idempotencyKey` | string | sí | Única en scope global vNext DB. |
| `attempt` | number | sí | Entero ≥ 0; se incrementa al claim. |
| `maxAttempts` | number | sí | Entero ≥ 1. |
| `priority` | number | sí | Menor = antes; default 100. |
| `input` | object (JSON tipado por kind) | sí | Referencias a paths/ids, no bytes. |
| `resultArtifactId` | string \| null | no | **Obligatorio si status=completed**. |
| `renderPlanId` | string \| null | no | Si aplica. |
| `error` | ApplicationErrorV1 \| null | no | Último error estructurado. |
| `stage` | string | sí | Etapa humana-legible (p.ej. `probing`, `encoding`). No es status. |
| `progressPct` | number | sí | 0–100. |
| `lockedBy` | string \| null | no | Worker/lease holder. |
| `leaseExpiresAt` | string RFC3339 \| null | no | |
| `cancelRequested` | boolean | sí | |
| `createdAt` | string RFC3339 | sí | |
| `updatedAt` | string RFC3339 | sí | |
| `startedAt` | string RFC3339 \| null | no | |
| `finishedAt` | string RFC3339 \| null | no | |

### JobStatusV1 (mínimo obligatorio)

```
queued | running | waiting_review | completed | failed | interrupted | cancelling | cancelled
```

### Kinds MVP (cerrado)

| kind | Descripción |
|---|---|
| `ingest_probe` | Hash + ffprobe de fuente |
| `transcribe` | ASR unificado → artifact transcript |
| `generate_short_candidates` | Clipping pipeline → candidates persistidos |
| `vertical_render` | Render 9:16 + burn-in según plan |

(Otros kinds se añaden en fases posteriores con migración de contrato, no ad-hoc en UI.)

### Transiciones permitidas (dominio puro)

| Desde | Hacia | Condición |
|---|---|---|
| `queued` | `running` | claim + lease |
| `running` | `waiting_review` | kind lo requiere (p.ej. candidatos listos) |
| `running` | `completed` | **resultArtifactId** presente y validado |
| `running` | `failed` | error terminal del intento |
| `running` | `interrupted` | lease expirado / crash recovery |
| `running` | `cancelling` | cancelRequested |
| `queued` | `cancelling` | cancel antes de claim |
| `cancelling` | `cancelled` | cleanup hecho o no-op |
| `failed` | `queued` | retry **explícito** (nuevo attempt o re-queue) |
| `interrupted` | `queued` | resume/retry **explícito** |
| `waiting_review` | `queued` o `running` | tras decisión que desbloquea siguiente job (Fase 3+; documentado) |

**Prohibido:** cualquier otra transición; en particular `* → completed` sin artifact validado.

### Idempotency key — reglas
- Formato sugerido: `{contentProjectId}:{kind}:{stablePayloadHash}` o keys de negocio (`render:{planId}:v1`).
- Re-crear job con la misma key devuelve el job existente si no está `cancelled` en estado terminal reutilizable, o falla de forma controlada — política exacta de “reopen” se implementa en Fase 2 tests.
- Un retry explícito puede usar key `…:attemptN` **o** reutilizar key solo si el job previo está `failed`/`interrupted` y se reencola sin nuevo output final.

### Invariantes
1. Insert en DB **antes** de side-effects de media.
2. `completed` ⇒ `resultArtifactId` NOT NULL y artifact `validationStatus = passed`.
3. `progressPct` no es fuente de verdad del status.
4. Lease: solo un holder activo; claim atómico.

### Relación legacy
- No usa `generation_jobs` de library.db.
- Copia el patrón: `idempotency_key`, `locked_by`, `lease_expires_at`, `cancel_requested`, `attempt`, `stage`.

### TypeScript
`JobSnapshotV1` con `status: JobStatusV1` union literal; `input` como discriminated union por `kind`.

---

## 4. ArtifactManifestV1

### Propósito
Registro inmutable de un archivo multimedia o sidecar producido por un job, con validación y linaje.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `id` | string UUID | sí | |
| `contentProjectId` | string UUID | sí | |
| `kind` | enum | sí | `source_proxy` \| `transcript_json` \| `transcript_srt` \| `candidates_snapshot` \| `vertical_mp4` \| `render_manifest_json` \| `other` |
| `role` | enum | sí | `intermediate` \| `final_deliverable` \| `sidecar` |
| `path` | string abs. | sí | Bytes en filesystem. |
| `sha256` | string | sí | Hash del contenido final. |
| `byteSize` | number | sí | Entero ≥ 0. |
| `mimeType` | string | sí | |
| `createdByJobId` | string UUID | sí | Job creador. |
| `parentArtifactIds` | string[] | sí | Linaje (puede ser vacío solo para ingest de fuente externa registrada). |
| `probe` | MediaProbeV1 \| null | no | Obligatorio para `vertical_mp4` final. |
| `validationStatus` | enum | sí | `pending` \| `passed` \| `failed` |
| `validationNotes` | string \| null | no | |
| `createdAt` | string RFC3339 | sí | |
| `immutable` | boolean | sí | Siempre `true` tras `passed` final. |

#### MediaProbeV1

| Campo | Tipo | Oblig. | Unidad |
|---|---|---|---|
| `durationS` | number | sí | segundos |
| `width` | number \| null | no | px |
| `height` | number \| null | no | px |
| `videoCodec` | string \| null | no | |
| `audioCodec` | string \| null | no | |
| `hasAudio` | boolean | sí | |
| `hasVideo` | boolean | sí | |
| `fps` | number \| null | no | |

### Invariantes
1. Tras `validationStatus = passed` y `role = final_deliverable`: **no** se modifica path ni bytes ni fila (excepto archivado lógico futuro fuera de MVP).
2. Job `completed` solo referencia artifacts `passed`.
3. `path` del final **nunca** es el path temporal del job.
4. Input cliente y output final: paths distintos (`safe_paths` invariant).
5. Retry no crea segundo final con mismo `idempotency` sin política: el artifact final se reutiliza o se crea nuevo id con nuevo path único.

### Relación legacy
Sustituye `ArtifactRef` pobre (`kind`+`path` sin hash). No usa `media_assets` de biblioteca (dominio distinto: assets reutilizables vs entregables de cliente).

---

## 5. ShortCandidateV1

### Propósito
Propuesta de Short persistida: span, score, framing y estado de revisión **de workflow**, sin ser la historia completa de decisiones humanas (eso es ReviewDecision).

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica / unidad |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `id` | string UUID | sí | |
| `contentProjectId` | string UUID | sí | |
| `sourceJobId` | string UUID | sí | Job que generó el set. |
| `sourceMediaPath` | string abs. | sí | |
| `startS` | number | sí | segundos en timeline **fuente** |
| `endS` | number | sí | segundos fuente; `endS > startS` |
| `durationS` | number | sí | `endS - startS` |
| `originalStartS` | number | sí | propuesta inicial |
| `originalEndS` | number | sí | |
| `transcriptText` | string | sí | texto del tramo (no log completo del video) |
| `title` | string | sí | |
| `summary` | string | sí | |
| `score` | number | sí | 0..100 (como `ClipScoreBreakdown::total`) |
| `confidence` | number | sí | 0..1 |
| `scoreBreakdown` | object | sí | mismos campos que `ClipScoreBreakdown` |
| `reasons` | {code,label,weight}[] | sí | |
| `warnings` | string[] | sí | |
| `strengths` | string[] | sí | |
| `risks` | string[] | sí | |
| `workflowStatus` | enum | sí | ver abajo |
| `variantGroupId` | string | sí | |
| `isPrimaryVariant` | boolean | sí | |
| `framing` | FramingV1 | sí | |
| `createdAt` | string RFC3339 | sí | |
| `updatedAt` | string RFC3339 | sí | |

#### FramingV1

| Campo | Tipo | Oblig. | Notas |
|---|---|---|---|
| `mode` | enum | sí | `auto_center` \| `manual` \| `blurred_background` \| `fit_with_bars` |
| `centerX` | number | sí | 0..1 normalizado fuente |
| `centerY` | number | sí | 0..1 |
| `zoom` | number | sí | ≥ 1 |
| `outputWidth` | number | sí | 1080 MVP |
| `outputHeight` | number | sí | 1920 MVP |

#### workflowStatus
`suggested` \| `preselected` \| `approved` \| `rejected` \| `modified` \| `exporting` \| `exported` \| `error` \| `discarded`  
(mirrors `ClipReviewStatus`; es **estado de UI/workflow actual**, no el log de decisiones).

### Invariantes
1. `endS > startS` y `durationS` coherente.
2. Cambios de span/framing por humano generan `ReviewDecision` append-only; el candidate se actualiza al último estado **y** la decisión queda en el log.
3. No es Artifact final.

### Relación legacy
Mapeo directo desde `ClipCandidate` + `ClipFraming`.

---

## 6. ReviewDecisionV1

### Propósito
Registro **append-only** de una decisión humana. Nunca se sobrescribe.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `id` | string UUID | sí | |
| `contentProjectId` | string UUID | sí | |
| `targetKind` | enum | sí | `short_candidate` (MVP) |
| `targetId` | string UUID | sí | id del candidate |
| `decision` | enum | sí | `approve` \| `reject` \| `modify_span` \| `modify_framing` \| `modify_cues` \| `defer` |
| `reason` | string \| null | no | Motivo (esp. reject) |
| `payload` | object | sí | Snapshot de lo decidido (span, framing, cues patch); tipado por `decision` |
| `actor` | string | sí | MVP: `local_operator` |
| `createdAt` | string RFC3339 | sí | **Inmutable** |
| `relatedJobId` | string \| null | no | Job que se desbloquea o originó la cola |

### Invariantes
1. **Solo INSERT.** Prohibido UPDATE/DELETE de filas de decisión en repositorio.
2. Una segunda decisión no borra la anterior; el “estado actual” del candidate se deriva del último decision aplicable + campos denormalizados en `short_candidates`.
3. `payload` debe ser suficiente para reconstruir la intención sin UI.

### Relación legacy
No existe hoy: aprobaciones viven solo en `ClipCandidate.status` en memoria.

---

## 7. VerticalRenderPlanV1

### Propósito
Documento **inmutable** que especifica un render vertical determinista. La UI no construye FFmpeg; el media engine consume este plan.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `id` | string UUID | sí | |
| `contentProjectId` | string UUID | sí | |
| `recipeId` | string UUID | sí | ProductionRecipe usada |
| `candidateId` | string UUID | sí | Short aprobado |
| `sourceMediaPath` | string abs. | sí | |
| `sourceStartS` | number | sí | segundos |
| `sourceEndS` | number | sí | segundos |
| `framing` | FramingV1 | sí | |
| `subtitles` | SubtitleBurnPlanV1 | sí | |
| `outputSpec` | VerticalRenderRecipeV1 | sí | copiado del recipe (congelado) |
| `createdAt` | string RFC3339 | sí | |
| `createdBy` | string | sí | `local_operator` \| `system` |

#### SubtitleBurnPlanV1

| Campo | Tipo | Oblig. | |
|---|---|---|---|
| `enabled` | boolean | sí | |
| `presetId` | string | sí | |
| `cues` | SubtitleCueV1[] | sí | tiempos en **timeline del clip de salida** (0 = inicio del short) |
| `sourceTranscriptArtifactId` | string \| null | no | linaje |

#### SubtitleCueV1

| Campo | Tipo | Oblig. | Unidad |
|---|---|---|---|
| `id` | string | sí | |
| `startS` | number | sí | segundos en output timeline del short |
| `endS` | number | sí | |
| `text` | string | sí | |

### Invariantes
1. **Inmutable** tras INSERT (nueva corrección = nuevo plan id).
2. No incluye command-line FFmpeg.
3. `sourceEndS > sourceStartS`.
4. `outputSpec.width/height` = 1080×1920 en MVP.
5. Input path ≠ output path (el output se resuelve en el job, no en el plan, o se declara como *patrón* de nombre sin ejecutar).

### Relación legacy
Reemplaza la construcción ad-hoc en `export_one_clip` (args FFmpeg armados en Rust sin plan versionado). El plan sigue permitiendo que **Rust** construya el comando a partir del plan (no la UI).

---

## 8. JobProgressV1

### Propósito
Evento/snapshot de progreso para acelerar UI. **No** es fuente de verdad del status del job.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `jobId` | string UUID | sí | **Obligatorio** (corrige progreso global). |
| `contentProjectId` | string UUID | sí | |
| `status` | JobStatusV1 | sí | Eco del último status conocido en emisor. |
| `stage` | string | sí | |
| `message` | string | sí | Mensaje corto UI; sin transcript completo ni secrets. |
| `percent` | number | sí | 0–100 |
| `updatedAt` | string RFC3339 | sí | |

### Invariantes
1. Tras pérdida de eventos, la UI reconstruye desde `JobSnapshotV1`.
2. `percent` no implica `completed`.

### Relación legacy
Reemplaza `JobProgress { job, stage, message, percent }` sin id durable.

---

## 9. ApplicationErrorV1

### Propósito
Error estructurado de la API vNext y de jobs.

### Versión
`contractVersion = "v1"`

### Campos

| Campo | Tipo | Oblig. | Semántica |
|---|---|---|---|
| `contractVersion` | `"v1"` | sí | |
| `code` | string | sí | Snake estable: `job_invalid_transition`, `artifact_validation_failed`, `path_input_equals_output`, `lease_conflict`, `not_found`, `cancelled`, `ffmpeg_failed`, `internal`. |
| `message` | string | sí | Mensaje seguro para UI (español o neutro); **sin** API keys; paths solo si necesarios y recortados. |
| `retryable` | boolean | sí | |
| `jobId` | string \| null | no | |
| `details` | Record&lt;string, string \| number \| boolean&gt; \| null | no | Metadatos no sensibles; **no** `unknown` libre de forma arbitraria — mapa de escalares. |

### Invariantes
1. No serializar secretos.
2. No volcar transcript completo en `message`/`details`.

### Relación legacy
`AppError` Rust se mapeará a este contrato en el boundary Tauri vNext (Fase 5).

---

## 10. Validaciones cruzadas de contratos (Fase 1)

| Requisito | Cumplimiento en este doc |
|---|---|
| Campos con unidad y semántica | `*S` segundos; px; RFC3339; SHA-256 |
| Una fuente de verdad | Job/Artifact/Decision en SQLite; progreso efímero |
| Job completed sin artifact | Prohibido por invariante Job + Artifact |
| RenderPlan inmutable | INSERT only |
| ReviewDecision append-only | INSERT only |
| TS sin unknown | Discriminated unions + mapas de escalares |
| Sin Python/daemon/HTTP/Supabase | No aparecen en contratos MVP |

---

## 11. Mapa TypeScript previsto (no generado en Fase 1)

Archivo futuro sugerido: `src/lib/types/vnext/v1.ts`  
Exportar todas las interfaces/enums de este documento con los mismos nombres `*V1`.  
Prohibido `any` / `unknown` en respuestas de commands vNext.

---

*Fin de 01-CONTRACTS-V1.md — sin código productivo.*
