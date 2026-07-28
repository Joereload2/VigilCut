# Codex → Grok — handoff de ciclos

## Programación del scheduler

Grok revisa este archivo **6 veces al día, hora de Chile continental
(`America/Santiago`)**: **18:00, 22:00, 02:00, 06:00, 10:00, 14:00.**

Equivalente en UTC ahora mismo (Chile en horario de invierno, UTC-4,
vigente hasta el 5 de septiembre de 2026): 22:00, 02:00, 06:00, 10:00,
14:00, 18:00 UTC — el mismo conjunto de horas, corrido de lugar, porque
el paso de 4 horas coincide con el offset actual.

**Atención — esto se corre el 5 de septiembre de 2026:** ese día Chile
pasa a horario de verano (UTC-3). Si la infraestructura real de Grok
programa estas 6 revisiones con horas fijas en UTC en vez de con zona
horaria `America/Santiago`, a partir de esa fecha las revisiones van a
caer una hora más tarde en hora de Chile, salvo que alguien actualice el
cron ese día a mano. Esto no se resuelve desde este archivo — es
configuración externa al repo. Si la infraestructura de Grok soporta zona
horaria IANA (`America/Santiago`) en vez de offset UTC fijo, usar esa
opción evita el problema por completo.

En cada una de las 6 revisiones diarias, Grok hace esto, en este orden:

1. Lee el campo **Estado del scheduler de Grok** (más abajo). Si dice
   `DETENIDO`, no hace nada más — no busca ciclos, no toca código, no se
   reprograma. Queda esperando a que Codex lo vuelva a poner en `ACTIVO`
   antes de la próxima revisión programada.
2. Si dice `ACTIVO`, busca entradas con `Estado: PENDIENTE` en este
   archivo y las ejecuta según el protocolo descrito en cada una.

**Estado del scheduler de Grok: ACTIVO**

Codex edita esta línea. Normalmente no tiene motivo para tocarla — la
apaga (`DETENIDO`) únicamente como parte del escalamiento de seguridad
descrito en la sección de Codex más abajo, nunca por su cuenta fuera de
ese caso. Fuera de esa excepción, solo la persona responsable la
reactiva.

## Programación de Codex (revisión de lo que hizo Grok)

Codex revisa este archivo **6 veces al día, hora de Chile continental
(`America/Santiago`), cada 4 horas empezando a las 20:00**: **20:00,
00:00, 04:00, 08:00, 12:00, 16:00.**

Esto cae, siempre, **2 horas después de la revisión de Grok
inmediatamente anterior** (Grok a las 18:00 → Codex a las 20:00; Grok a
las 22:00 → Codex a las 00:00; y así) — le da a Grok una corrida entera
de margen antes de que Codex evalúe lo que produjo, en vez de revisar
sobre trabajo recién empezado.

Equivalente en UTC ahora mismo (mismo horario de invierno que arriba):
00:00, 04:00, 08:00, 12:00, 16:00, 20:00 UTC. Aplica el mismo aviso del 5
de septiembre de 2026 de la sección anterior — no se repite acá, pero
rige igual para este horario.

En cada una de las 6 revisiones diarias, Codex hace esto, en este orden:

1. Lee **Estado del scheduler de Codex** (más abajo). Si dice
   `DETENIDO`, no hace nada más.
2. Busca ciclos con `Estado: RESUELTO POR GROK` que todavía no tengan
   una línea `Verificado por Codex:` en su entrada.
3. Para cada uno de esos ciclos, hace **como mínimo 3 loops de
   revisión**, cada uno desde un punto de vista distinto y funcional al
   problema concreto del ciclo — nunca repetir el mismo ángulo tres
   veces en la misma revisión. Elegir entre (rotando cuáles se usan de
   una revisión a la siguiente para no mirar siempre por el mismo lado):
   - **Agente ejecutor en frío:** releer el "Resultado Grok" como si no
     se supiera nada más del ciclo — ¿la evidencia ahí alcanza para
     creer que está resuelto, o falta un criterio de aceptación sin
     verificar?
   - **Dinero/seguridad:** ¿lo implementado toca la Sección 4 o 5 de
     `AGENTS.md`? Si sí, ¿el "Resultado Grok" prueba explícitamente que
     el gate correspondiente sigue funcionando, con un caso de prueba
     concreto, o lo da por sentado?
   - **Adversarial:** si alguien intentara evadir la regla que este
     ciclo debía imponer, ¿el código nuevo se lo permite?
   - **Mantenedor futuro:** ¿el cambio dejó algo desactualizado en
     `AGENTS.md` u otro doc, sin corregir (ver Sección 0 y 11.6 de
     `AGENTS.md`)?
   - **Asunción no verificada:** releer el plan original del ciclo
     contra el código real tocado, buscando específicamente una
     asunción del plan que nadie haya confirmado contra el código —
     no alcanza con razonar en abstracto, cada loop implica mirar
     código real (`grep`/`view`), igual que exige `AGENTS.md`.
4. Si los 3+ loops no encuentran problemas: agregar
   `Verificado por Codex: sí, <fecha>` a la entrada del ciclo. Queda
   cerrado.
5. Si algún loop encuentra un problema: **no reescribir el "Resultado
   Grok"** — ese es el registro de lo que Grok reportó, no se edita.
   En cambio:
   - Si es un ajuste acotado dentro del mismo alcance: volver a poner
     `Estado: PENDIENTE` en ese mismo ciclo, y agregar una subsección
     "Corrección de Codex tras verificación" en su documento de detalle,
     con evidencia archivo+línea, igual que cualquier hallazgo nuevo.
   - Si es un problema nuevo y separado, no del mismo alcance: crear un
     ciclo nuevo (`CYCLE-NNN` siguiente), nunca mezclarlo con el
     anterior — un ciclo, un problema.
   - **Excepción de seguridad:** si la corrección toca la Sección 4
     (dinero) o 5 (secretos) de `AGENTS.md`, o si este es ya el tercer
     ciclo consecutivo que Codex reabre sobre el mismo `cycle_id`, no
     poner `Estado: PENDIENTE` — poner
     `Estado: PENDIENTE — REQUIERE REVISIÓN HUMANA`. Grok busca
     literalmente el texto `Estado: PENDIENTE`, así que este estado
     distinto lo deja fuera de su búsqueda automáticamente: no vuelve a
     tocarlo hasta que una persona lo revise y lo cambie de nuevo a
     `PENDIENTE` a mano. **En el mismo momento en que Codex escribe este
     estado, también pone `Estado del scheduler de Grok: DETENIDO` Y
     `Estado del scheduler de Codex: DETENIDO`** — Codex nunca manda a
     parar a Grok y sigue funcionando por su cuenta; si el motivo alcanza
     para frenar a Grok, alcanza para frenarse a sí mismo también, hasta
     que una persona revise y reactive los dos a mano.
6. Cada revisión de Codex deja constancia en este archivo de qué ciclos
   revisó y con qué resultado, aunque el resultado sea "sin novedad" —
   silencio no es lo mismo que "se revisó y estaba bien".

**Estado del scheduler de Codex: ACTIVO**

Codex puede escribir `DETENIDO` acá **únicamente en el mismo momento en
que también pone en `DETENIDO` el de Grok**, como parte del escalamiento
de seguridad del paso 5. Fuera de ese caso puntual, Codex nunca toca este
valor. Y en ningún caso —ni siquiera en ese escalamiento— Codex se
reactiva a sí mismo escribiendo `ACTIVO` acá: eso es exclusivo de la
persona responsable del proyecto, siempre.

## Programación de Codex (revisión y corrección)

Codex revisa este archivo y el estado real del repo **6 veces al día,
hora de Chile continental (`America/Santiago`)**: **20:00, 00:00, 04:00,
08:00, 12:00, 16:00.** Esto es cada 4 horas, empezando a las 20:00, y
queda **2 horas después de cada revisión de Grok** (18→20, 22→00, 02→04,
06→08, 10→12, 14→16) — el margen existe a propósito, para darle a Grok
tiempo de terminar un ciclo antes de que Codex lo evalúe; no correr la
revisión de Codex más cerca de la de Grok sin ajustar esto también.

Mismo caveat de horario de invierno/verano que la sección anterior:
válido con Chile en UTC-4 (hasta el 5 de septiembre de 2026); si el
scheduler real usa UTC fijo en vez de zona horaria `America/Santiago`, se
desalinea ese día — mismo problema, misma solución.

**Estado del scheduler de Codex: ACTIVO**

Campo independiente del de Grok — se puede pausar la revisión de Codex
sin pausar la ejecución de Grok, o viceversa. Mismo mecanismo: cambiar a
`DETENIDO` para detener, sin acción especial requerida más que leerlo.

*(Reactivados 2026-07-28 por la persona responsable tras corregir el
hallazgo de dinero de CYCLE-003 y resolver CYCLE-004.)*

### Qué hace Codex en cada revisión

1. Lee su propio campo de estado primero. Si `DETENIDO`, no hace nada
   más.
2. Si `ACTIVO`, identifica:
   - Ciclos marcados `RESUELTO POR GROK` desde la última revisión de
     Codex (a verificar).
   - Ciclos `PENDIENTE` que llevan más de una revisión de Codex sin que
     Grok los tome (posible bloqueo a diagnosticar, no a resolver Codex
     mismo).
3. Para cada ciclo a verificar, corre **como mínimo 3 loops de revisión,
   cada uno desde un punto de vista distinto y funcional al problema
   concreto** — no 3 repasos superficiales del mismo ángulo con otras
   palabras. Elegir 3 (o más) de este menú según lo que el ciclo haya
   tocado, y dejar escrito cuáles se usaron:
   - **Evidencia real, no memoria:** ¿Lo que dice "Resultado Grok" se
     puede confirmar con `git log`, el resultado real de los comandos de
     la Sección 3 de `AGENTS.md`, o una búsqueda en el código tal como
     quedó? Si Codex no puede verificarlo de forma independiente, no se
     da por bueno solo porque el texto suena completo.
   - **Dinero y secretos (obligatorio si el ciclo tocó Sección 4 o 5 de
     `AGENTS.md`):** ¿Hay algún camino, aunque sea de borde, donde el
     cambio termine gastando plata sin pasar por el gate de presupuesto,
     o exponiendo una credencial? Ver CYCLE-003 como caso de referencia
     de este tipo de hallazgo.
   - **El agente ejecutor en frío:** si otra instancia de Grok leyera
     solo este ciclo y `AGENTS.md`, sin el historial de la conversación
     que lo originó, ¿tiene todo lo que necesita? ¿Alguna referencia
     colgante, alguna asunción no verificada?
   - **Deriva de documentación:** ¿Algo que el ciclo asumía sobre el
     código sigue siendo cierto hoy, o cambió por otro ciclo resuelto
     entre medio? Este repo ya tuvo el problema de docs desalineados
     (`ROADMAP.md` vs `QA_REPORT.md`) — no repetirlo acá.
   - **Alcance:** ¿Grok tocó algo fuera de lo declarado en
     "Fuera de alcance" del ciclo? Revisar el diff real, no solo el
     resumen que Grok escribió de sí mismo.
4. Si algún loop encuentra un problema: Codex **no corrige código
   directamente** — redacta un nuevo ciclo `CYCLE-NNN` con el mismo
   estándar que los anteriores (causa raíz con evidencia, alcance
   dentro/fuera, pasos verificables por comando), lo deja `PENDIENTE`
   para que Grok lo tome en su próxima revisión.
5. Si los 3+ loops no encuentran problemas: Codex deja constancia
   explícita agregando una subsección `### Revisión Codex` debajo de
   `### Resultado Grok` en el ciclo correspondiente, con: fecha, qué
   ángulos se usaron, y qué se verificó concretamente (no alcanza con
   "revisado, todo bien").

---

## CYCLE-000

- Rol: Sistema
- Estado: RESUELTO POR GROK
- Base HEAD: ef1f982b363fee904dca68617144bfdd67340377
- Fecha: 2026-07-23
- Prioridad: baja
- cycle_id: CYCLE-000

### Instrucciones para Grok

Ciclo de arranque del protocolo. No requiere implementación de producto.

### Criterios de aceptación

- [x] Archivo de handoff creado
- [x] Scheduler Grok activo (intervalo 10m)
- [x] Tracker de último cycle_id listo

### Resultado Grok

Protocolo activado. Esperando el siguiente ciclo `PENDIENTE` de Codex.

### Revisión Codex

- Fecha: 2026-07-28.
- Ángulos: evidencia real, agente ejecutor en frío y deriva documental.
- Evidencia: `git log` confirma el handoff y
  `docs/reviews/.grok_last_cycle.json` existe. El scheduler efectivo es
  infraestructura externa; se validan los artefactos versionados, no su
  ejecución externa.

---

## CYCLE-001

- Rol: Product Manager
- Estado: RESUELTO POR GROK
- Base HEAD: 9ef5b2e42cd34ba384c2b6312721dc0dd92c0d53
- Fecha: 2026-07-23
- Prioridad: alta
- cycle_id: CYCLE-001
- Detalle completo: `docs/reviews/CYCLE-001_PM.md`

### Instrucciones para Grok

Ver `CYCLE-001_PM.md` (PM-001 layout, PM-002 scene context, PM-003 single placement write, PM-004 reject-then-regenerate, PM-005 import without place).

### Resultado Grok

Implementado 2026-07-23. Tests: `intel_flow_tests` 2 passed; `npm run check` 0 errors. Detalle en `CYCLE-001_PM.md` § Resultado Grok.

### Revisión Codex

- Fecha: 2026-07-28.
- Ángulos: evidencia real, alcance y deriva/adversarial.
- Evidencia: `f3ab841` contiene operación única, test idempotente, contexto de
  escena, import con confirmación y rechazo progresivo. `npm run check`: 0
  errores; suite visual serial: 49/49.
- Resultado: verificado. `d45ede9` reintrodujo después regeneración previa al
  rechazo; la regresión separada queda en CYCLE-004.

---

## CYCLE-002

- Rol: Product Manager / Arquitectura
- Estado: RESUELTO POR GROK
- Base HEAD: ab1e51932d5cd50c6d637b1d01086ab9d898e2af
- Fecha: 2026-07-27
- Prioridad: alta
- cycle_id: CYCLE-002
- Detalle completo: `docs/reviews/CYCLE-002_BROLL_CONSULTIVE.md`

### Instrucciones para Grok

B-roll debe volver a ser solo consultivo respecto de la Biblioteca: puede
buscar y usar assets existentes, pero no puede importar imágenes, importar
carpetas ni disparar generación automática. Causa raíz identificada: el
prop `brollOnly` de `VisualWorkspace.svelte` nunca se activa desde
`VisualPanel.svelte` (el consumidor real de B-roll), y aunque se activara,
el header de controles de escritura no lo respeta. Ver
`CYCLE-002_BROLL_CONSULTIVE.md` para el paso a paso completo (5 pasos:
activar el prop, ocultar controles de escritura, verificar el atajo de
import en el picker, test de regresión, verificación manual).

### Resultado Grok

Implementado 2026-07-27. Ver detalle completo en
`CYCLE-002_BROLL_CONSULTIVE.md` § Resultado Grok.

Resumen:
- `VisualPanel.svelte` pasa `brollOnly={true}` (panel solo monta con video).
- `VisualWorkspace.svelte`: controles de escritura gated por `!brollOnly`;
  fix del `$effect` de vista que expulsaba de `"video"` cuando `brollOnly`.
- `importImage` picker branch intacta (Paso 3, verificación sin edición).
- Sin runner de tests de componentes Svelte: smoke manual documentado.
- `npm run check`: 0 errors (1 warning a11y preexistente en ExportSuccess).
- `npm run test:unit`: 105 passed, 0 failed.
- Sin tocar Rust de producto ni secretos ni dinero (Sección 4/5 AGENTS.md).

### Revisión Codex

- Fecha: 2026-07-28.
- Ángulos: ejecutor en frío, adversarial y alcance/deriva.
- Evidencia: `VisualPanel.svelte:564` activa `brollOnly`;
  `VisualWorkspace.svelte:640-721` oculta importación, daily y escaneo; el
  picker no ofrece escritura. `410725b` no tocó Rust ni proveedores.
  `npm run check` pasó y la suite visual serial terminó 49/49.
- Resultado: verificado; B-roll continúa consultivo.

---

## CYCLE-003

- Rol: Product Manager / Arquitectura
- Estado: RESUELTO POR GROK
- Base HEAD: ab1e51932d5cd50c6d637b1d01086ab9d898e2af (ejecutado después
  de CYCLE-002 `410725b`)
- Fecha: 2026-07-27
- Prioridad: alta
- cycle_id: CYCLE-003
- Detalle completo: `docs/reviews/CYCLE-003_LIBRARY_SEPARATION_OMNIROUTE.md`

### Instrucciones para Grok

Dos problemas relacionados en la Biblioteca: (1) carga y consulta de
imágenes están mezcladas en el mismo servicio y ambas dependen sin control
de `pipeline::visual::library` legacy — hay que separarlas en dos traits
claros (`LibraryIngestion` / `LibraryQuery`) detrás de un único adaptador
de infraestructura, con alcance verificado de 6 archivos concretos (ver
detalle). (2) la generación de imágenes no tiene cadena de fallback — hoy
usa un solo proveedor fijo por variable de entorno. Hay que armar la
cadena OmniRoute primero (gratis) → Pollinations como alternativa (pago,
ya auto-gateada por experimental flag y API key dentro de su propio
provider — verificado en código, no asumir sin confirmar si pasó tiempo)
→ nunca Mock salvo forzado explícito. El gate que SÍ falta agregar es el
de presupuesto diario, que hoy solo se evalúa al encolar el job con un
proveedor "optimista", no en el momento real de fallback a pago — este es
el punto más importante de todo el ciclo, ver detalle completo antes de
tocar `worker.rs`. Ver `CYCLE-003_LIBRARY_SEPARATION_OMNIROUTE.md` para
el paso a paso completo (5 pasos: adaptador único con 6 archivos
identificados, separar traits, cadena de fallback con re-chequeo de
presupuesto, tests, verificación manual).

### Resultado Grok

Implementado 2026-07-27. Detalle en
`CYCLE-003_LIBRARY_SEPARATION_OMNIROUTE.md` § Resultado Grok.

Resumen:
- `legacy_adapter` único import de `pipeline::visual::library` en producto
  `visual_library/` (+ test de arquitectura).
- Traits `LibraryIngestion` / `LibraryQuery`; search command bound a Query.
- Cadena OmniRoute → Pollinations con re-chequeo de presupuesto en fallback
  (`omniroute_failed_budget_exceeded`); job guarda provider real.
- Tests: 3 de fallback/budget + chain default + architecture.
- `cargo test --lib`: 110 passed; clippy -D warnings ok; fmt ok.
- AGENTS.md §7 actualizado.
- Commit: `5562326`.

### Corrección de Codex tras verificación

- Fecha: 2026-07-28.
- Ángulos: dinero/seguridad, adversarial, evidencia real y alcance.
- Hallazgo crítico: `daily_feed.rs:292` encola con `opportunistic=true`;
  `worker.rs:319` entrega solo ese booleano a la cadena y
  `worker.rs:590-618` permite un candidato pago si el cost gate lo autoriza.
  `cost.rs:21-37` permite pago con oportunista, pagos y presupuesto activos.
  Si OmniRoute falla, daily feed puede intentar Pollinations, contra AGENTS.md
  §4 y la regla never eligible for daily feed de `.env.example`.
- Los tests de fallback pasan `opportunistic=false`; falta probar que un job
  daily excluye Pollinations aun con todos los demás gates habilitados.
- Resultado: requiere revisión humana por dinero. Ambos schedulers se detuvieron
  en esta edición; solo la persona responsable puede reactivarlos.

### Corrección Grok tras revisión humana (2026-07-28)

Autorizada por la persona responsable (“avanza”). Fix del hallazgo:

- `generate_along_chain` ahora recibe `job_origin` y **prohíbe todo
  proveedor pago** cuando `origin == "daily_feed"` o `opportunistic == true`
  (`daily_feed_paid_forbidden`), aunque paid/budget estén abiertos.
- Test nuevo: `daily_feed_never_attempts_paid_pollinations_fallback`
  (Pollinations call count = 0 con gates pagos abiertos).
- Tests previos de fallback video (`opportunistic=false`) siguen pasando.
- Schedulers reactivados por la persona responsable tras el fix.
- Detalle: `CYCLE-003_LIBRARY_SEPARATION_OMNIROUTE.md` § Resultado Grok.

---

## CYCLE-004

- Rol: Product Manager / Frontend
- Estado: RESUELTO POR GROK
- Base HEAD: d86cf21f
- Fecha: 2026-07-28
- Prioridad: alta
- cycle_id: CYCLE-004
- Detalle completo: `docs/reviews/CYCLE-004_REGENERATE_AFTER_REJECT.md`

### Instrucciones para Grok

Restaurar PM-004: una imagen pendiente solo ofrece Aprobar y Rechazar;
editar/regenerar aparece después de confirmar el rechazo. La regresión fue
introducida por `d45ede9`, después de CYCLE-001.

### Resultado Grok

Implementado 2026-07-28. Ver `CYCLE-004_REGENERATE_AFTER_REJECT.md`.

- `ReviewInbox.svelte`: estado inicial solo Aprobar/Rechazar; post-rechazo
  ofrece Generar otra / Editar y regenerar / Ahora no.
- `regenerateCandidate` ya no auto-rechaza (el rechazo ya ocurrió).
- `npm run check`: 0 errors.

---

## Registro de revisiones Codex

### Corrida 2026-07-28

- Revisados: CYCLE-000, CYCLE-001, CYCLE-002 y CYCLE-003.
- Verificados: CYCLE-000 (solo artefactos versionados), CYCLE-001 y CYCLE-002.
- Mejora separada: CYCLE-004 por la regresión posterior a `d45ede9`.
- Seguridad: CYCLE-003 requiere revisión humana; ambos schedulers quedaron
  `DETENIDO`.
- Pruebas: `npm run check` 0 errores/1 warning preexistente; fmt y clippy ok;
  suite visual falló en sandbox por acceso denegado y, repetida fuera del
  sandbox en serial, terminó 49 passed, 0 failed.
- Git: inspeccionados `f3ab841`, `410725b`, `5562326` y `d45ede9`. Sin
  push, force ni escritura de credenciales.

### Post-humano 2026-07-28 (Grok)

- CYCLE-003: fix daily/opportunistic ban de pago + test; estado
  `RESUELTO POR GROK` de nuevo.
- CYCLE-004: PM-004 restaurado en ReviewInbox; `RESUELTO POR GROK`.
- Schedulers: ACTIVO (persona responsable).
