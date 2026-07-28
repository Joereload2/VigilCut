# CYCLE-003 — Biblioteca: separar carga de consulta + OmniRoute primero

- Rol: Product Manager / Arquitectura
- Estado: PENDIENTE
- Base HEAD: ab1e51932d5cd50c6d637b1d01086ab9d898e2af (puede correr en
  paralelo con CYCLE-002 — verificado: CYCLE-002 solo toca
  `VisualPanel.svelte`/`VisualWorkspace.svelte`, este ciclo solo toca Rust
  en `visual_library/` y `pipeline/visual/generation/`; no hay archivos en
  común. La única dependencia es lógica: el Paso 5 de este ciclo valida
  que `brollOnly` siga funcionando, así que conviene revisar el resultado
  de CYCLE-002 antes de dar por cerrado el Paso 5, aunque el código en sí
  no se pise.)
- Fecha: 2026-07-27
- Prioridad: alta
- cycle_id: CYCLE-003

## Problema

Dos pedidos de producto relacionados:

1. La Biblioteca debe tener **dos procesos claramente separados**: uno de
   carga de imágenes (ingestion) y otro de consulta (query). Hoy están
   mezclados.
2. La carga de imágenes debe **intentar primero OmniRoute** (da tokens
   gratis) y solo si no está disponible o falla, usar **la alternativa
   normal** (Pollinations). Hoy no hay ninguna cadena de fallback: se elige
   un único proveedor fijo por variable de entorno.

## Causa raíz (con evidencia)

### B1 — Carga y consulta mezcladas, y ambas dependen de código legacy

`src-tauri/src/visual_library/application/library_service.rs`:

- El trait público `VisualLibrary` mezcla lectura y escritura sin
  distinción: `search`, `get_asset` (lectura) junto con
  `request_generation`, `record_usage` (escritura/telemetría).
- `ingest_asset` — el método de carga real — **ni siquiera está en el
  trait**. Es un método suelto en `LocalVisualLibrary` que ejecuta SQL
  directo sobre `media_assets` y llama a
  `pipeline::visual::library::import_image_detailed`.
- Este mismo archivo importa directo de `pipeline::visual::intelligent_match`,
  `pipeline::visual::generation::worker` y `pipeline::visual::needs`.
- `src-tauri/src/visual_library/application/sync_service.rs` (el push a
  Supabase) también importa directo de `pipeline::visual::library`.

Resultado: no hay ningún punto único por el que pase toda escritura, ni
ningún punto único por el que pase toda lectura. Cualquier módulo puede
llegar a `pipeline::visual::library` por su cuenta.

### B2 — No existe cadena de fallback de proveedores

`src-tauri/src/pipeline/visual/generation/provider.rs`, función
`select_provider()`:

- Devuelve **un solo proveedor**, elegido una vez, por variable de entorno
  (`VIGILCUT_IMAGE_PROVIDER`) o por si `OMNIROUTE_BASE_URL` está seteada.
- Si OmniRoute no está configurado, cae a **Mock**, no a Pollinations.
- No hay ningún lugar en `worker.rs` que, ante un error de OmniRoute,
  reintente con otro proveedor. El manejo de error actual solo marca el
  job como fallido.

`PollinationsImageProvider` ya está correctamente marcado
`free_configured: false` (comentario en el propio código: "Pollinations
consumes metered Pollen. Promotional credits are not a verified zero-cost
entitlement") — es la alternativa de pago/medida correcta para usar como
segundo escalón, nunca como primero.

## Alcance de esta fase

Dentro:
- Reorganizar `visual_library/application/` en dos servicios explícitos:
  `IngestionService` (toda escritura: import manual, import de carpeta,
  aprobación de generación, sync a Supabase) y `QueryService` (toda
  lectura: search, get_asset, record_usage de consulta).
- Un único punto de acceso a `pipeline::visual::library` (un adaptador de
  infraestructura), usado solo por `IngestionService` y `QueryService` —
  ningún otro módulo puede importar `pipeline::visual::library` directo.
- Cadena de selección de proveedor: OmniRoute primero (probe + generate);
  si el probe falla, si `OMNIROUTE_BASE_URL` no está configurado, o si
  `generate()` devuelve error, pasar automáticamente a Pollinations. Mock
  solo si se fuerza explícitamente (`VIGILCUT_IMAGE_PROVIDER=mock`, uso en
  tests).
- Mantener intactos los cost gates existentes (`paid_providers_enabled`,
  `CostKind`) — Pollinations sigue reportándose como no-gratis, nunca
  usarlo silenciosamente si el usuario no habilitó proveedores pagos, en
  cuyo caso: si OmniRoute falla y Pollinations requiere pago no habilitado,
  el job debe fallar con un mensaje claro, no generar igual.

Fuera de alcance:
- Extraer la Biblioteca a un servicio/proceso realmente separado (deploy
  aparte). La base de código no está lista para eso todavía; este ciclo es
  el prerrequisito, no el destino final.
- Cambios de esquema SQLite o de las migraciones de Supabase.
- Tocar `VisualPicker.svelte` ni nada de la UI de consulta de B-roll
  (ya resuelto en CYCLE-002).

## Paso a paso para Grok

### Paso 1 — Definir el adaptador único de infraestructura legacy

Alcance verificado (búsqueda completa, no parcial): hoy **6 archivos**
dentro de `src-tauri/src/visual_library/` importan `pipeline::visual::library`
directo, no solo 2:

- `application/library_service.rs`
- `application/sync_service.rs`
- `domain/usage.rs` (re-exporta `AssetUsageRow` directo del legacy — no es
  un tipo de dominio propio, hay que definir un tipo propio o justificar
  explícitamente por qué se mantiene el re-export)
- `infrastructure/sqlite/mod.rs` (re-exporta `open_db` directo)
- `infrastructure/storage/mod.rs` (re-exporta `library_root` directo)
- `infrastructure/providers/pollinations.rs` (solo en tests: usa
  `lock_library_for_test` / `set_library_root_override` — esto puede
  quedar así, es harness de test compartido, no lógica de producto)

Crear `src-tauri/src/visual_library/infrastructure/legacy_adapter.rs` que
envuelva las funciones de `pipeline::visual::library` usadas por los
primeros 5 archivos de la lista (`import_image_detailed`, `open_db`,
`get_asset_by_id`, `record_usage`, `library_root`, etc.). Este adaptador
es el único archivo de producto (no de test) autorizado a importar
`pipeline::visual::library` directamente.

**Importante — no confundir alcance:** esto NO implica tocar el resto de
`pipeline::visual` (`needs.rs`, `render.rs`, `concepts.rs`, `qa.rs`,
`worker.rs`, `library_dashboard.rs`, `library_requests.rs`,
`intelligent_match.rs`, `commands/visual.rs`, `commands/visual_intel.rs`,
`bin/vigilcut_cli.rs`). Esos módulos comparten la misma base SQLite por
diseño (`independent-domain.md`: B-roll posee needs/placements y usa la
misma base física) y **siguen llamando a `pipeline::visual::library`
directo, sin pasar por el adaptador** — eso es correcto, no es parte de
este ciclo, y no hay que "arreglarlo".

Criterio de aceptación: `grep -r "pipeline::visual::library" src-tauri/src/visual_library`
solo devuelve resultados dentro de `legacy_adapter.rs` y, como excepción
documentada, dentro de los tests de `infrastructure/providers/pollinations.rs`.

### Paso 2 — Separar `IngestionService` de `QueryService`

En `library_service.rs`, dividir el trait actual `VisualLibrary` en dos:

```rust
pub trait LibraryIngestion {
    fn ingest_asset(&self, request: AssetIngestionRequest) -> AppResult<AssetIngestionResult>;
    fn request_generation(&self, request: LibraryGenerationRequest) -> AppResult<Option<String>>;
}

pub trait LibraryQuery {
    fn search(&self, query: &AssetQuery) -> AppResult<Vec<AssetMatch>>;
    fn get_asset(&self, asset_id: &str) -> AppResult<MediaAsset>;
    fn record_usage(&self, usage: AssetUsage) -> AppResult<()>;
}
```

(Nombres orientativos — Grok puede ajustarlos, pero la separación en dos
traits es el requisito, no opcional.) Ambos implementados por
`LocalVisualLibrary`, pero cada comando Tauri (`commands/library_commands.rs`,
`commands/sync_commands.rs`) debe declarar explícitamente cuál de los dos
usa. Los comandos de import/generación usan `LibraryIngestion`; los
comandos de búsqueda/detalle usan `LibraryQuery`.

Criterio de aceptación: ningún comando Tauri de solo-lectura (buscar,
obtener asset) puede llamar a `ingest_asset` ni a `request_generation` por
tipo — el compilador debe impedirlo porque esos comandos solo reciben
`&dyn LibraryQuery`.

### Paso 3 — Cadena de fallback OmniRoute → Pollinations

En `provider.rs`, reemplazar `select_provider()` por una función que
devuelva una **lista ordenada** de candidatos en vez de uno solo, y mover
la lógica de intento/fallback a `worker.rs`:

1. Si `VIGILCUT_IMAGE_PROVIDER` fuerza explícitamente `mock` o
   `pollinations`, respetar esa orden (comportamiento de test/override no
   cambia).
2. Si no, orden por defecto: `OmniRoute` (si `OMNIROUTE_BASE_URL` está
   configurado) → `Pollinations`.
3. En `worker.rs`, en el punto donde hoy se llama a `provider.generate(&req)`
   una sola vez: intentar con el primer candidato; si devuelve
   `ProviderError` (cualquier variante, incluyendo timeout y `PaidDisabled`
   cuando corresponda), intentar con el siguiente candidato de la lista
   antes de marcar el job como fallido.
4. **Hallazgo crítico — el cost gate se evalúa antes de saber qué
   proveedor se usa de verdad.** Hoy `queue_generation_with_key()` elige
   el proveedor UNA vez al encolar el job (antes de generar), y con ese
   resultado calcula `is_paid` para correr `can_enqueue_generation()`, que
   chequea `policy.daily_paid_budget` (presupuesto real, no solo un
   booleano). Si en ese momento OmniRoute está configurado, `is_paid =
   false` y el chequeo de presupuesto pago **ni se ejecuta**. Si más
   tarde, ya en generación real, OmniRoute falla y el fallback usa
   Pollinations (pago), ese gasto nunca pasó por el control de
   presupuesto — riesgo real de gastar plata sin que el sistema se entere.
   Por eso, **antes de intentar Pollinations como fallback, hay que
   re-correr el mismo chequeo de presupuesto/cost gate que se corre al
   encolar** (no solo mirar el flag `paid_providers_enabled`), usando el
   presupuesto y el conteo del día actual. Si no alcanza presupuesto, el
   job falla con motivo explícito ("omniroute_failed_budget_exceeded" o
   similar) — nunca generar pagado saltándose el presupuesto diario.
5. Registrar en el job (`generation_jobs` / `generated_candidates`, campo
   `provider`) cuál de los dos se usó realmente, para que quede visible en
   la UI y en QA cuál fue el costo real de cada imagen.

Criterio de aceptación: con `OMNIROUTE_BASE_URL` apuntando a una URL que
falla (ej. puerto cerrado) y `VIGILCUT_PAID_PROVIDERS=1`, un job de
generación termina exitoso usando Pollinations, y el campo `provider`
guardado dice `pollinations`, no `omniroute`.

### Paso 4 — Tests

- Test unitario de la cadena de fallback: mock de OmniRoute que siempre
  falla + mock de Pollinations que siempre funciona → confirmar que el job
  termina `Completed` con `provider = pollinations`.
- Test unitario del cost gate: OmniRoute falla, Pollinations requeriría
  pago, `paid_providers_enabled = false` → confirmar que el job termina
  `Failed` con el motivo esperado, sin llamar nunca a Pollinations
  (se puede verificar con un contador de llamadas en el mock).
- **Test específico del hallazgo de presupuesto (Loop 2):** encolar un job
  con OmniRoute configurado (`is_paid=false` al encolar, presupuesto diario
  ya en 0 o consumido), luego forzar que OmniRoute falle en la generación.
  Confirmar que el fallback a Pollinations **no ocurre** porque el
  presupuesto no alcanza — el job debe fallar por presupuesto, no generar
  igual. Este test es el que hoy no existiría con el diseño ingenuo
  (proveedor fijado solo al encolar) y es el más importante de todo el
  ciclo.
- Test de compilación/arquitectura: el `grep` del Paso 1 como parte de un
  test o check de CI, para que no se rompa la regla a futuro.

### Paso 5 — Verificación manual

- Con OmniRoute local corriendo (o su mock de desarrollo): generar una
  imagen desde la Biblioteca standalone y confirmar que usa OmniRoute.
- Apagar/desconfigurar OmniRoute y repetir: confirmar que cae a
  Pollinations automáticamente (con `VIGILCUT_PAID_PROVIDERS=1`) o falla
  con mensaje claro (con `VIGILCUT_PAID_PROVIDERS=0`).
- Confirmar que B-roll (modo `brollOnly`, ya resuelto en CYCLE-002) sigue
  sin poder disparar `request_generation` ni `ingest_asset` directamente —
  esto no debería haber cambiado, pero es la intersección entre ambos
  ciclos y vale la pena confirmarlo una vez más.
- `cargo test` y `npm run check` sin errores nuevos.

## Resultado Grok

_(completar al terminar: commits, resultado de tests, cualquier desviación
del plan y por qué)_
