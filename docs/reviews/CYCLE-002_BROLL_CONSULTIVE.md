# CYCLE-002 — B-roll debe ser solo consultivo respecto de la Biblioteca

- Rol: Product Manager / Arquitectura
- Estado: RESUELTO POR GROK
- Base HEAD: ab1e51932d5cd50c6d637b1d01086ab9d898e2af
- Fecha: 2026-07-27
- Prioridad: alta
- cycle_id: CYCLE-002

## Problema

Antes de la Biblioteca independiente, B-roll solo dejaba buscar y colocar
imágenes. Hoy, al trabajar B-roll dentro de un video, el usuario puede
importar imágenes sueltas, importar carpetas completas y disparar generación
automática — exactamente las operaciones que la Biblioteca debe poseer en
exclusiva. El requisito de producto es:

> La Biblioteca para B-roll debe ser SOLO consultiva. Si la imagen existe, se
> agrega. B-roll no puede agregar imágenes nuevas a la Biblioteca.

## Causa raíz (root cause, no síntoma)

`VisualWorkspace.svelte` ya tiene un prop `brollOnly` que restringe
navegación (fuerza la vista a `"video"`, oculta la pestaña "Por revisar").
Pero:

1. **`VisualPanel.svelte`** — el único componente que renderiza
   `VisualWorkspace` mientras se edita un video (o sea, el B-roll real) —
   **nunca pasa `brollOnly`**. Solo pasa `compact`, `projectKey`, `view` y
   callbacks. Resultado: `brollOnly` siempre vale `false` en el flujo de
   B-roll real, y el usuario ve el modo "Visuales" completo con las 3
   pestañas (Este video / Biblioteca / Por revisar).
2. Aun si se corrigiera (1), el header de `VisualWorkspace.svelte` con los
   controles de escritura **no respeta `brollOnly`**:
   - Botón "Importar" (línea ~628) → `importImage()`, sin gate.
   - Botón "Importar carpeta" → `importFolder()`, sin gate.
   - "Biblioteca automática…" en el menú `⋯` → abre `dailyOpen`
     (generación), sin gate.
   - "Buscar archivos ausentes" en el mismo menú → operación de
     mantenimiento de la Biblioteca, sin gate.
3. `importImage()` (línea ~377) tiene una rama específica para cuando el
   picker de B-roll está abierto: en vez de bloquear el import, ofrece
   "úsala directo en la escena". Es la confirmación de que el
   comportamiento actual fue una decisión de UX previa, no un bug — y es
   la que hay que revertir.
4. El modal de consulta real, `VisualPicker.svelte` (el que se abre por
   escena, con "Coincidencias de la Biblioteca" y "Usar esta imagen"), **ya
   es correctamente consultivo**. No expone import ni generación. No
   requiere cambios.

Backend: `visual_import_image` / `visual_import_folder`
(`src-tauri/src/commands/visual.rs`) son comandos genéricos sin noción de
"modo caller". Están bien así — los sigue necesitando la Biblioteca. **No
tocar Rust.** El control de "solo consulta" debe vivir enteramente en qué
controles se renderizan según el modo, en el frontend.

No hay ningún test que cubra `brollOnly` hoy (`grep` de `brollOnly` en
`src-tauri/tests/**` y specs de frontend: cero resultados).

## Alcance de esta fase

Dentro:
- Activar `brollOnly` de verdad en el flujo de edición de video.
- Ocultar en modo `brollOnly` todo control que escriba en la Biblioteca:
  importar imagen, importar carpeta, biblioteca automática (generación),
  buscar archivos ausentes.
- Un test (Svelte o smoke) que falle si algún control de escritura queda
  visible en modo `brollOnly`.

Fuera de alcance (no tocar en este ciclo):
- Comandos Rust de import/generación (siguen siendo genéricos, correcto).
- `VisualPicker.svelte` (ya es consultivo, no requiere cambios).
- Lógica de matching/ranking de assets.
- Cualquier cambio de esquema SQLite o Supabase.

## Paso a paso para Grok

### Paso 1 — Activar `brollOnly` en el flujo real de B-roll

Archivo: `src/lib/components/VisualPanel.svelte` (~línea 560, invocación de
`<VisualWorkspace ... />`).

Agregar el prop `brollOnly` a la invocación. Debe ser `true` cuando el
panel se usa en el contexto de edición de un video existente (es decir,
siempre que `VisualPanel` esté montado con un video abierto — confirmar
la condición exacta leyendo cómo `VisualPanel` decide que hay video, y
usar esa misma condición; no inventar una nueva señal de estado).

Criterio de aceptación: al abrir un video y entrar a "Visuales", la vista
queda fija en "Este video" y no aparece la pestaña "Por revisar" (esto ya
lo hace el componente internamente una vez `brollOnly=true` llega bien).

### Paso 2 — Ocultar controles de escritura en el header cuando `brollOnly`

Archivo: `src/lib/components/visual/VisualWorkspace.svelte`.

Envolver con `{#if !brollOnly}`:
- El botón "Importar" (`onclick={() => void importImage()}`).
- El botón "Importar carpeta" (`onclick={() => void importFolder()}`).

Dentro del menú `⋯` (`menuOpen`), envolver con `{#if !brollOnly}`:
- El botón "Biblioteca automática…" (abre `dailyOpen`).
- El botón "Buscar archivos ausentes" (`scanMissing()`).

No tocar "Detectar momentos" — es análisis del video actual, no escribe en
la Biblioteca, debe seguir visible en B-roll.

Criterio de aceptación: en modo `brollOnly=true`, el header solo muestra
el campo de búsqueda y el botón `⋯` reducido a las opciones que no
escriben en la Biblioteca (o el botón `⋯` desaparece si queda vacío —
decisión de Grok, priorizar simplicidad visual).

### Paso 3 — Cerrar el atajo de `importImage()` para contexto de picker

Archivo: `src/lib/components/visual/VisualWorkspace.svelte`, función
`importImage()` (~línea 377).

Como el botón que la dispara ya no existe en modo `brollOnly` (Paso 2), la
rama `if (pickerOpen && (pickerNeed || sceneContext))` queda inalcanzable
en ese modo. Dejarla intacta para el modo Biblioteca/Visuales general
(donde sigue siendo válida). No requiere cambio de código si el Paso 2 se
implementó bien — usar este paso solo como verificación, no como edición
adicional.

### Paso 4 — Test de regresión

Agregar un test (Svelte component test si el proyecto ya tiene ese
tooling, o smoke test equivalente) que:
1. Monte `VisualWorkspace` con `brollOnly=true`.
2. Verifique que no existen en el DOM controles con las acciones de
   importar imagen, importar carpeta, biblioteca automática o buscar
   archivos ausentes.
3. Verifique que "Coincidencias de la Biblioteca" (vía `VisualPicker`)
   sigue funcionando y permite "Usar esta imagen".

Si el repo no tiene aún test runner de componentes Svelte configurado, no
introducir uno nuevo en este ciclo — dejar constancia explícita en el
"Resultado Grok" de por qué se omitió, y cubrir el criterio con un smoke
test manual documentado paso a paso en su lugar.

### Paso 5 — Verificación manual antes de marcar RESUELTO

- Abrir un video, ir a B-roll/Visuales: confirmar que no aparecen
  "Importar", "Importar carpeta" ni "Biblioteca automática…".
- Abrir el picker de una escena sin imagen: confirmar que solo se puede
  buscar y usar assets existentes, nunca crear uno nuevo desde ahí.
- Ir a la Biblioteca standalone (`LibraryWorkspace.svelte`, `libraryOnly`):
  confirmar que ahí SÍ siguen disponibles importar, importar carpeta y
  biblioteca automática — este ciclo no debe restringir la Biblioteca en
  sí, solo el consumo desde B-roll.
- Correr `npm run check` (0 errores) y la suite Rust existente
  (`cargo test`) para confirmar que no se rompió nada fuera del frontend.

## Resultado Grok

Implementado 2026-07-27 sobre base `ab1e519`.

### Cambios

1. **Paso 1 — `VisualPanel.svelte`:** pasa `brollOnly={true}` a
   `VisualWorkspace`. Condición confirmada en `App.svelte`: `VisualPanel`
   solo se monta cuando hay `projectStore.mediaPath` (modo visual con
   video abierto); la Biblioteca standalone usa `LibraryWorkspace` con
   `libraryOnly`, no este panel. No se inventó una señal nueva.
2. **Paso 2 — `VisualWorkspace.svelte`:**
   - Botones "Importar" / "Importar carpeta" envueltos en `{#if !brollOnly}`.
   - Menú `⋯`: "Biblioteca automática…" y "Buscar archivos ausentes"
     envueltos en `{#if !brollOnly}`. "Detectar momentos" sigue visible
     cuando hay video. Si `brollOnly` y no hay items de menú útiles, el
     botón `⋯` solo aparece si `hasVideo` (para Detectar momentos).
   - Panel `DailySettings` no se renderiza cuando `brollOnly`.
3. **Paso 3 — `importImage()`:** sin cambios de código. La rama
   `pickerOpen && (pickerNeed || sceneContext)` queda inalcanzable en
   `brollOnly` porque el botón Importar no se renderiza. Intacta para
   modo Biblioteca/Visuales general.
4. **Paso 4 — test de regresión:** el repo **no** tiene runner de
   componentes Svelte (ni vitest/jest en `package.json`; `test:unit` es
   `cargo test --lib`). No se introdujo tooling nuevo (alcance del ciclo).
   Cobertura del criterio con smoke manual abajo.
5. **Hallazgo vs plan (asunción que no se sostenía):** el plan decía que
   con `brollOnly=true` el componente "ya" fija la vista en "Este video".
   En el código real, el `$effect` de vista hacía
   `(libraryOnly || brollOnly || !hasVideo) && view === "video" → view = "library"`,
   lo que **expulsaba** de `"video"` cuando `brollOnly` era true. Corregido
   en el mismo ciclo (dentro del alcance de "activar brollOnly de verdad"):
   `libraryOnly` fuerza library; `brollOnly` fuerza video; sin media y sin
   brollOnly se sale de video.

### Smoke manual (Paso 5)

| Paso | Acción | Esperado |
|------|--------|----------|
| A | Abrir video → modo Visual/B-roll | Header "B-roll"; sin pestaña "Por revisar" / "Biblioteca"; sin "Importar" / "Importar carpeta"; sin "Biblioteca automática…" ni "Buscar archivos ausentes" en `⋯` |
| B | Abrir picker de escena sin imagen | Solo buscar y "Usar esta imagen"; sin import |
| C | Modo Biblioteca standalone (`libraryOnly`) | Siguen "Importar", "Importar carpeta", "Biblioteca automática…" |

### Verificación comandos (AGENTS.md §3)

- `npm run check`: **0 errors** (1 warning a11y preexistente en
  `ExportSuccess.svelte`, no introducido por este ciclo).
- `npm run test:unit` (`cargo test --lib`): **105 passed; 0 failed**
  (sin cambios de backend en este ciclo; smoke de no-regresión).
- No se tocó Sección 4 (dinero) ni Sección 5 (secretos).

### Archivos tocados

- `src/lib/components/VisualPanel.svelte`
- `src/lib/components/visual/VisualWorkspace.svelte`
- `docs/reviews/CYCLE-002_BROLL_CONSULTIVE.md`
- `docs/reviews/CODEX_TO_GROK.md`
- `AGENTS.md` (añadido a la raíz del repo; estaba solo en el paquete de
  handoff del escritorio — la tarea lo asumía en raíz y no existía)

### Commits

Ver `git log` tras el commit de este ciclo (mensaje prefijo
`fix(ui): broll consultive only — CYCLE-002`).
