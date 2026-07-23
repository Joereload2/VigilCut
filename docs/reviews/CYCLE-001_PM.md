# CYCLE-001 — Product Manager

- Estado: PENDIENTE
- Base HEAD: 9ef5b2e42cd34ba384c2b6312721dc0dd92c0d53
- Fecha: 2026-07-23
- Prioridad: alta
- cycle_id: CYCLE-001

## Objetivo

Validar que la unificación de Visuales reduzca la confusión entre B-roll, Biblioteca y revisión. La estructura de tres vistas es correcta, pero el producto todavía presenta experiencias distintas según haya video y pierde contexto en el flujo principal de búsqueda.

## Conservar

- Un único modo superior `Visuales`.
- Vistas `Este video`, `Biblioteca` y `Por revisar`.
- Biblioteca y revisión sin video.
- Una acción principal por estado.
- Picker común para buscar, importar, generar u omitir.
- Daily en menú avanzado y candidatos daily en Por revisar.
- Generación enqueue-only sin worker ejecutado desde Svelte.
- Mock identificado como simulación.

## Instrucciones para Grok

### PM-001 — Dar espacio coherente a cada vista

Sin video, `VisualWorkspace` ocupa la pantalla. Con video, Biblioteca y Por revisar se comprimen en el 30% derecho de `VisualPanel.svelte`; `LibraryView` divide nuevamente ese ancho entre grid e inspector.

Mantener editor video/timeline para `Este video`, pero permitir que `Biblioteca` y `Por revisar` usen el ancho completo incluso con video:

1. Este video: preview/timeline + escenas.
2. Biblioteca: workspace ancho; referencia compacta al video, sin split 70/30.
3. Por revisar: workspace ancho para evaluar previews.
4. Cambiar vista no pierde video, plan, selección ni playhead.

No resolver reduciendo fuentes o contenido.

Aceptación:

- A 1280×720, cards ≥170 px e inspector ≥280 px.
- Por revisar permite evaluar calidad y contexto.
- Biblioteca mantiene la misma estructura con y sin video.
- Volver a Este video conserva plan, selección y playhead.
- No aparecen dos headers completos de Visuales.

### PM-002 — Preservar la escena al buscar

El buscador cambia a Biblioteca (`VisualWorkspace.svelte:104-110`), pero `sceneLabel` siempre es null (`59-65`) y Library recibe null (`723-741`). Se pierde la intención “buscar para esta escena”.

Crear un contexto explícito iniciado desde `Buscar imagen` o `Cambiar`. Al expandir resultados a Biblioteca, mostrar:

> Eligiendo imagen para 01:24 — Persona pagando con tarjeta  [Cancelar]

Las cards muestran `Usar en 01:24`. El contexto termina al usar, cancelar o salir explícitamente. No elegir automáticamente la primera necesidad uncovered.

Aceptación:

- Se conservan needId, tiempo y label.
- Cada resultado puede asignarse a esa escena.
- Cancelar no modifica el plan.
- Búsqueda sin escena no muestra placement CTA.
- Cambiar tabs no asigna assets.

### PM-003 — Una sola escritura de placement

`useAssetOnNeed` asigna el asset, crea placement y luego llama `visualApplyNeedsToPlan` (`VisualWorkspace.svelte:261-288`). Una acción debe producir exactamente una colocación.

Definir una única operación `Usar esta imagen` que asigne, cree o reemplace un placement, persista y devuelva el plan. Usar un solo orquestador backend; no encadenar mecanismos con catch silencioso.

Aceptación:

- Un click crea exactamente un placement.
- Repetir no duplica.
- Cambiar conserva tiempos y reemplaza asset.
- Fallo de persistencia no anuncia éxito.
- Test automatizado afirma conteo antes/después.

### PM-004 — Regenerar solo después de rechazar

`ReviewInbox.svelte` muestra Aprobar, Rechazar y Generar otra simultáneamente.

Cambiar a:

1. Estado inicial: Aprobar y Rechazar.
2. Tras confirmar rechazo: `¿Quieres generar otra versión?`.
3. Acciones: Generar otra / Ahora no.
4. Persistir el motivo; no regenerar automáticamente.

Aceptación:

- Generar otra no aparece antes del rechazo.
- Ahora no deja candidato rechazado y escena disponible.
- Generar otra crea intento nuevo.
- Daily usa el mismo flujo sin placement.

### PM-005 — Importar no implica colocar

Importar desde VisualPicker llama inmediatamente `useAssetOnNeed` (`VisualWorkspace.svelte:338-358`).

Después de importar dentro del picker, mostrar el asset seleccionado con `Usar esta imagen`. No crear placement hasta confirmar. Fuera del picker, abrir el asset en Biblioteca.

Aceptación:

- Cerrar tras importar deja asset en Biblioteca y plan intacto.
- Confirmar Usar crea placement.
- La misma imagen no se duplica.

## Orden de implementación

1. PM-003: operación única de asignación/placement.
2. PM-002: contexto explícito de escena.
3. PM-001: layout por vista sin duplicar estado.
4. PM-005: importación contextual confirmada.
5. PM-004: rechazo progresivo.
6. Gates y smoke de los flujos siguientes.

## Flujos obligatorios

### A — Video y Biblioteca

Abrir video → Visuales → escena sin imagen → Buscar → expandir Biblioteca → confirmar barra de contexto → usar asset → comprobar un placement → cambiar vistas sin perder estado.

### B — Importación cancelada

Abrir picker → importar → cerrar sin Usar → asset existe en Biblioteca → plan no cambia.

### C — Rechazo

Por revisar → Generar otra inicialmente ausente → rechazar con motivo → Ahora no; repetir y elegir Generar otra → nuevo intento sin placement automático.

## Pruebas ejecutadas por Codex

- `git status --short --branch`: rama correcta; `pr-comments.json` y `pr-reviews.json` no rastreados, preservados.
- `npm.cmd run check`: exit 0, 0 errores; 1 warning preexistente en `ExportSuccess.svelte:50`.
- `npm.cmd run build`: primer intento bloqueado por sandbox; repetido fuera del sandbox, exit 0, 151 módulos.
- `git diff --check`: exit 0 antes de añadir este ciclo.

## Resultado requerido de Grok

Cambiar `Estado: PENDIENTE` a `Estado: RESUELTO POR GROK` y añadir commit, archivos modificados, decisiones PM-001–PM-005, pruebas reales, evidencia A/B/C y puntos no resueltos.
