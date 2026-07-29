# VigilCut vNext — Secuencia de implementación

**Fase:** 1 (solo plan)  
**Fecha:** 2026-07-29  
**Alcance del documento:** descomponer Fases 2–6 en tareas verificables.  
**No contiene código.**  
**No autoriza por sí mismo ejecutar Fases 2–6.**

Referencias:
- Contratos: `docs/vnext/01-CONTRACTS-V1.md`
- SQLite: `docs/vnext/02-SQLITE-MIGRATION-PROPOSAL.md`
- Producto: `docs/VIGILCUT_DESIGN_REVIEW.md`

---

## Convenciones de cada tarea

Para cada tarea se define: objetivo, archivos previstos, cambios, dependencias, pruebas, aceptación, riesgos, **punto de detención**.

Leyenda de dependencias: `T-x.y` debe completarse antes.

---

# FASE 2 — Backend durable mínimo

**No ejecutar hasta aprobación explícita.**  
Sin frontend, sin export vertical nuevo.

## T-2.1 — Módulo `vnext` y frontera de crates internos

| | |
|---|---|
| **Objetivo** | Crear esqueleto `src-tauri/src/vnext/{domain,application,persistence,jobs}` sin lógica pesada. |
| **Archivos** | `src-tauri/src/vnext/mod.rs`, submódulos, registro en `lib.rs` |
| **Cambios** | Módulos vacíos o con `pub use`; sin commands Tauri nuevos. |
| **Deps** | Fase 1 docs |
| **Pruebas** | `cargo check` / `cargo test --lib` sin regresiones |
| **Aceptación** | Compila; no cambia comportamiento UI |
| **Riesgos** | Contagio de deps Tauri en domain — **prohibido** |
| **Detener** | Tras check verde; no migraciones aún si se prefiere split, o continuar T-2.2 |

## T-2.2 — Dominio puro: JobStatus y transiciones

| | |
|---|---|
| **Objetivo** | Enum `JobStatus` + `fn transition(from, to) -> Result` con tabla del contrato. |
| **Archivos** | `vnext/domain/job.rs`, `vnext/domain/status.rs`, tests en mismo módulo |
| **Cambios** | Solo dominio; sin IO |
| **Deps** | T-2.1 |
| **Pruebas** | Todas las transiciones válidas; muestreo exhaustivo de inválidas; `completed` sin artifact id rechazado a nivel de *comando de dominio* `complete(job, artifact_id)` |
| **Aceptación** | Tests de dominio pasan sin SQLite |
| **Riesgos** | Duplicar estados visuales — nombres alineados al contrato v1 |
| **Detener** | Suite dominio verde |

## T-2.3 — Dominio: Artifact / Recipe / Project IDs types

| | |
|---|---|
| **Objetivo** | Newtypes o structs mínimos + invariantes (paths distintos, plan immutable flag). |
| **Archivos** | `vnext/domain/{project,recipe,artifact,error}.rs` |
| **Deps** | T-2.1 |
| **Pruebas** | Construcción inválida falla; ApplicationError mapping unit |
| **Aceptación** | Sin dependencia rusqlite/tauri en `domain` (enforced por grafo de módulos) |
| **Detener** | Check de deps (revisión manual o test de arquitectura simple) |

## T-2.4 — Migración v1 `vnext.db`

| | |
|---|---|
| **Objetivo** | Implementar `02-SQLITE-MIGRATION-PROPOSAL` v1. |
| **Archivos** | `vnext/persistence/schema.rs` o `migrations`, open_db path helper |
| **Cambios** | Crear DB separada; **no** tocar `library.db` schema_meta |
| **Deps** | T-2.1, doc 02 |
| **Pruebas** | empty migrate; migrate×2; open existing library.db dir sin mutar version visual |
| **Aceptación** | Tablas listadas existen; version=1 |
| **Riesgos** | Path data dir Windows — usar dirs de app existentes |
| **Detener** | Tests de migración verdes |

## T-2.5 — Repositorios SQLite: Project + Recipe

| | |
|---|---|
| **Objetivo** | CRUD mínimo content_projects + insert-only recipes. |
| **Archivos** | `vnext/persistence/projects.rs`, `recipes.rs` |
| **Deps** | T-2.4 |
| **Pruebas** | insert/get/list; recipe no se updatea (método ausente o error) |
| **Aceptación** | Round-trip snapshot campos del contrato |
| **Detener** | — |

## T-2.6 — Repositorio Jobs: enqueue, get, claim, lease

| | |
|---|---|
| **Objetivo** | Persistir job **antes** de ejecutar; claim atómico; lease. |
| **Archivos** | `vnext/persistence/jobs.rs`, `vnext/jobs/runner.rs` (esqueleto) |
| **Deps** | T-2.2, T-2.4 |
| **Pruebas** | claim concurrente (dos threads, un winner); lease expirado → interrupted/requeue policy documentada; idempotency key unique |
| **Aceptación** | No hay job `running` sin fila previa `queued` |
| **Riesgos** | Carreras en Windows — usar transacciones IMMEDIATE |
| **Detener** | Tests claim/lease verdes |

## T-2.7 — Artifacts + parents + complete job

| | |
|---|---|
| **Objetivo** | Insert artifact validado; `complete` solo con validation passed. |
| **Archivos** | `persistence/artifacts.rs`, application service `complete_job` |
| **Deps** | T-2.6 |
| **Pruebas** | complete sin artifact falla; complete con validation failed falla; parents link |
| **Aceptación** | Invariante completed⇔artifact enforced |
| **Detener** | — |

## T-2.8 — Retry / cancel / interrupt recovery on startup

| | |
|---|---|
| **Objetivo** | failed/interrupted → queued explícito; cancel path; al abrir app marcar leases vencidos. |
| **Archivos** | application services; hook en setup Tauri (solo recovery, sin runner de clipping) |
| **Deps** | T-2.6, T-2.7 |
| **Pruebas** | retry; cancel queued; cancel running→cancelling→cancelled; recovery marca interrupted |
| **Aceptación** | Criterios Fase 2 del brief |
| **Detener** | **Fin de Fase 2** — reporte, sin UI, sin push salvo orden |

### Criterio de salida Fase 2 (gate)

- [ ] Migraciones empty + idempotent
- [ ] Transiciones dominio
- [ ] Claim concurrente
- [ ] Lease / interrupt
- [ ] Retry / cancel
- [ ] Complete requiere artifact validado
- [ ] Idempotencia no duplica fila job
- [ ] `cargo fmt`, `clippy -D warnings`, tests del módulo
- [ ] `git diff --check`
- [ ] **STOP** — solicitar aprobación Fase 3

---

# FASE 3 — Clipping persistente y revisión

**No ejecutar hasta aprobación.**

## T-3.1 — Persist short_candidates desde pipeline clipping

| | |
|---|---|
| **Objetivo** | Tras generar candidatos (reutilizando `pipeline/clipping`), escribir `short_candidates`. |
| **Archivos** | application `generate_candidates`; adaptador desde `ClipCandidate` |
| **Deps** | Fase 2 completa |
| **Pruebas** | N candidates en DB; reinicio de proceso simulado (nueva conexión) los relee |
| **Aceptación** | Equivalencia de score/span/framing con run en memoria |
| **Riesgos** | No reescribir scoring — solo adaptar |
| **Detener** | — |

## T-3.2 — Desacoplar ClippingCache como caché, no verdad

| | |
|---|---|
| **Objetivo** | `commands/clipping.rs` lee/escribe DB vía application; memoria opcional. |
| **Archivos** | `commands/clipping.rs` (adaptación mínima), application |
| **Deps** | T-3.1 |
| **Pruebas** | Tests clipping existentes verdes; nuevo test persistencia |
| **Aceptación** | Aprobar candidate, drop cache, get → sigue approved |
| **Riesgos** | Regresión UI clips — mantener shape JSON de respuesta |
| **Detener** | — |

## T-3.3 — ReviewDecision append-only + update workflow status

| | |
|---|---|
| **Objetivo** | `save_review_decision` inserta decisión y actualiza candidate. |
| **Archivos** | persistence decisions; application |
| **Deps** | T-3.1 |
| **Pruebas** | dos decisiones no borran la primera; reject queda registrado; modify span sobrevive |
| **Aceptación** | Append-only enforced (no update API) |
| **Detener** | — |

## T-3.4 — nextAction del ContentProject

| | |
|---|---|
| **Objetivo** | Derivar próxima acción tras jobs/candidates/decisions. |
| **Archivos** | domain/application project service |
| **Deps** | T-3.3, jobs |
| **Pruebas** | matriz de estados → nextAction |
| **Aceptación** | Reconstruir tras reinicio |
| **Detener** | **Fin Fase 3** gate |

### Gate Fase 3

- [ ] Candidatos sobreviven reinicio
- [ ] Aprobación / span / framing sobreviven
- [ ] Decisión rechazada registrada
- [ ] Segunda decisión no borra la anterior
- [ ] Suite clipping legacy relevante verde
- [ ] **STOP**

---

# FASE 4 — Render vertical y subtítulos

**No ejecutar hasta aprobación.**

## T-4.1 — VerticalRenderPlan persist + factory desde candidate

| | |
|---|---|
| **Objetivo** | Crear plan inmutable desde candidate + recipe + cues. |
| **Archivos** | domain plan; persistence render_plans |
| **Deps** | Fase 3 |
| **Pruebas** | plan insert; update intent falla |
| **Detener** | — |

## T-4.2 — Adaptador export vertical: temp → validate → atomic

| | |
|---|---|
| **Objetivo** | Reutilizar FFmpeg framing; aplicar `safe_paths` / patrón `export.rs`; output 1080×1920. |
| **Archivos** | `pipeline/clipping/export_clips.rs` adaptado **o** nuevo `vnext/media/vertical_render.rs` que llame helpers; **no** UI FFmpeg |
| **Deps** | T-4.1, safe_paths, sidecar |
| **Pruebas** | input≠output; temp no es final; corrupt fails job; duración tolerancia |
| **Riesgos** | No borrar export legacy hasta equivalencia |
| **Detener** | — |

## T-4.3 — Subtítulos unificados + burn-in preset

| | |
|---|---|
| **Objetivo** | Un solo camino Whisper/SRT → cues → burn en plan. Eliminar dependencia del stub para el happy path vNext. |
| **Archivos** | adaptar `whisper_cli.rs`; no usar stub de `subtitles.rs` en vNext |
| **Deps** | T-4.1 |
| **Pruebas** | cues en zona segura (heurística); sin red |
| **Aceptación** | Preset cerrado; **STOP antes** de más presets/proveedores externos |
| **Detener** | Aprobación humana de preset |

## T-4.4 — Job vertical_render end-to-end backend

| | |
|---|---|
| **Objetivo** | Job durable: plan → temp → ffprobe → artifact final + manifest linaje. |
| **Deps** | T-4.2, T-4.3, Fase 2 jobs |
| **Pruebas** | retry no duplica final; cancel limpia solo work dir del job; interrupt mid-render |
| **Gate Fase 4** | Criterios brief § render; **STOP** |

---

# FASE 5 — API Tauri y frontend vNext

**No ejecutar hasta aprobación.**

## T-5.1 — Commands Tauri casos de uso tipados

| | |
|---|---|
| **Objetivo** | Lista del brief (create/list/get project, jobs, review, render…). |
| **Archivos** | `commands/vnext_*.rs` o `vnext/commands`; `tauri.ts` wrappers **tipados** |
| **Deps** | Fases 2–4 |
| **Pruebas** | serde round-trip; errores ApplicationErrorV1 |
| **Aceptación** | Cero `unknown` en wrappers vNext |
| **Detener** | — |

## T-5.2 — Stores frontend separados

| | |
|---|---|
| **Objetivo** | `uiStore`, `projectStore`, `jobStore`, `reviewStore` |
| **Archivos** | `src/lib/stores/vnext/*` |
| **Deps** | T-5.1 |
| **Pruebas** | Si aún no hay runner: tests mínimos o harness acordado; preferir añadir vitest **solo si se aprueba** dependencia — si no, tests de contratos TS via `tsc` / svelte-check |
| **Aceptación** | App.svelte no llama FFmpeg |
| **Detener** | — |

## T-5.3 — Navegación Proyectos | Cola | Biblioteca + legacy flag

| | |
|---|---|
| **Objetivo** | Shell vNext; legacy bajo entrada secundaria. |
| **Archivos** | `App.svelte`, componentes vNext |
| **Deps** | T-5.2 |
| **Pruebas** | check; smoke manual documentado |
| **Detener** | — |

## T-5.4 — UX revisión Short

| | |
|---|---|
| **Objetivo** | Play, approve/reject, span, framing, cues, render, progress, retry. |
| **Archivos** | reutilizar ShortPlayer, VerticalClipPreview |
| **Deps** | T-5.3, Fase 4 |
| **Pruebas** | matriz estados job en UI (documentada + check) |
| **Gate Fase 5** | **STOP** |

---

# FASE 6 — E2E y aceptación

**No ejecutar hasta aprobación.**

## T-6.1 — Fixture E2E autorizado

| | |
|---|---|
| **Objetivo** | Video sintético corto en `src-tauri/tests/fixtures` (no cliente real). |
| **Deps** | Fases 2–5 |
| **Pruebas** | flujo completo brief |
| **Detener** | — |

## T-6.2 — Matriz de interrupción

| | |
|---|---|
| **Objetivo** | Interrumpir en cada punto del brief; verificar interrupted + retry + no final parcial. |
| **Deps** | T-6.1 |
| **Gate final** | Legacy suite relevante verde; **STOP** release decision |

---

## Orden resumido y puntos de aprobación humana

```text
[APROBAR] Fase 1 docs
    → Fase 2 (T-2.1 … T-2.8) → [APROBAR]
    → Fase 3 (T-3.1 … T-3.4) → [APROBAR]
    → Fase 4 (T-4.1 … T-4.4) → [APROBAR preset]
    → Fase 5 (T-5.1 … T-5.4) → [APROBAR]
    → Fase 6 (T-6.1 … T-6.2) → [APROBAR release]
```

Ninguna fase posterior se inicia sin aprobación explícita del propietario.

---

## Tareas explícitamente fuera de secuencia MVP

- Image Factory / Daily Feed / Supabase
- Lotes de Shorts (post-MVP design review Fase 2 producto)
- Faceless horizontal / Idea / Scene
- Daemon / HTTP / Python worker
- Timeline multipista / keyframes
- Drop de tablas o comandos legacy

---

*Fin de 03-IMPLEMENTATION-SEQUENCE.md*
