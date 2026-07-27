# Especificación UX unificada — Visuales

| Campo | Valor |
|-------|--------|
| **Estado** | Spec de producto + implementación (fuente de verdad UI visual) |
| **Versión** | 1.0 · 2026-07-23 |
| **Rama** | `feat/intelligent-clipping` |
| **Supersede** | `docs/VISUAL_LIBRARY_UI_SPEC.md` en lo que proponga un **4.º modo “Biblioteca”** separado. Ese enfoque **queda cancelado**. |
| **Relacionados** | `docs/visual-library/supervision-ui.md`, `docs/VISUAL_LIBRARY_DESIGN.md`, `docs/VISUAL_LAYOUT_CONTRACT.md` |
| **Backend** | Reutilizar commands Tauri existentes; supervisor Rust (no `worker_tick` desde UI) |

---

## 0. Instrucción principal para Grok

**No** construir una Biblioteca como producto aparte y complejo.

Unificar Biblioteca, B-roll, imágenes generadas y necesidades del video en **un solo espacio superior: Visuales**.

El usuario **no** debe aprender:

| Dominio (sigue en Rust/SQLite) | UI (no usar estos nombres) |
|-------------------------------|----------------------------|
| `MediaAsset` | imagen / asset de la biblioteca |
| `VisualConcept` | tema / etiqueta de significado |
| `VisualNeed` | **escena** o **momento visual** |
| Candidate / Job | revisión / en cola / generando |
| Placement | “en el video” / “en 01:24” |

### Las tres únicas preguntas de la UI

1. ¿Qué imágenes **necesita este video**?  
2. ¿Qué imágenes **tengo disponibles**?  
3. ¿Qué imágenes **necesitan mi revisión**?

Todo lo demás es detalle, menú `…` o “Detalles técnicos”.

---

## 1. Decisión de producto

### 1.1 Un solo modo superior

| Hoy (retirar como modelo mental) | Mañana |
|----------------------------------|--------|
| Visual / B-roll + 7 sub-tabs (Colocar, Imágenes IA, Biblioteca…) | **Visuales** |
| 4.º modo global “Biblioteca” (spec antigua) | **No**. Biblioteca = vista **dentro** de Visuales |

Modos superiores finales:

```text
Silencios | Shorts 9:16 | Visuales
```

```ts
export type WorkspaceMode = "silence" | "clips" | "visual";
// "visual" se etiqueta en UI como "Visuales" (no "Visual / B-roll")
```

### 1.2 Tres vistas internas (no más)

```text
[ Este video ]  [ Biblioteca ]  [ Por revisar (N) ]
```

| Vista | Pregunta | ¿Requiere video? |
|-------|----------|------------------|
| **Este video** | ¿Qué necesita este video? | Sí (si no hay video → deshabilitada) |
| **Biblioteca** | ¿Qué tengo? | No |
| **Por revisar** | ¿Qué debo aprobar? | No |

**Prohibido** como tabs principales del panel derecho: Colocar, Propiedades, Excepciones, Imágenes IA, Texto, Config, “daily” suelto.  
Esas capacidades se **reparten** en las 3 vistas + timeline/preview (ver §8).

---

## 2. Modelo mental

```text
Encontrar o crear → Revisar si hace falta → Guardar en Biblioteca → Usar en el video
```

- **Biblioteca** = repositorio común (importadas + generadas aprobadas + daily aprobadas).  
- **B-roll / placement** = *uso* de un asset en el timeline del video actual, no otro almacén.  
- Origen (import / IA / mock / automática) = **metadata**, no flujo distinto.

---

## 3. Regla de simplificación

### Visible siempre (acciones frecuentes)

- Buscar  
- Importar  
- Generar **solo** cuando no hay coincidencia (o tras rechazo)  
- Aprobar / Rechazar  
- Usar en el video  
- Cambiar / quitar del video  

### Solo en “Detalles técnicos” (colapsado)

- Provider / modelo  
- Prompt completo / negative  
- IDs, hashes, idempotency  
- Stage interno del job  
- Scores numéricos exactos  
- Ruta administrada, provenance JSON  

### Lenguaje

| Usar | Evitar |
|------|--------|
| Visuales, Este video, Biblioteca, Por revisar | Need, Candidate, Worker, Job, Tick |
| escena / momento visual | coverage, apply match |
| Buscar imagen, Generar una nueva | free_configured, probe |
| Usar esta imagen, Guardar en Biblioteca | Daily feed (como nombre de pestaña) |
| Revisión pendiente, En cola, Generando… | Idempotency, SSRF, lease |

Mock: siempre **“Simulación (mock) — no es IA”**, nunca “IA local”.

---

## 4. Navegación

### 4.1 Sin video

- Modo **Visuales** accesible desde TopBar **sin** exigir archivo.  
- Vistas activas: **Biblioteca**, **Por revisar**.  
- **Este video** deshabilitada:

  > Abre un video para ver qué imágenes necesita.

- No abrir el file picker solo por entrar a Visuales.

### 4.2 Con video

- Vista por defecto: **Este video**.  
- Orden de tabs: Este video → Biblioteca → Por revisar (badge si `N > 0`).  
- Última subvista solo en memoria de sesión (no SQLite).

### 4.3 Layout global (con video)

```text
┌─ TopBar: Silencios | Shorts | Visuales ─────────────────────────┐
├─ Preview 70% ──────────────────────┬─ Shell Visuales 30% ───────┤
│  video + overlays placement        │  header + search + Import  │
│  timeline / track B-roll           │  [Este video|Biblio|Revisar]│
│                                    │  contenido de la vista     │
└────────────────────────────────────┴────────────────────────────┘
```

Sin video: shell Visuales puede ocupar el área principal (como Welcome, pero no reemplazar forever: el usuario puede volver a Silencios).

### 4.4 Orden de render en `App.svelte` (recomendado)

```svelte
{#if workspaceTab === "visual"}
  <VisualWorkspace />
{:else if !projectStore.mediaPath}
  <Welcome />
{:else}
  <!-- silence / clips -->
{/if}
```

`VisualWorkspace` decide internamente si hay `mediaPath` para habilitar “Este video”.

---

## 5. Shell común (`VisualWorkspace`)

```text
Visuales
Encuentra, revisa y usa imágenes sin salir de tu proyecto.

[ 🔍 Buscar imágenes…              ]  [ Importar ▾ ]

[ Este video ] [ Biblioteca ] [ Por revisar 3 ]
```

| Control | Comportamiento |
|---------|----------------|
| Buscador | Un solo campo. Por defecto busca en **Biblioteca**. Si hay escena seleccionada en “Este video”, los resultados del picker se anclan a esa escena. |
| Importar | Menú: **Importar imagen** / **Importar carpeta**. Un solo sitio. |
| Menú `…` del shell | Biblioteca automática (daily settings), Detectar nuevamente (si hay video), Buscar archivos ausentes. |

**No** duplicar Importar en cada vista.  
**No** botón global “Generar imágenes” en el header.

---

## 6. Vista 1 — Este video

### 6.1 Resumen

Una frase + barra de progreso simple:

> **6 de 8** momentos visuales están cubiertos.

Secundario: `Detectar nuevamente` (equivale a `visual_detect_needs` + merge no destructivo).

**No** seis chips técnicos (reused / generated / waiting / …) en la primera pantalla.

### 6.2 Lista de momentos

Cada fila = una `VisualNeed` en dominio, **“escena”** en UI.

Mostrar:

- Rango de tiempo `01:24–01:31`  
- Descripción humana (`label`)  
- Thumbnail si cubierta  
- Estado corto (1 frase)  
- **Una** acción principal  

| Estado dominio (aprox.) | Texto UI | Acción principal |
|-------------------------|----------|------------------|
| uncovered / skipped | Sin imagen | **Buscar imagen** |
| matched / covered | Lista | **Cambiar** |
| queued | En cola | **Cancelar** |
| running / processing | Generando… | **Cancelar** |
| cancelling | Cancelando… | (ninguna) |
| needs_human_review | Revisión pendiente | **Revisar** (salta a Por revisar filtrado) |
| failed | No se pudo generar | **Reintentar** |
| rejected / cancelled | Sin imagen (antes falló) | **Buscar imagen** |

Regla: **nunca** mostrar a la vez Buscar + Generar + Continuar sin imagen + Regenerar en la misma fila.

### 6.3 Selector único `Buscar imagen` → `VisualPicker`

Drawer / panel modal (no página nueva):

```text
Buscar imagen para 01:24–01:31 · “persona comprando”

1) Coincidencias de la Biblioteca   [auto al abrir]
   [card] [card] …  → Usar esta imagen

2) Importar imagen                 → dialog archivo

3) Generar una nueva               → enqueue-only

… Más opciones
   · Dejar esta escena sin imagen
```

Flujo feliz:

```text
Buscar imagen → ver coincidencias → Usar esta imagen
```

Si no hay match:

```text
Buscar imagen → Generar una nueva → (badge Por revisar) → Aprobar → Usar ahora / Solo biblioteca
```

- **Cambiar** abre el mismo picker; la imagen actual permanece hasta confirmar otra.  
- Generar: IPC enqueue-only; fila pasa a En cola; **sin** `worker_tick` en UI.  
- Continuar sin imagen solo en menú secundario del picker.

### 6.4 Colocar en timeline / props / excepciones

No son tabs del shell Visuales:

| Capacidad actual | Dónde vive en UX unificada |
|------------------|----------------------------|
| Colocar manual | Timeline: “+ Imagen” o picker con intervalo del playhead |
| Propiedades del placement | Al seleccionar bloque en timeline → inspector bajo preview o drawer “En el video” |
| Excepciones composición | Badge en track + lista corta bajo preview, no tab principal |
| Texto / Whisper | Menú del modo o subárea del preview (fuera del triple-tab de assets) |
| Config visual | Menú `…` del shell |

MVP de migración: si hace falta conservar Colocar temporalmente, debe ser **acción** dentro de “Este video” (“Colocar en el playhead”), no tab de igual rango que Biblioteca.

---

## 7. Vista 2 — Biblioteca

### 7.1 Objetivo

Encontrar una imagen existente y decidir reutilizarla.  
**No** jobs, candidates ni daily feed aquí.

### 7.2 Controles

Filtros visibles (≤ 5):

- Todas  
- Horizontal  
- Vertical  
- IA  
- Importadas  

`Más filtros`: estado (activa/archivada/ausente), licencia, orden (recientes / usadas / título).

### 7.3 Card

- Imagen (`convertFileSrc`, lazy)  
- Título  
- Máx. 2 tags/conceptos  
- Badge **solo** si hay advertencia: licencia desconocida, ausente, mock, bloqueada  

**No** amontonar licencia + QA + provider + MIME + uso en la card.

Acción contextual:

- Con escena seleccionada: **Usar en 01:24**  
- Sin escena: click → inspector  

### 7.4 Inspector

Básico: preview, título, tags/conceptos, licencia en lenguaje humano, veces usada, Usar/Cambiar.

Colapsable **Detalles técnicos**: origen, provider/model, QA, prompt, dimensiones, archivo.

Menú `…`: editar metadatos, archivar/restaurar, bloquear. **Sin** borrado físico.

Commands: `visual_list_assets`, `visual_import_*`, `visual_update_asset`, `visual_list_usage`, `visual_scan_missing`.

---

## 8. Vista 3 — Por revisar

### 8.1 Una sola bandeja

Incluye:

- Candidatos del video (`origin=video_need`)  
- Candidatos de alimentación automática (`origin=daily_feed`)  
- Cualquier `needs_human_review`  

**No** panel daily colgado al final de otro tab.

### 8.2 Agrupación (encabezados, no sub-tabs)

```text
Para este video
  [card] [card]

Para la Biblioteca
  [card] [card]   ← daily y generadas sin placement
```

### 8.3 Card de revisión

- Preview  
- Concepto / escena en lenguaje humano  
- Contexto requerido / prohibido (corto)  
- Origen: “Este video” | “Biblioteca automática”  
- Badge mock / coste no verificado / licencia  

### 8.4 Acciones

Principales:

- **Aprobar**  
- **Rechazar**  

Secundaria (después o en menú): **Generar otra**.

**No** mostrar siempre “Aprobar y usar” + “Solo biblioteca” + “Rechazar” + “Regenerar” a la vez.

Tras aprobar candidato **de video**:

```text
¿Usar ahora en 01:24?     [Usar ahora]  [Solo guardar en Biblioteca]
```

Tras aprobar candidato **daily / biblioteca**:

- Solo guardar en Biblioteca.  
- **Nunca** placement directo.

Tras rechazar:

1. Motivos rápidos (chips).  
2. Luego: “¿Generar otra?”  

Cancelación: mismos textos que en Este video (En cola → Cancelando… → Cancelada solo al confirmar backend).

Commands: `visual_supervision` / `visual_supervision_global`, `visual_approve_*`, `visual_reject_*`, `visual_regenerate_need`, `visual_cancel_job`.

---

## 9. Importación unificada

Un botón **Importar** en el shell:

1. Importar imagen → `visual_import_image`  
2. Importar carpeta → `visual_import_folder` (+ checkbox subcarpetas)  

Después:

| Contexto | Siguiente paso |
|----------|----------------|
| Cubriendo una escena (picker abierto) | Preguntar: ¿Usar en esta escena? |
| Solo en Biblioteca | Seleccionar asset en inspector |
| Sin video | Abrir en Biblioteca |

**Nunca** crear placement sin confirmación explícita.

Web (`!isTauri`): Importar deshabilitado, mensaje “Disponible en la app de escritorio”.

---

## 10. Generación unificada

- Solo desde **picker de escena** o “Generar otra” tras rechazo.  
- **No** CTA global en Biblioteca.  
- Enqueue-only (`visual_generate_need` / regenerate); supervisor procesa.  
- UI: En cola en la fila; badge Por revisar al terminar; **sin** modal forzado.  
- Poll de **snapshot** (supervisión), nunca `visual_worker_tick` en producto.

---

## 11. Alimentación automática (daily)

Ubicación: `Visuales → menú … → Biblioteca automática` (`DailySettings`).

Contenido:

- Switch activar/desactivar  
- Texto: solo simulación mock o gratis **verificado**  
- Último ciclo / intervalo  
- Resumen semanal opcional  

Resultados: **solo** en Por revisar → “Para la Biblioteca”.  
**No** acordeón permanente bajo el video.

---

## 12. Wireframes (ASCII) — entregar / validar antes de code completo

### 12.1 Este video

```text
┌ Visuales ──────────────────────────────────────────────┐
│ [buscar…] [Importar]                    [ … ]          │
│ [ Este video ] [ Biblioteca ] [ Por revisar 2 ]        │
├────────────────────────────────────────────────────────┤
│ 6 de 8 momentos cubiertos          [Detectar de nuevo] │
│ ████████████░░░░                                       │
│                                                        │
│ 01:24–01:31  Persona en el supermercado                │
│ [thumb] Lista                              [ Cambiar ] │
│                                                        │
│ 02:10–02:18  Precios / inflación                       │
│ Sin imagen                          [ Buscar imagen ]  │
│                                                        │
│ 03:00–03:06  Generando…                    [ Cancelar ]│
└────────────────────────────────────────────────────────┘
```

### 12.2 Picker

```text
┌ Buscar imagen · 02:10–02:18 ─────────────────────────┐
│ Coincidencias                                        │
│  [img] Carrito  [ Usar esta imagen ]                 │
│  [img] Tienda   [ Usar esta imagen ]                 │
│ ───────────────────────────────────────────────────  │
│ [ Importar imagen ]                                  │
│ [ Generar una nueva ]                                │
│ … Dejar sin imagen                                   │
└──────────────────────────────────────────────────────┘
```

### 12.3 Biblioteca (sin video)

```text
┌ Visuales ────────────────────────────────────────────┐
│ [buscar…] [Importar]                                 │
│ [ Este video (off) ] [ Biblioteca ] [ Por revisar ]  │
│ Todas | Horiz. | Vert. | IA | Import. | Más filtros  │
│ ┌────┐ ┌────┐ ┌────┐     │ Preview + título          │
│ │    │ │    │ │    │     │ Tags · Licencia           │
│ └────┘ └────┘ └────┘     │ Usada 3 veces             │
│                          │ [Detalles técnicos ▸]     │
└──────────────────────────────────────────────────────┘
```

### 12.4 Por revisar

```text
│ Para este video                                      │
│ [preview] Supermercado · video · mock                │
│          [ Aprobar ] [ Rechazar ]                    │
│ Para la Biblioteca                                   │
│ [preview] Concepto X · automática                    │
│          [ Aprobar ] [ Rechazar ]                    │
```

---

## 13. Mapa dominio → UI

| Acción UI | Command / módulo |
|-----------|------------------|
| Detectar momentos | `visual_detect_needs` |
| Listar momentos | `visual_supervision` / needs list |
| Buscar coincidencias | `visual_search_library_for_need` / `visual_match_need` |
| Encolar generación | `visual_generate_need` (enqueue-only) |
| Cancelar | `visual_cancel_job` |
| Regenerar | `visual_regenerate_need` |
| Aprobar / rechazar | `visual_approve_*`, `visual_reject_*` |
| Colocar en plan | `visual_approve_and_use` (place) / `visual_create_manual_placement` / apply needs |
| Catálogo | `visual_list_assets` |
| Importar | `visual_import_image`, `visual_import_folder` |
| Editar asset | `visual_update_asset` |
| Uso | `visual_list_usage` |
| Daily settings | `visual_daily_feed_*` |
| Snapshot global review | `visual_supervision_global` |

Nuevos commands: **solo** si falta un contrato; justificar en el PR.

---

## 14. Componentes

| Archivo | Rol |
|---------|-----|
| `VisualWorkspace.svelte` | Shell, tabs 3 vistas, search, import, menú `…` |
| `VideoVisualsView.svelte` | Lista de momentos + resumen |
| `VisualPicker.svelte` | Drawer buscar/importar/generar |
| `LibraryView.svelte` | Grid + filtros + selección |
| `ReviewInbox.svelte` | Bandeja única |
| `AssetCard.svelte` | Card compartida |
| `AssetInspector.svelte` | Metadata + detalles |
| `DailySettings.svelte` | Solo settings daily |
| `visualsStore.svelte.ts` o controller | Snapshot, selection, poll lectura |

**No** seguir creciendo `ImageGenerationPanel.svelte`. Plan de retiro al lograr paridad.

### Plan de implementación

| Fase | Entrega |
|------|----------|
| **F0** | Wireframes validados (esta doc §12) + rename tab “Visuales” |
| **F1** | `VisualWorkspace` shell + 3 tabs vacíos; Visuales sin video → Biblioteca |
| **F2** | `ReviewInbox` sobre APIs de supervisión actuales |
| **F3** | `VisualPicker` + `VideoVisualsView` (necesidades) |
| **F4** | `LibraryView` + import unificado |
| **F5** | DailySettings en `…`; quitar acordeón daily |
| **F6** | Migrar Colocar/props a timeline/inspector; retirar tabs viejos e `ImageGenerationPanel` |

No mantener dos UIs completas en producción al final de F6.

---

## 15. Feedback y estados

- Skeleton local en grid/lista; no overlay a pantalla completa por una imagen.  
- Toast / `aria-live` para importar, guardar, errores.  
- Cancelar: **Cancelando…** hasta status terminal backend.  

Errores en lenguaje humano:

- `No pudimos generar la imagen. Reintenta o elige una de la Biblioteca.`  
- `El archivo ya no está disponible. Busca otro o vuelve a importarlo.`  
- `La licencia no está verificada. Revisa los detalles antes de publicar.`  

Stack / códigos → solo Detalles técnicos.

---

## 16. Accesibilidad y diseño

- Tabs con `role="tablist"`; teclado completo.  
- Drawer picker: `role="dialog"`, Escape, foco devuelto.  
- Mock / licencia: texto + color (no solo color).  
- Tokens: `surface`, `sky` (Visuales), `violet` (mock / revisar), `amber` (avisos), `emerald` (ok).  
- Densidad similar al panel actual 30%, más aire en grid Biblioteca.

---

## 17. Eliminar o esconder de la UI actual

| Elemento actual | Destino |
|-----------------|---------|
| Tab “Imágenes IA” | Fusionar en Este video + Por revisar |
| Tab “Biblioteca” aislada + coverage técnica | Vista Biblioteca limpia |
| Botones Buscar en biblioteca + Generar a la vez | Un picker |
| Daily acordeón permanente | Menú `…` + bandeja |
| IDs / prompts por defecto | Detalles técnicos |
| Provider/model en card | Detalles |
| Seis filtros de estado de entrada | Más filtros |
| Continuar sin imagen en cada fila | Menú del picker |
| Aprobar y usar + Solo biblio + Rechazar + Regenerar juntos | Flujo en 2 pasos |
| `worker_tick` / “Actualizar cola” | Eliminar de producto |
| 4.º modo superior Biblioteca | **No implementar** |

---

## 18. Relación con otras specs

| Doc | Rol tras esta decisión |
|------|------------------------|
| **Esta** (`UNIFIED_VISUALS_UX_SPEC`) | **Fuente de verdad UX** del dominio visual |
| `VISUAL_LIBRARY_UI_SPEC.md` | Reutilizable como detalle de **vista Biblioteca** (grid, inspector, import, tipos). **Ignorar** el 4.º modo y shell independiente. |
| `supervision-ui.md` | Contratos de estados job/candidate; mapear a lenguaje humano de esta spec |
| `VISUAL_LAYOUT_CONTRACT.md` | Preview / placement espacial (sigue válido) |

---

## 19. Criterios de aceptación

- [ ] Un solo modo superior **Visuales** (label UI; id `visual` ok).  
- [ ] Solo 3 vistas: Este video, Biblioteca, Por revisar.  
- [ ] Sin video: Biblioteca + Por revisar usables; Este video off.  
- [ ] Este video: una acción principal por estado de escena.  
- [ ] Buscar imagen = picker único (match + import + generar).  
- [ ] Biblioteca = repositorio común; sin jobs en la vista.  
- [ ] Una bandeja Por revisar; daily solo ahí + settings en `…`.  
- [ ] Daily **nunca** ofrece “usar en timeline” al aprobar.  
- [ ] UI principal sin IDs/prompts/jobs; mock etiquetado.  
- [ ] No `worker_tick` desde UI; generar enqueue-only.  
- [ ] Sin duplicar `ImageGenerationPanel` al final.  
- [ ] Teclado + `npm run check` + `npm run build` + `git diff --check`.  

---

## 20. Definition of Done

1. Wireframes §12 revisados (texto o capturas).  
2. Flujos con y sin video validados manualmente.  
3. Implementación por fases F0–F6; paridad antes de borrar lo viejo.  
4. Retiro o reducción a cero de `ImageGenerationPanel` en rutas de usuario.  
5. Commands existentes; nuevos justificados.  
6. Sin mocks de catálogo en frontend.  
7. Sin pagos ni Supabase producción.  
8. Informe corto: archivos, decisiones, pruebas.

---

## 21. Fuera de alcance

- Edición / crop de imagen  
- Borrado físico  
- Gestión masiva  
- Marketplace  
- Sync Supabase  
- Generación libre sin escena/concepto  
- Drag-and-drop avanzado a timeline (MVP puede ser click “Usar en…”)  
- Analytics complejos  

No implementar a medias durante la unificación.

---

## 22. Flujo mínimo de referencia

### Con video

```text
Este video → Buscar imagen → Usar esta imagen
                 └→ Generar → Por revisar → Aprobar → Usar ahora
```

### Sin video

```text
Visuales → Biblioteca (importar/buscar)
        → Por revisar (aprobar a biblioteca)
```

---

## 23. Checklist de migración desde la UI de la captura del usuario

La captura con tabs **Colocar | Propiedades | Excepciones | Biblioteca | Texto | Config** (y a veces Imágenes IA):

| Tab viejo | Acción |
|-----------|--------|
| Colocar | Integrar en Este video / timeline |
| Propiedades | Inspector de placement al seleccionar en timeline |
| Excepciones | Lista compacta bajo preview |
| Biblioteca | Vista **Biblioteca** del shell |
| Imágenes IA | **Este video** + **Por revisar** |
| Texto / Config | Fuera del triple-tab (menú o área preview) |

Objetivo de producto: el usuario **deja de preguntar “dónde está Imágenes IA”** porque no es un destino: **Buscar imagen** y **Por revisar** cubren esa necesidad.
