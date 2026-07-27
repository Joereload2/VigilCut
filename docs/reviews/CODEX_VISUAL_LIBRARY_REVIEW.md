# Codex Review  Visual Library

## Metadata

- Fecha: 2026-07-22 (America/Santiago).
- Rama: `feat/intelligent-clipping`.
- HEAD revisado: `b8200b331e4a9b80bc250cbd20ed0f08cd5ca448` (`fix(ui): empty state, resume queue, library search, reject chips, daily collapse`).
- Estado inicial de Git: limpio; `## feat/intelligent-clipping...origin/feat/intelligent-clipping`. `git fetch` completado y HEAD coincidía con `origin/feat/intelligent-clipping`.
- Alcance: dos loops de revisión estática y dinámica sobre dominio visual, SQLite, worker/daily feed, OmniRoute, costes/licencias, Supabase/RLS/Storage, UI Svelte, documentación y suites solicitadas. No se llamó OmniRoute real, Supabase de producción ni servicios pagados.

## Resumen ejecutivo

La separación de dominio es razonable: `MediaAsset`, `VisualConcept`, `VisualNeed` y `VisualPlacement` son tipos distintos; el matching busca antes de generar; SQLite persiste necesidades, jobs, candidatos y QA; el proveedor está abstraído; la descarga tiene límite de 25 MB y validación por magic bytes; la QA humana es el valor por defecto; las pruebas visuales y smoke pasan con mocks.

No funciona como sistema autónomo: no hay worker residente ni scheduler periódico. La UI llama `worker_tick`, los comandos de generar/regenerar esperan la generación, daily ejecuta un ciclo solo al activar el switch, y los jobs `running` sobreviven a un reinicio sin recuperación. La cancelación no aborta HTTP. La aprobación/promoción no es atómica y admite carreras. La política daily confía en `free_configured`, aunque el propio resultado queda `free_verified=false`. La protección SSRF no cubre DNS, redirects ni IPv6 correctamente. La licencia de toda imagen IA se eleva sin evidencia a `Owned` y `commercial_use=true`.

El proveedor `mock` es funcional solo como fixture local: crea una imagen sintética, no una generación IA local. Supabase es esquema/documentación “ready”, pero no existe cliente ni sincronización runtime. Además, las políticas Storage permiten a cualquier autenticado operar sobre cualquier objeto del bucket y varias tablas con RLS no tienen políticas utilizables.

Veredicto: **no fusionar**. Bloquean la fusión autonomía/recuperación del worker, garantía de coste, SSRF, atomicidad de aprobación y licenciamiento. Después deben cerrarse RLS/Storage o excluir explícitamente la migración remota del alcance de entrega.

## Hallazgos críticos

### CRIT-001 — No existe worker ni scheduler autónomo

- Archivo: `src-tauri/src/commands/visual_intel.rs:230-268,340-351`; `src-tauri/src/pipeline/visual/generation/daily_feed.rs:111-259`; `src/lib/components/visual/ImageGenerationPanel.svelte:121-130,263-309`.
- Función o componente: `visual_generate_need`, `visual_regenerate_need`, `run_daily_cycle`, `ImageGenerationPanel.startPoll/toggleDaily`.
- Evidencia concreta: generar/regenerar encolan y ejecutan `worker_tick(1).await`; el polling de Svelte ejecuta `visualWorkerTick(2)` cada 2 s; activar daily llama una sola vez `visualDailyFeedCycle`; `interval_minutes` se persiste pero no se consume. La búsqueda global no encontró `tokio::spawn`/`tokio::time::interval` para esta cola.
- Consecuencia: cerrar/no abrir el panel detiene jobs; daily no es diario ni periódico; el frontend es simultáneamente UI, scheduler y worker; una solicitud de hasta 90 s bloquea la acción IPC.
- Corrección mínima: iniciar desde `setup` de Tauri un supervisor único con `tokio::spawn`, intervalo cancelable y wake-up al encolar; comandos de generación deben devolver tras commit de `queued`; daily debe ser disparado por el supervisor según `enabled`, `interval_minutes` y `last_cycle_at`.
- Prueba de aceptación: encolar con el panel desmontado devuelve en <200 ms; el job llega a estado terminal; con reloj controlado daily no corre antes del intervalo y corre una vez al vencer; ninguna llamada de UI invoca `worker_tick`.

### CRIT-002 — No hay recuperación tras reinicio y un job `running` queda atascado

- Archivo: `src-tauri/src/pipeline/visual/generation/worker.rs:149-198,506-515`; `src-tauri/src/pipeline/visual/generation/supervision.rs:110-138`; `src/lib/components/visual/ImageGenerationPanel.svelte:298-304`.
- Función o componente: `process_next_job`, `worker_tick`, `resumeQueue`.
- Evidencia concreta: el selector solo toma `status='queued'`; no hay lease, heartbeat ni recuperación de `running`. `resumeQueue` solo se ejecuta si existe `projectKey` y solo procesa queued. Un cierre después del UPDATE a `running` deja el job invisible al worker para siempre.
- Consecuencia: estado imposible de resolver desde producto, necesidad marcada `Generating`, cancelación/regeneración confusa y bloqueo de prioridad daily por el conteo de `running`.
- Corrección mínima: añadir lease (`locked_by`, `lease_expires_at`, heartbeat) o, como MVP, transacción de startup que reencole `running` vencidos y reconcilie necesidad/candidato; limitar intentos y registrar motivo de recuperación.
- Prueba de aceptación: fixture SQLite con job `running` antiguo + reinicio; el supervisor lo reclama una sola vez, incrementa intento de forma definida y termina o falla; un `running` con lease vigente no se duplica.

### CRIT-003 — Cancelar no aborta la operación HTTP y la UI no ofrece cancelación durante la espera inicial

- Archivo: `src-tauri/src/pipeline/visual/generation/supervision.rs:352-395`; `src-tauri/src/pipeline/visual/generation/worker.rs:217-229`; `src-tauri/src/pipeline/visual/generation/omniroute.rs:119-136,150-194,277-317`; `src/lib/components/visual/ImageGenerationPanel.svelte:171-190`.
- Función o componente: `cancel_job`, `process_next_job`, `OmniRouteImageProvider::generate`, `generate`.
- Evidencia concreta: para running solo se escribe `cancel_requested=1`; el siguiente check ocurre después de `provider.generate(&req).await`. `reqwest` no recibe `CancellationToken` ni se usa `select!`; retries/backoff tampoco son cancelables. El frontend espera `visualGenerateNeed` antes de refrescar/pollear, por lo que no muestra el job mientras esa misma llamada bloquea.
- Consecuencia: el usuario puede esperar hasta timeout/retries, consumir cuota aun después de cancelar y recibir un mensaje de “cancelada” que no equivale a abortada.
- Corrección mínima: comando enqueue-only; token por job compartido con worker; envolver POST, descarga y backoff en `tokio::select!`; al cancelar abortar future, borrar temporal y transicionar atómicamente a `cancelled`.
- Prueba de aceptación: servidor mock mantiene respuesta abierta; cancelar completa en <500 ms, observa desconexión, no crea candidato, no incrementa contador y deja job/necesidad coherentes.

### CRIT-004 — SSRF evadible por DNS, redirects, IPv6 y parsing manual

- Archivo: `src-tauri/src/pipeline/visual/generation/omniroute.rs:238-286`.
- Función o componente: `download_url_streaming`.
- Evidencia concreta: solo se comparan strings para algunos IPv4; faltan 172.16/12, 127/8, 100.64/10, multicast/documentación y rangos IPv6. Un hostname público que resuelva a IP privada pasa. `Policy::limited(3)` sigue redirects sin revalidar destino. El parsing por `split` no normaliza host/puerto/IPv6 y la excepción localhost depende de que `base_url` contenga texto.
- Consecuencia: una respuesta de proveedor controlada puede leer endpoints locales/cloud metadata o pivotar por redirect/DNS rebinding.
- Corrección mínima: parsear con `url::Url`; permitir solo http/https; resolver todas las IP y rechazar cualquier dirección no global; deshabilitar redirects automáticos y validar/resolver cada `Location`; fijar la IP conectada o usar resolver seguro para evitar rebinding; aplicar la misma política al endpoint base configurable.
- Prueba de aceptación: tabla de tests para 127/8, 10/8, 172.16/12, 192.168/16, link-local, CGNAT, `::1`, ULA, IPv4-mapped IPv6, DNS→privada y redirect público→privado; todos fallan antes de enviar GET al destino protegido.

### CRIT-005 — Aprobación/promoción no atómica y vulnerable a doble ejecución

- Archivo: `src-tauri/src/pipeline/visual/generation/worker.rs:454-503,518-580`; `src-tauri/src/commands/visual_intel.rs:272-336`.
- Función o componente: `promote_candidate`, `human_approve_candidate`, `visual_approve_and_use`.
- Evidencia concreta: se lee `approved_asset_id`, se importa/copia asset, luego se actualizan asset, candidato y necesidad en conexiones/operaciones separadas. No hay `Transaction` ni UPDATE condicional. Dos callers pueden observar NULL y promover; un fallo entre pasos deja asset huérfano o candidato sin enlace. La colocación y `save_visual_plan` ocurren después y su error se descarta (`let _ =`).
- Consecuencia: duplicados parciales, estado aprobado incoherente, UI anunciando biblioteca aunque guardar el plan falle y reintentos no deterministas.
- Corrección mínima: transacción `IMMEDIATE`; claim condicional del candidato; promoción y updates en una unidad; clave única por candidate/origen; separar “aprobar” de “colocar” con resultado explícito y no ocultar fallo de persistencia del plan.
- Prueba de aceptación: dos aprobaciones concurrentes producen exactamente un asset y el mismo ID; fault injection tras copiar/insertar/update revierte DB y limpia temporal; fallo al guardar plan retorna error/estado parcial explícito, no éxito genérico.

### CRIT-006 — Daily puede ejecutar una ruta no verificada como gratuita

- Archivo: `src-tauri/src/pipeline/visual/generation/daily_feed.rs:144-153,216-228`; `src-tauri/src/pipeline/visual/generation/omniroute.rs:22-68,227-234,342-353`.
- Función o componente: `run_daily_cycle`, `is_free_tier`, `cost_kind`, `probe`.
- Evidencia concreta: `OMNIROUTE_FREE_TIER` por defecto es true; `is_free_tier()` devuelve `free_configured`; daily acepta ese valor. La generación y probe guardan explícitamente `free_verified=false`. Por tanto “solo gratis/local” depende de configuración no verificada.
- Consecuencia: alimentación oportunista puede gastar dinero sin interacción y viola el presupuesto daily cero.
- Corrección mínima: daily solo admite provider local o capability persistida `free_verified=true` con TTL/modelo exacto; `free_configured` nunca satisface el gate oportunista; default OmniRoute debe ser no verificado/no elegible.
- Prueba de aceptación: OmniRoute `free_configured=true`, `free_verified=false` devuelve `no_free_route` y no hace HTTP; mock local sí procede; capability verificada vencida vuelve a bloquear.

### CRIT-007 — Storage remoto rompe aislamiento entre usuarios

- Archivo: `supabase/migrations/20260722000000_visual_library.sql:349-365`.
- Función o componente: policies `visual_library_storage_*`.
- Evidencia concreta: SELECT/INSERT/UPDATE/DELETE solo exigen bucket y `auth.uid() IS NOT NULL`; no validan owner/workspace ni prefijo. Todo usuario autenticado puede leer, sobrescribir o borrar objetos de otros usuarios si conoce/lista rutas.
- Consecuencia: exposición y destrucción cruzada de assets; RLS de tablas no protege Storage.
- Corrección mínima: ruta con owner/workspace inmutable y policies que comparen `(storage.foldername(name))[1]` con `auth.uid()` o membership validada; preferir signed URLs server-side y bloquear overwrite ajeno.
- Prueba de aceptación: dos usuarios y dos workspaces; A no puede SELECT/INSERT/UPDATE/DELETE ruta de B, miembro autorizado sí, usuario anónimo nunca.

## Hallazgos altos

### HIGH-001 — Idempotencia daily bloquea regeneración después de rechazo

- Archivo/función: `daily_feed.rs:186-225` (`run_daily_cycle`), `worker.rs:82-95` (`queue_generation_with_key`).
- Evidencia: cada ciclo crea una necesidad nueva pero usa `daily:{concept}:v1`. Si el job previo quedó `succeeded` y el humano rechazó su candidato, el enqueue devuelve ese job y marca la necesidad nueva `Covered` sin asset. Solo `failed/cancelled` evitan `Generating`, pero tampoco crean un job nuevo con la misma UNIQUE key.
- Consecuencia: el concepto puede quedar en bucle falso de cobertura y no regenerarse.
- Corrección mínima: idempotencia por ciclo/intento persistido y estado de aceptación; rechazo incrementa versión del concepto dentro de transacción.
- Prueba de aceptación: generar→rechazar→siguiente ciclo crea job v2 distinto; aprobar→siguiente ciclo reutiliza asset y no genera.

### HIGH-002 — Métricas daily aprobadas/rechazadas/necesita revisión no se actualizan

- Archivo/función: `daily_feed.rs:66-108` (`bump_metric`, `week_summary`), `worker.rs:287-412,518-612`.
- Evidencia: `bump_metric` admite `approved`, `rejected`, `needs_review`, pero ninguna transición del worker ni acción humana las llama. El resumen afirma imágenes aprobadas basándose en ceros incompletos.
- Consecuencia: métricas y mensaje semanal engañosos; no se puede auditar efectividad/coste.
- Corrección mínima: registrar evento/transición exactamente una vez, asociado al origen daily y día de la transición; derivar o reconciliar métricas desde eventos.
- Prueba de aceptación: una aprobación, un rechazo y un needs-review daily incrementan una vez; reintentar la misma acción no duplica.

### HIGH-003 — Licencia IA se declara `Owned` y uso comercial sin verificar términos

- Archivo/función: `worker.rs:454-503` (`promote_candidate`).
- Evidencia: toda promoción llama `import_image(..., LicenseStatus::Owned)` y ejecuta `commercial_use=1`, aunque solo conserva provider/model/prompt y no captura licencia/ToS/version/receipt.
- Consecuencia: biblioteca comunica derechos que VigilCut no puede demostrar; riesgo legal en exportación comercial.
- Corrección mínima: default `Unknown` y `commercial_use=NULL/false`; registrar evidencia de licencia por provider/model/fecha y elevar solo mediante regla verificada.
- Prueba de aceptación: mock/OmniRoute sin evidencia quedan unknown; fixture con evidencia válida asigna estado correcto y attribution; UI no presenta permiso comercial sin respaldo.

### HIGH-004 — Claim de jobs no es atómico y dos workers pueden procesar el mismo job

- Archivo/función: `worker.rs:149-198` (`process_next_job`).
- Evidencia: SELECT del primer queued y UPDATE posterior no incluye `WHERE status='queued'` ni comprueba filas afectadas. No existe lock global del worker; UI, CLI y comandos pueden llamar ticks concurrentes.
- Consecuencia: doble solicitud/coste, dos candidatos y carreras de estado.
- Corrección mínima: claim transaccional `UPDATE ... WHERE id=(SELECT...) AND status='queued' RETURNING ...` con lease/worker ID.
- Prueba de aceptación: 20 workers concurrentes sobre un job provocan una llamada al provider y un candidato.

### HIGH-005 — Fallback EDL inventa 60 segundos y oculta errores reales

- Archivo/función: `src-tauri/src/commands/visual_intel.rs:291-294` (`visual_approve_and_use`).
- Evidencia: cualquier error de `edl_helper` se reemplaza por `Edl::from_remove_spans(..., 60.0, &[])`.
- Consecuencia: placements fuera de duración o desalineados; se transforma un fallo verificable en plan aparentemente válido.
- Corrección mínima: requerir EDL/duración real vía ffprobe/cache; si falta, aprobar a biblioteca pero rechazar `place=true` con error accionable.
- Prueba de aceptación: vídeo de 12 s sin EDL nunca crea EDL de 60 s; `place=true` falla o usa 12 s medidos.

### HIGH-006 — Supabase está preparado, no conectado, y la migración RLS queda parcialmente inutilizable

- Archivo/función: `docs/visual-library/database.md:16-20`; búsqueda runtime completa; `supabase/...sql:267-347`.
- Evidencia: documentación dice “future client”; no hay dependencia/cliente Supabase ni llamadas Storage/sync en `src`/`src-tauri`. Se habilita RLS en 14 tablas, pero solo themes, concepts, assets, needs, jobs y provider capabilities reciben policies; workspaces, memberships, asset_concepts, candidates, QA, usage, assignments y sync_state quedan sin acceso normal. Jobs tampoco tienen DELETE; candidates/QA no tienen operación alguna.
- Consecuencia: no existe sincronización local-remota y un despliegue de la migración no soporta el flujo descrito.
- Corrección mínima: decidir alcance: (A) retirar Supabase de la entrega y marcarlo diseño futuro, o (B) implementar cliente/sync/outbox/conflictos y matriz completa de policies con tests locales; no mantener estado intermedio publicitado como funcional.
- Prueba de aceptación: si B, integración local Supabase demuestra push/pull, retry/idempotencia y acceso CRUD por rol en cada tabla; si A, UI/docs no prometen sync y migración no es requisito de release.

### HIGH-007 — Regeneración descarta candidato antes de garantizar nuevo job

- Archivo/función: `supervision.rs:407-430` (`queue_regenerate`).
- Evidencia: marca el candidato previo `discarded`, calcula versión por COUNT y después intenta encolar. Un fallo de policy/DB deja perdido el candidato anterior; COUNT+1 no es una secuencia segura ante concurrencia.
- Consecuencia: pérdida lógica y colisión de idempotencia en doble regeneración.
- Corrección mínima: transacción; reservar versión con UNIQUE/retry y descartar anterior solo tras insertar el nuevo queued.
- Prueba de aceptación: policy deny conserva candidato; dos regeneraciones concurrentes crean como máximo un job activo sin descartar indebidamente.

### HIGH-008 — Daily no es accesible ni operable sin `projectKey`

- Archivo/función: `ImageGenerationPanel.svelte:66-71,106-119,298-309,676-723`.
- Evidencia: el bloque daily se renderiza, pero `refresh()` retorna sin `projectKey`, el effect no carga snapshot/settings y `dailyPending` depende de `snap.pendingReview`. Usuario sin video ve switch con estado por defecto y no ve candidatos reales.
- Consecuencia: contradice alimentación sin video y bloquea flujo de usuario nuevo.
- Corrección mínima: endpoint/snapshot global daily independiente del proyecto; cargar settings/candidatos al montar; separar panel global de supervisión del proyecto.
- Prueba de aceptación: app recién abierta sin video muestra estado persistido, conceptos/contexto y candidatos daily; aprobar a biblioteca funciona; “usar” permanece oculto.

## Hallazgos medios

### MED-001 — Candidato daily muestra UUID, no concepto/contexto

- Archivo/función: `ImageGenerationPanel.svelte:699-720`; `supervision.rs:38-63` (`CandidateView`).
- Evidencia: fila usa `dc.id.slice(0,8)` y el DTO no incluye título de concepto, label de need ni contextos.
- Consecuencia: revisión humana sin información semántica suficiente.
- Corrección mínima: join candidate→job/need→concept y mostrar título, prompt resumido, contextos positivos/negativos y QA.
- Prueba de aceptación: fixture daily renderiza título/contexto legibles y no depende del UUID para decidir.

### MED-002 — Mock se presenta como generación iniciada/local sin distinción inequívoca

- Archivo/función: `ImageGenerationPanel.svelte:171-184,562-574`; `docs/visual-library/supervision-ui.md:9`; `generation/mock.rs`.
- Evidencia: el mock escribe PNG sintético; mensaje dice “Generación iniciada” y la UI solo muestra coste/provider en detalles dispersos. La documentación habla de “mock offline”, pero el flujo principal puede interpretarse como IA local.
- Consecuencia: demo confundida con capacidad real.
- Corrección mínima: badge persistente “Simulación (mock), no IA” en job/candidato y CTA; no usar “generación local” para mock.
- Prueba de aceptación: provider mock siempre muestra el badge en lista, preview y toast.

### MED-003 — `ImageGenerationPanel` concentra demasiadas responsabilidades

- Archivo/componente: `ImageGenerationPanel.svelte` (725 líneas).
- Evidencia: carga de datos, worker polling, daily scheduler, filtros, preview, generación, aprobación/placement, rechazo, accesibilidad y render viven juntos.
- Consecuencia: estados globales (`rejectOpen`, `rejectReason`, `globalBusy`) se comparten entre candidatos y dificultan pruebas/flujo sin proyecto.
- Corrección mínima: extraer store/controller; `NeedList`, `CandidateReview`, `DailyFeedPanel`, `JobProgress`; eliminar worker control del frontend.
- Prueba de aceptación: componentes con props/eventos tipados y tests separados; daily monta sin proyecto.

### MED-004 — Test `daily_then_video_reuses` no prueba reutilización

- Archivo/función: `daily_feed.rs:262-293`.
- Evidencia: acepta “may generate or reuse” y solo hace `assert!(r.get("ok").is_some())`; no crea necesidad de video, no ejecuta matching sobre ella ni compara asset IDs.
- Consecuencia: pasa aunque daily falle (`ok:false`) y aunque nunca haya reutilización.
- Corrección mínima: exigir `ok==true`, aprobar candidato si aplica, crear need video semánticamente equivalente, ejecutar match y afirmar mismo asset y cero jobs adicionales.
- Prueba de aceptación: el test falla al desactivar matching o cambiar concepto.

### MED-005 — Estados SQLite carecen de constraints y FKs suficientes

- Archivo: `schema.rs:228-279`.
- Evidencia: `status`, `stage`, `origin`, `cost_kind` son TEXT sin CHECK; `need_id`, `job_id`, `concept_id` no declaran FK; se ignoran todos los errores de ALTER (`let _ =`).
- Consecuencia: estados imposibles, huérfanos y migraciones parcialmente aplicadas sin señal.
- Corrección mínima: migración versionada con CHECK/FK e integridad; inspeccionar columnas antes de ALTER y propagar errores inesperados.
- Prueba de aceptación: inserts inválidos fallan; migrar esquema anterior dos veces es idempotente y verificable.

### MED-006 — Errores del worker se convierten en éxito aparente

- Archivo/función: `visual_intel.rs:241-249,259-267`; `daily_feed.rs:225-240`.
- Evidencia: `worker_tick(1).await.unwrap_or(0)` traga error y responde `action:queued/generated` con `processed:0`.
- Consecuencia: mensajes optimistas, job posiblemente atascado y diagnóstico perdido.
- Corrección mínima: con enqueue-only, responder queued; worker persiste error estructurado. Mientras tanto, no usar `unwrap_or(0)` y distinguir enqueue de ejecución.
- Prueba de aceptación: provider/DB fault deja job failed/retry con error visible; command no afirma generated.

### MED-007 — Cobertura de accesibilidad y feedback es insuficiente

- Archivo/componente: `ImageGenerationPanel.svelte:312-725`; resultado `npm run check`.
- Evidencia: imágenes daily usan `alt=""`; botones “OK/No” carecen de nombre contextual; no hay `aria-live` local para progreso/cancelación ni progreso cuantificado. El check global además reporta un warning de dialog en `ExportSuccess.svelte:50`.
- Consecuencia: revisión por lector de pantalla ambigua y progreso poco comprensible.
- Corrección mínima: labels con concepto, región `aria-live`, estados ocupados/foco al abrir rechazo y texto “Aprobar en biblioteca/Rechazar”; corregir warning global.
- Prueba de aceptación: axe/keyboard flow sin violaciones críticas; nombres accesibles incluyen concepto y acción.

### MED-008 — La UI mezcla estado persistido y estado local de forma engañosa

- Archivo/función: `ImageGenerationPanel.svelte:282-296`; `supervision.rs:87-157,352-388`.
- Evidencia: `dailyEnabled=v` se aplica localmente antes de recargar; cancel running queda con status `running` + `cancel_requested`, pero `map_ui_state` lo etiqueta “Generando imagen”, no “Cancelando”; mensaje inmediato dice “Generación cancelada”.
- Consecuencia: lo mostrado no coincide con SQLite.
- Corrección mínima: derivar siempre de snapshot; introducir UI state `cancelling`; toast “Cancelación solicitada” hasta estado terminal.
- Prueba de aceptación: running cancel_requested renderiza Cancelando y solo anuncia cancelada al persistirse `cancelled`.

## Simplificaciones

1. Eliminar `visual_worker_tick` de la API pública UI y todo polling que ejecute trabajo; conservar un supervisor Rust y eventos/snapshot de solo lectura.
2. Separar daily global de necesidades de proyecto. Ambos pueden reutilizar una sola cola y una sola máquina de estados; daily solo aporta origen, prioridad y política de coste.
3. Unificar transiciones en un repositorio transaccional (`claim`, `complete`, `fail`, `cancel`, `approve`, `reject`) y prohibir UPDATE dispersos desde worker/supervision/commands.
4. Derivar métricas desde eventos de transición idempotentes en vez de contadores manuales incompletos.
5. Sustituir la versión calculada por COUNT por un `generation_attempt` monotónico/UNIQUE por need o concept.
6. Tratar mock exclusivamente como fixture/demo con etiqueta; no mezclarlo con “local AI”.
7. Elegir Supabase runtime real o diseño futuro. Si es futuro, retirar tablas/policies incompletas del criterio de “implementado”; si es real, usar outbox local y una frontera `SyncService` única.

## UX/UI

Flujo mínimo recomendado:

1. Al abrir la app, cargar una sección global “Biblioteca automática” sin video: estado real, próxima ejecución, provider/coste verificado y candidatos con concepto/contexto.
2. Al abrir un video, detectar necesidades y mostrar primero coincidencias de biblioteca; CTA de generación solo para faltantes.
3. “Generar” retorna inmediatamente y muestra En cola → Generando → Revisando → Necesita revisión mediante eventos/snapshot; Cancelar cambia a Cancelando y permanece disponible.
4. Preview muestra imagen, concepto, contexto permitido/prohibido, provider/model, coste verificado y licencia. Mock lleva badge inequívoco.
5. Candidato de video: “Aprobar en biblioteca” y, solo si hay video/EDL válido, “Aprobar y colocar”. Candidato daily: nunca “Aprobar y usar”; solo biblioteca/rechazo.
6. Rechazar solicita motivo y ofrece regenerar; regenerar no descarta el anterior hasta que el nuevo job quede encolado.
7. Reintento y recuperación explican intento, último error y si el job fue recuperado tras reinicio.

## Plan de implementación para Grok

1. Congelar contratos de estados y escribir tests de concurrencia/reinicio/cancelación/coste que actualmente fallen.
2. Crear repositorio transaccional SQLite con claim+lease, transición validada e idempotencia monotónica. Dependencia de todos los pasos siguientes.
3. Implementar supervisor Rust residente y recuperación startup; convertir IPC generate/regenerate a enqueue-only; retirar ticks de Svelte/CLI como mecanismo operativo.
4. Añadir cancelación cooperativa real al provider, descarga y backoff; exponer `cancelling`.
5. Endurecer política de costes: daily solo local o `free_verified`; reconciliar contadores/eventos.
6. Hacer approval/reject/regenerate transaccionales, añadir licencia unknown por defecto y eliminar fallback EDL 60 s.
7. Endurecer OmniRoute SSRF con URL+DNS+redirect por salto y tests de servidor local controlado.
8. Dividir UI y habilitar daily global; enriquecer DTO de candidato con concepto/contexto; corregir mensajes/accesibilidad/mock badge.
9. Decidir Supabase A/B. Si se mantiene: corregir Storage, completar RLS, añadir outbox/sync y suite Supabase local. No conectar producción.
10. Corregir Clippy, fortalecer tests engañosos y ejecutar gates completos hasta cero fallos/warnings acordados.

## Pruebas obligatorias

- `enqueue_returns_immediately_without_panel`: timeout 200 ms y procesamiento con panel desmontado.
- `startup_recovers_expired_running_once`: lease vencido se procesa una vez; lease vigente no.
- `two_workers_claim_once`: 20 ticks concurrentes, una llamada provider/un candidato.
- `cancel_aborts_http_and_download`: servidor colgado observa disconnect <500 ms; cero candidato/contador.
- `daily_reject_then_v2`: rechazo produce intento nuevo; aprobación produce reutilización.
- `daily_requires_free_verified`: configured/no verificado no toca red.
- `approval_is_atomic_and_idempotent`: doble aprobación concurrente, un asset; fault injection revierte.
- `regenerate_policy_failure_preserves_candidate`: sin descarte prematuro.
- `no_fake_60s_edl`: duración real o error explícito.
- `ai_license_defaults_unknown`: sin evidencia no hay owned/commercial.
- `ssrf_matrix_and_redirects`: rangos IPv4/IPv6, DNS privado, rebinding controlado y redirect por salto.
- `daily_then_video_reuses_exact_asset`: mismo asset ID y ningún job nuevo.
- `daily_ui_without_project`: settings/candidatos/concepto disponibles sin `projectKey`; sin CTA de placement.
- `ui_persisted_state_parity`: queued/running/cancel_requested/failed/review/approved/rejected mapean a textos y acciones esperados.
- Supabase local, si entra en alcance: matriz CRUD por tabla/rol y Storage cross-tenant denegado; outbox reintenta sin duplicar.

## Definition of Done

- Todos los bloqueadores CRIT y HIGH cerrados con tests anteriores.
- Ningún trabajo depende de un panel Svelte, video abierto o llamada manual a tick.
- Reinicio no deja jobs running indefinidos; todos tienen lease/heartbeat y transición terminal/retry limitada.
- Cancelar aborta trabajo externo y no consume/guarda resultado posterior.
- Daily jamás usa ruta pagada o no verificada y respeta intervalo/cap de forma persistente.
- Aprobación/rechazo/regeneración son atómicos e idempotentes bajo concurrencia.
- Licencia y uso comercial reflejan evidencia real.
- SSRF protege URL inicial, DNS y cada redirect para IPv4/IPv6.
- Daily funciona sin proyecto y muestra contexto; mock se identifica como simulación.
- Supabase está explícitamente fuera de alcance o conectado con sync, RLS y Storage aislados y probados.
- `git diff --check`, check, build, fmt, Clippy, unit visual y smoke terminan en 0; Clippy no contiene warnings.

## Comandos ejecutados

- `git status --short --branch` → limpio; rama esperada siguiendo origin.
- `git branch --show-current` → `feat/intelligent-clipping`.
- `git rev-parse HEAD` → `b8200b331e4a9b80bc250cbd20ed0f08cd5ca448`.
- `git fetch` → éxito (requirió ejecución fuera del sandbox para escribir `.git/FETCH_HEAD`).
- `git diff --check` inicial → éxito.
- `npm run check` → exit 0; 0 errores, 1 warning de accesibilidad en `ExportSuccess.svelte:50`. La invocación `npm` directa fue bloqueada por execution policy de PowerShell; se repitió como `npm.cmd`.
- `npm run build` → primer intento falló por acceso de sandbox a `vite.config.ts`; repetido fuera del sandbox: exit 0, 143 módulos, build Vite completado; conserva warning de accesibilidad y warning de import dinámico/estático.
- `npm run test:fmt` → exit 0; warning ambiental `could not canonicalize path C:\Users\jose`.
- `npm run test:clippy` → **exit 1**: import no usado `save_needs` en `src-tauri/src/commands/visual_intel.rs:28`, convertido en error por `-D warnings`.
- `npm run test:unit:visual` → exit 0; 29 passed, 0 failed, 55 filtered; warning por el mismo import. Algunos tests tardaron >60 s.
- `npm run test:smoke` → exit 0; 7 passed en cuatro binarios (2 clipping, 2 pipeline, 2 visual, 1 visual_intel); usó fixtures/mock, no OmniRoute real. Warnings por import y linker.
- `git diff --check` final → se ejecutará tras cerrar este informe; su resultado se actualizará antes del commit.

## Limitaciones de la revisión

- No se ejecutó OmniRoute real ni se validaron precios/capabilities contra un servicio externo; el análisis se hizo sobre código y mocks por restricción expresa.
- No se conectó Supabase ni Storage, producción o local; no existe cliente runtime en el repositorio. RLS/Storage se evaluaron estáticamente.
- No se verificó DNS rebinding/redirect SSRF dinámicamente porque no hay tests seguros incluidos; la falla se demuestra por ausencia de resolución/revalidación en código.
- No se ejecutó una sesión GUI Tauri con lector de pantalla; UX/accesibilidad se revisaron por código y `svelte-check`.
- No se probó un crash real a mitad de request ni doble ventana; las carreras se derivan de SELECT/UPDATE no atómicos y falta de lease/token.
- Las suites generaron artefactos ignorados en `dist`/`target`; no se editaron producción, migraciones ni tests.
