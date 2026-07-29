# VigilCut vNext — Propuesta de migración SQLite

**Fase:** 1 (solo diseño)  
**Fecha:** 2026-07-29  
**Estado:** propuesta — **ninguna migración aplicada** en esta fase.

---

## 1. Inventario del schema actual relevante

### 1.1 Base visual / biblioteca — `library.db`

**Ubicación lógica:** `pipeline::visual::library::library_root()` + `library.db`  
**Migraciones:** aditivas en runtime vía `pipeline::visual::schema.rs` (`SCHEMA_VERSION = 7`, tabla `schema_meta`).

| Tabla / mecanismo | Rol | Notas para vNext |
|---|---|---|
| `schema_meta(key,value)` | Versionado simple | Patrón a **imitar** (version int), no compartir el mismo contador con vNext sin namespace. |
| `media_assets` | Assets de biblioteca | **No** es Artifact de cliente. Fuera del MVP Short. |
| `asset_usage` | Telemetría de uso | Fuera de MVP Short. |
| `visual_concepts`, `themes`, `asset_concepts` | Biblioteca | Posponer. |
| `visual_needs` | B-roll needs | Posponer. |
| `generation_jobs` | Jobs de generación de imagen | Patrón: `idempotency_key UNIQUE`, status, attempt, lease (`locked_by`, `lease_expires_at`), cancel, stage, origin. **Reutilizar patrón, no la tabla.** |
| `generated_candidates` | Candidatos de imagen | No son Short candidates. |
| `qa_checks`, `provider_capabilities`, `cost_counters`, `sync_queue`, `daily_*`, `library_requests` | Biblioteca / factory visual | Fuera de MVP Short; no DROP. |

**Estilo de migración actual:** `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE … ADD COLUMN` con `let _ = conn.execute` (ignora “duplicate column”). No hay archivos SQL versionados en disco; es código Rust.

### 1.2 Proyecto legacy

| Persistencia | Contenido | Durable |
|---|---|---|
| `{projects_dir}/{id}/project.json` | `Project` (media_path, segments, preset) | Sí (JSON) |
| `AnalysisRun` en disco (commands/analyze) | Events, EDL, exceptions | Sí (JSON) |
| `ClippingRun` en `Mutex<HashMap>` | Candidatos, framing, review | **No** |
| `BatchJob` en `AppState` HashMap | Lote silence | **No** |

### 1.3 No existe hoy

- Tablas `content_projects`, `jobs` (genéricos), `artifacts`, `short_candidates`, `review_decisions`, `render_plans`, `production_recipes`.
- Migraciones numeradas en archivos para un DB de producción de cliente.

---

## 2. Estrategia general vNext

### 2.1 Base de datos dedicada (recomendación)

**Nueva base:** `vnext.db` (nombre de archivo) dentro del data dir de la app, **separada** de `library.db`.

**Justificación:**
- Evita acoplar el schema de Biblioteca Visual (versión 7, muchos ALTER) con el de producción de cliente.
- Reduce riesgo de migraciones cruzadas y de “mezclar videos de clientes con Biblioteca Visual” (regla de privacidad del brief).
- Permite backup/restore del dominio de producción sin tocar assets generados.

**Alternativa rechazada para MVP:** meter tablas vNext dentro de `library.db` con prefijo. Posible después, pero complica ownership y backups.

### 2.2 Versionado

Tabla:

```text
vnext_schema_meta (
  key   TEXT PRIMARY KEY,   -- 'version'
  value TEXT NOT NULL       -- entero decimal como string, p.ej. '1'
)
```

- Constante Rust prevista: `VNEXT_SCHEMA_VERSION: i32 = 1` (primera migración aplica tablas base).
- Migraciones **numeradas monótonas** en código (`migrate_v1`, `migrate_v2`, …) o archivos `migrations/vnext/001_*.sql` embebidos — decisión de implementación en Fase 2; el contrato es: **una versión, un paso aditivo, transaccional**.

### 2.3 Principios no negociables de migración

1. **No `DROP TABLE`.**
2. **No renombrar destructivamente** tablas legacy.
3. Solo `CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`, `ALTER TABLE ADD COLUMN` aditivo.
4. Sin backfill ambiguo de ClippingRun en memoria (no hay fuente).
5. Migración idempotente: aplicar dos veces no corrompe.
6. Backup del archivo `vnext.db` antes de migrar si el archivo ya existe y version &lt; target (Fase 2).

---

## 3. Tablas nuevas propuestas (v1 schema)

Todas las PK son `TEXT` (UUID). Timestamps `TEXT` RFC3339 UTC. Booleanos `INTEGER` 0/1. JSON como `TEXT` validado en aplicación.

### 3.1 `content_projects`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `title` | TEXT | NOT NULL |
| `client_label` | TEXT | NULL |
| `privacy_mode` | TEXT | NOT NULL DEFAULT `'local_only'` |
| `status` | TEXT | NOT NULL |
| `source_media_path` | TEXT | NOT NULL |
| `source_sha256` | TEXT | NULL |
| `source_duration_s` | REAL | NULL |
| `source_width` | INTEGER | NULL |
| `source_height` | INTEGER | NULL |
| `source_has_audio` | INTEGER | NULL |
| `source_has_video` | INTEGER | NULL |
| `source_container` | TEXT | NULL |
| `work_dir` | TEXT | NOT NULL |
| `active_recipe_id` | TEXT | NULL |
| `legacy_project_id` | TEXT | NULL |
| `created_at` | TEXT | NOT NULL |
| `updated_at` | TEXT | NOT NULL |

**Índices:**  
- `idx_content_projects_status` (`status`)  
- `idx_content_projects_updated` (`updated_at`)

**FK:** ninguna física a legacy. `active_recipe_id` se valida en aplicación (o FK a `production_recipes` DEFERRABLE / tras insert).

### 3.2 `production_recipes`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `content_project_id` | TEXT | NOT NULL → FK `content_projects(id)` |
| `label` | TEXT | NOT NULL |
| `recipe_json` | TEXT | NOT NULL (ProductionRecipeV1 body) |
| `supersedes_recipe_id` | TEXT | NULL |
| `created_at` | TEXT | NOT NULL |

**Índices:** `idx_recipes_project` (`content_project_id`)

**Inmutabilidad:** la aplicación **no actualiza** `recipe_json`. Solo INSERT.

### 3.3 `jobs`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `content_project_id` | TEXT | NOT NULL → FK |
| `kind` | TEXT | NOT NULL |
| `status` | TEXT | NOT NULL |
| `idempotency_key` | TEXT | NOT NULL **UNIQUE** |
| `attempt` | INTEGER | NOT NULL DEFAULT 0 |
| `max_attempts` | INTEGER | NOT NULL |
| `priority` | INTEGER | NOT NULL DEFAULT 100 |
| `input_json` | TEXT | NOT NULL |
| `result_artifact_id` | TEXT | NULL |
| `render_plan_id` | TEXT | NULL |
| `error_json` | TEXT | NULL |
| `stage` | TEXT | NOT NULL DEFAULT `''` |
| `progress_pct` | REAL | NOT NULL DEFAULT 0 |
| `locked_by` | TEXT | NULL |
| `lease_expires_at` | TEXT | NULL |
| `cancel_requested` | INTEGER | NOT NULL DEFAULT 0 |
| `created_at` | TEXT | NOT NULL |
| `updated_at` | TEXT | NOT NULL |
| `started_at` | TEXT | NULL |
| `finished_at` | TEXT | NULL |

**Índices:**  
- `idx_jobs_project_status` (`content_project_id`, `status`)  
- `idx_jobs_status_priority` (`status`, `priority`, `created_at`)  
- `idx_jobs_lease` (`status`, `lease_expires_at`)  

**Constraints de aplicación (no solo SQL):**  
- `status = 'completed'` ⇒ `result_artifact_id IS NOT NULL`.

**FK opcionales:** `result_artifact_id` → `artifacts(id)` (añadir cuando orden de insert lo permita; si circular, validar en app en v1).

### 3.4 `artifacts`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `content_project_id` | TEXT | NOT NULL → FK |
| `kind` | TEXT | NOT NULL |
| `role` | TEXT | NOT NULL |
| `path` | TEXT | NOT NULL |
| `sha256` | TEXT | NOT NULL |
| `byte_size` | INTEGER | NOT NULL |
| `mime_type` | TEXT | NOT NULL |
| `created_by_job_id` | TEXT | NOT NULL |
| `probe_json` | TEXT | NULL |
| `validation_status` | TEXT | NOT NULL |
| `validation_notes` | TEXT | NULL |
| `created_at` | TEXT | NOT NULL |

**Índices:**  
- `idx_artifacts_project` (`content_project_id`)  
- `idx_artifacts_job` (`created_by_job_id`)  
- `idx_artifacts_sha` (`sha256`)  

**Inmutabilidad:** no UPDATE de `path`/`sha256` tras `validation_status = 'passed'`.

### 3.5 `artifact_parents`

| Columna | Tipo | Constraints |
|---|---|---|
| `artifact_id` | TEXT | NOT NULL → FK `artifacts(id)` |
| `parent_artifact_id` | TEXT | NOT NULL → FK `artifacts(id)` |
| PRIMARY KEY (`artifact_id`, `parent_artifact_id`) | | |

**Índice:** `idx_artifact_parents_parent` (`parent_artifact_id`)

### 3.6 `short_candidates`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `content_project_id` | TEXT | NOT NULL → FK |
| `source_job_id` | TEXT | NOT NULL |
| `source_media_path` | TEXT | NOT NULL |
| `start_s` | REAL | NOT NULL |
| `end_s` | REAL | NOT NULL |
| `duration_s` | REAL | NOT NULL |
| `original_start_s` | REAL | NOT NULL |
| `original_end_s` | REAL | NOT NULL |
| `transcript_text` | TEXT | NOT NULL |
| `title` | TEXT | NOT NULL |
| `summary` | TEXT | NOT NULL |
| `score` | REAL | NOT NULL |
| `confidence` | REAL | NOT NULL |
| `score_breakdown_json` | TEXT | NOT NULL |
| `reasons_json` | TEXT | NOT NULL |
| `warnings_json` | TEXT | NOT NULL |
| `strengths_json` | TEXT | NOT NULL |
| `risks_json` | TEXT | NOT NULL |
| `workflow_status` | TEXT | NOT NULL |
| `variant_group_id` | TEXT | NOT NULL |
| `is_primary_variant` | INTEGER | NOT NULL |
| `framing_json` | TEXT | NOT NULL |
| `created_at` | TEXT | NOT NULL |
| `updated_at` | TEXT | NOT NULL |

**Índices:**  
- `idx_candidates_project_score` (`content_project_id`, `score` DESC)  
- `idx_candidates_project_status` (`content_project_id`, `workflow_status`)  
- `idx_candidates_variant` (`variant_group_id`)

**CHECK sugerido (SQLite):** `end_s > start_s`

### 3.7 `review_decisions`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `content_project_id` | TEXT | NOT NULL → FK |
| `target_kind` | TEXT | NOT NULL |
| `target_id` | TEXT | NOT NULL |
| `decision` | TEXT | NOT NULL |
| `reason` | TEXT | NULL |
| `payload_json` | TEXT | NOT NULL |
| `actor` | TEXT | NOT NULL |
| `related_job_id` | TEXT | NULL |
| `created_at` | TEXT | NOT NULL |

**Índices:**  
- `idx_decisions_target` (`target_kind`, `target_id`, `created_at`)  
- `idx_decisions_project` (`content_project_id`, `created_at`)

**Append-only:** repositorio sin método update/delete. Sin ON DELETE CASCADE desde candidates que borre historia (si se “descarta” candidate, las decisiones permanecen).

### 3.8 `render_plans`

| Columna | Tipo | Constraints |
|---|---|---|
| `id` | TEXT | PK |
| `content_project_id` | TEXT | NOT NULL → FK |
| `recipe_id` | TEXT | NOT NULL |
| `candidate_id` | TEXT | NOT NULL |
| `plan_json` | TEXT | NOT NULL (VerticalRenderPlanV1 completo) |
| `created_at` | TEXT | NOT NULL |
| `created_by` | TEXT | NOT NULL |

**Índices:** `idx_render_plans_project` (`content_project_id`)

**Inmutabilidad:** solo INSERT.

### 3.9 Tablas auxiliares estrictamente necesarias

#### `job_events` (opcional v1.1 — **no requerida en primera migración**)

Si se necesita auditoría de progreso durable más allá del último `progress_pct` en `jobs`, se puede añadir en migración v2:

- `id`, `job_id`, `at`, `stage`, `percent`, `message`

**MVP:** basta con columnas en `jobs` + eventos Tauri efímeros. **No** crear `job_events` en la primera migración salvo que Fase 2 demuestre necesidad de tests de reconstrucción.

**No proponer:** tablas Idea, Scene, VisualRequirement, scripts, CRM, pagos.

---

## 4. Foreign keys y orden de creación

Orden `migrate_v1`:

1. `vnext_schema_meta`
2. `content_projects`
3. `production_recipes` (FK project)
4. `render_plans` (FK project; candidate_id sin FK rígida opcional en v1 si candidates se crean después — **recomendación:** crear `short_candidates` antes y FK `candidate_id`)
5. `short_candidates`
6. `jobs`
7. `artifacts`
8. `artifact_parents`
9. `review_decisions`
10. Ajustar FKs `jobs.result_artifact_id` si se desea (PRAGMA foreign_keys)

**PRAGMA:** `foreign_keys = ON` en cada conexión vNext.

---

## 5. Convivencia con legacy

| Sistema | Acción |
|---|---|
| `library.db` | Intocado por migraciones vNext. |
| `project.json` | Sigue funcionando para silence/clips legacy. |
| `ClippingCache` memoria | Sigue hasta Fase 3; dual-read opcional después. |
| Comandos Tauri legacy | Sin cambios en Fase 2 inicial; se añaden commands vNext en Fase 5. |

**No migrar todavía:**
- ClippingRuns en memoria (imposible de forma fiable).
- Batch jobs.
- generation_jobs / media_assets.
- AnalysisRun JSON → content project (puede ser import opcional posterior).

---

## 6. Backup y recuperación

### Antes de migrar `vnext.db` existente
1. Copiar `vnext.db` → `vnext.db.bak-{timestamp}`.
2. Ejecutar migración en transacción.
3. Si falla: restaurar backup; reportar `ApplicationErrorV1` `schema_migration_failed`.

### Base vacía
- Crear archivo + migrate_v1 → version 1.

### Base actual (app instalada sin vnext.db)
- Idéntico a vacía; no toca `library.db`.

### Rollback
- SQLite no tiene down-migrations en este diseño.
- Recuperación = restaurar backup de archivo.
- Nunca down-DROP.

### Migración repetida
- Si `version >= target`, no-op.
- Tests: migrate ×2 sin error ni duplicar índices/tablas.

---

## 7. Pruebas de migración (a implementar en Fase 2, especificadas aquí)

| Caso | Criterio |
|---|---|
| Empty DB | Tras migrate, 8 tablas (+ meta) existen; version=1 |
| Idempotent | migrate dos veces; version estable; count tablas estable |
| From “current app” | Abrir dir con solo library.db + projects; crear vnext.db sin mutar library schema_meta |
| FK | Insert job con project inexistente falla si FK on |
| Completed sin artifact | Rechazado en capa dominio/repo (test unitario), no solo SQL |
| Windows paths | `work_dir` y paths con `\` redondeados en tests |

---

## 8. Riesgos

| Riesgo | Mitigación |
|---|---|
| Dos DBs confunden operadores | Documentar paths en UI solo como “datos locales”; logs con nombres de archivo no de ruta completa de cliente si es sensible |
| FK cycles job↔artifact | Validar completed en dominio; FK laxa en v1 si hace falta |
| JSON schema drift | `contractVersion` + tests de serde round-trip en Fase 2 |
| Alguien escribe en library.db por error | API de persistencia vNext solo abre `vnext.db` |
| Crecimiento de `transcript_text` en candidates | Aceptable en MVP; no loguear; índice no sobre texto completo |
| ALTER futuro de jobs | Solo ADD COLUMN; version bump |

---

## 9. Qué no se hace en esta propuesta

- No `DROP` / `RENAME` de tablas visuales.
- No unificar library jobs con content jobs.
- No migrar datos de clipping (no hay store durable).
- No Supabase.
- No aplicar SQL en esta Fase 1.

---

## 10. Resumen de numeración prevista

| Versión | Contenido |
|---|---|
| **v1** | Tablas de la sección 3 (+ meta) |
| v2+ | Solo con ADR y tests (p.ej. job_events, columnas nuevas aditivas) |

---

*Fin de 02-SQLITE-MIGRATION-PROPOSAL.md — sin migraciones aplicadas.*
