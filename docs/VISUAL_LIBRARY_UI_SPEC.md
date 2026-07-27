# Especificación UI — Biblioteca Visual de VigilCut

> **Actualización de producto (2026-07-23):** la fuente de verdad UX es  
> **`docs/UNIFIED_VISUALS_UX_SPEC.md`**.  
> **No** implementar un 4.º modo superior “Biblioteca”. La Biblioteca es una  
> **vista interna de Visuales**. Este documento sigue siendo útil como detalle  
> de grid/inspector/import/tipos de la **vista Biblioteca**.

| Campo | Valor |
|-------|--------|
| **Estado** | Parcialmente supersedido — ver UNIFIED_VISUALS_UX_SPEC |
| **Versión** | 1.1 · 2026-07-23 |
| **Rama objetivo** | `feat/intelligent-clipping` |
| **Backend** | Ya disponible (SQLite + Tauri). No requiere worker/OmniRoute/Supabase |
| **Docs relacionadas** | `docs/UNIFIED_VISUALS_UX_SPEC.md` (prioridad), `docs/VISUAL_LIBRARY_DESIGN.md`, `docs/visual-library/*` |

---

## 1. Propósito

Implementar una **pantalla global de catálogo** para explorar, importar, editar metadatos y mantener la Biblioteca Visual Inteligente.

La Biblioteca es un **espacio de aplicación**, no una subsección de un video. Debe abrirse al iniciar VigilCut aunque no existan:

- `mediaPath`
- `projectKey`
- análisis / EDL
- plan visual

### Qué no es esta pantalla

| Superficie | Rol | Esta spec |
|------------|-----|-----------|
| **Biblioteca** (modo global) | Catálogo de assets reutilizables | **Sí** |
| **Visual / B-roll** | Composición sobre un video (placements, timeline) | No |
| **Imágenes IA** (`ImageGenerationPanel`) | Supervisión de necesidades, cola de generación, daily feed | No (panel separado) |

No convertir `ImageGenerationPanel.svelte` en catálogo. No duplicar su lógica aquí. Enlaces opcionales (CTA “Ver cola de generación”) son backlog.

---

## 2. Resultado esperado

### 2.1 Cuarto modo de trabajo

Añadir **Biblioteca** junto a:

| Modo actual | Color tab | Dependencia de video |
|-------------|-----------|----------------------|
| Silencios | `vigil` / verde | Sí (hoy) |
| Shorts 9:16 | `amber` | Sí (hoy) |
| Visual / B-roll | `sky` | Sí (hoy) |
| **Biblioteca** | **`violet` o `emerald`** | **No** |

```ts
export type WorkspaceMode = "silence" | "clips" | "visual" | "library";
```

Usar el alias en `App.svelte`, `TopBar.svelte`, `ModeTabs.svelte` y `ModeNav.svelte` (si aplica).

### 2.2 Contenido de la pantalla

1. Encabezado + métricas reales  
2. Buscador + filtros + orden  
3. Grid de assets  
4. Inspector lateral del asset seleccionado  
5. Importación de imagen y carpeta  
6. Empty state útil  
7. Loading / error / missing  

---

## 3. Restricciones de arquitectura

| Regla | Motivo |
|-------|--------|
| No depender de `projectStore.mediaPath` | Catálogo global |
| No exigir `projectKey` | Sin EDL |
| No llamar `visual_worker_tick` / `visualWorkerTick` | Supervisor Rust; UI solo lectura de catálogo |
| No Supabase | Fuera de alcance (`docs/visual-library/database.md`) |
| No pagos ni generación real en esta UI | Solo import + metadatos |
| Fuente de verdad: SQLite vía Tauri | Sin mocks presentados como producto |
| Diseño dark actual | Tokens `surface`, `vigil`, `brand`, `amber`, `sky`, `violet` |
| No editar migraciones / worker / OmniRoute / render | Salvo bug de contrato aprobado aparte |

---

## 4. Navegación global

### 4.1 Visibilidad de tabs

Hoy los modos solo tienen sentido con proyecto. Nueva regla:

- Tab **Biblioteca**: **siempre visible** (con o sin video).
- Silencios / Shorts / Visual: pueden seguir requiriendo video para su contenido.
- Al abrir Biblioteca sin video: **no** renderizar `Welcome` detrás ni pedir abrir archivo.

### 4.2 Orden de render en `App.svelte`

```svelte
{#if workspaceTab === "library"}
  <VisualLibraryPanel onMessage={showToast} onError={(e) => (projectStore.error = e)} />
{:else if !projectStore.mediaPath}
  <Welcome ... />
{:else}
  <!-- modos de proyecto: silence / clips / visual -->
{/if}
```

### 4.3 Tab Biblioteca

| Propiedad | Valor |
|-----------|--------|
| Etiqueta | `Biblioteca` |
| `role="tab"` | sí |
| `aria-selected` | según modo |
| Nombre accesible | `Abrir Biblioteca Visual` |
| Color activo | `bg-violet-600 text-white` (recomendado) |
| Atajo opcional (backlog) | `Ctrl+4` / `Cmd+4` |

### 4.4 TopBar

- Incluir el tab aunque `!mediaPath`.
- Si el resto de la barra asume video, ocultar acciones de export/analizar solo en modos de proyecto; en Biblioteca mostrar al menos el logo y el switcher de modos.

---

## 5. Layout

### 5.1 Wireframe desktop (≥ 1100 px)

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ TopBar  [Silencios] [Shorts] [Visual] [Biblioteca]          …            │
├──────────────────────────────────────────────────────────────────────────┤
│ Biblioteca visual                          [Importar imagen] [Carpeta ▾] │
│ Imágenes reutilizables · 124 assets · 3 ausentes · 7 por revisar QA      │
├──────────────────────────────────────────────────────────────────────────┤
│ 🔍 Buscar…   Estado▾  Licencia▾  Origen▾  Orient.▾  Orden▾   Grid|Lista  │
│ 32 de 124 assets                              [Limpiar filtros]          │
├─────────────────────────────────────────────┬────────────────────────────┤
│  [card] [card] [card] [card]                │ Inspector                  │
│  [card] [card] [card] [card]                │ ┌────────────────────────┐ │
│  … scroll propio                            │ │ preview                │ │
│                                             │ └────────────────────────┘ │
│                                             │ Título / tags / conceptos  │
│                                             │ Licencia · QA · Uso · Prov │
│                                             │ [Guardar] [Archivar] …     │
└─────────────────────────────────────────────┴────────────────────────────┘
│ StatusBar                                                                    │
```

### 5.2 Reglas de layout CSS

```text
.panel          → flex flex-col min-h-0 min-w-0 h-full overflow-hidden
.header+toolbar → shrink-0
.body           → flex-1 min-h-0 grid
                  grid-template-columns: minmax(0,1fr) minmax(280px, 340px)
.grid-scroll    → overflow-y-auto min-h-0
.inspector      → overflow-y-auto min-h-0 border-l border-surface-800
```

- Sin scroll horizontal de ventana.
- Grid de cards: `repeat(auto-fill, minmax(180px, 1fr))`, gap 12 px.
- Cards: max-width visual ~240 px en monitores anchos (contenedor del grid, no `width` fijo absoluto).

### 5.3 Responsive

| Ancho | Comportamiento |
|-------|----------------|
| ≥ 1100 | Dos columnas fijas (grid + inspector) |
| 760–1099 | Inspector 280–300 px o drawer derecho |
| < 760 | Grid 1–2 cols; inspector = drawer/modal `role="dialog"` |

Siempre accesibles: buscador e Importar imagen.

---

## 6. Archivos

### 6.1 Crear

```text
src/lib/components/visual/VisualLibraryPanel.svelte
src/lib/components/visual/library/LibraryToolbar.svelte
src/lib/components/visual/library/AssetGrid.svelte
src/lib/components/visual/library/AssetCard.svelte
src/lib/components/visual/library/AssetInspector.svelte
src/lib/components/visual/library/LibraryEmptyState.svelte
src/lib/components/visual/library/libraryTypes.ts
src/lib/components/visual/library/libraryFilters.ts   # funciones puras (filtros/orden/tags)
```

### 6.2 Modificar

```text
src/App.svelte
src/lib/components/TopBar.svelte
src/lib/components/ModeTabs.svelte
src/lib/components/ModeNav.svelte          # si se usa en algún layout
src/lib/utils/tauri.ts                     # tipar + visualUpdateAsset
```

### 6.3 No modificar para “cerrar” el layout

Worker, supervisor, daily feed, OmniRoute, Supabase, pipeline FFmpeg, schema SQL.

---

## 7. Contratos de datos (backend real)

Serialización Rust: `#[serde(rename_all = "camelCase")]` en `MediaAsset`.  
Enums de estado/licencia/QA: `snake_case` en el wire.

### 7.1 `MediaAsset` (frontend)

Fuente: `src-tauri/src/models/visual.rs` + extensiones intel.

```ts
// libraryTypes.ts

export type AssetStatus =
  | "active"
  | "archived"
  | "blocked"
  | "missing"
  | "invalid";

export type LicenseStatus =
  | "owned"
  | "licensed"
  | "public_domain"
  | "attribution_required"
  | "unknown";

export type QaStatus =
  | "none"
  | "pending"
  | "automated_review"
  | "needs_human_review"
  | "approved"
  | "rejected";

export interface AssetProvenance {
  source: string;
  provider?: string | null;
  model?: string | null;
  prompt?: string | null;
  negativePrompt?: string | null;
  seed?: number | null;
  generatedAt?: string | null;
}

export interface MediaAsset {
  id: string;
  kind: string;
  managedPath: string;
  thumbnailPath?: string | null;
  sha256: string;
  title: string;
  description?: string | null;
  tags: string[];
  concepts: string[];
  category?: string | null;
  width: number;
  height: number;
  orientation: string; // "landscape" | "portrait" | "square" | …
  mimeType: string;
  fileSize: number;
  licenseStatus: LicenseStatus;
  source?: string | null;
  attribution?: string | null;
  timesUsed: number;
  lastUsedAt?: string | null;
  allowSameVideoRepeat: boolean;
  minimumVideosBeforeReuse: number;
  qualityScore?: number | null;
  status: AssetStatus;
  originalPath?: string | null;
  createdAt: string;
  updatedAt: string;
  literalDescription: string[];
  meanings: string[];
  positiveContexts: string[];
  negativeContexts: string[];
  hardExclusions: string[];
  aspectRatio?: string | null;
  safeArea?: string | null;
  perceptualHash?: string | null;
  qaStatus: QaStatus;
  technicalScore?: number | null;
  semanticScore?: number | null;
  provenance?: AssetProvenance | null;
  commercialUse?: boolean | null;
}
```

### 7.2 `ImportFolderResult`

```ts
export interface ImportFolderResult {
  scanned: number;
  imported: number;
  duplicates: number;
  failed: number;
  assetIds: string[];
  errors: string[];
}
```

### 7.3 `AssetUsageRow`

```ts
export interface AssetUsageRow {
  id: string;
  assetId: string;
  mediaPath: string;
  runId?: string | null;
  usedAt: string;
  outputStart?: number | null;
  outputEnd?: number | null;
}
```

### 7.4 Comandos Tauri

| Command Rust | Wrapper TS | Notas |
|--------------|------------|--------|
| `visual_list_assets` | `visualListAssets(query?, limit?)` | Default backend limit 100; **UI debe pedir 250** |
| `visual_import_image` | `visualImportImage(path, title?, tags, concepts)` | Devuelve un `MediaAsset` (si SHA existe, el existente) |
| `visual_import_folder` | `visualImportFolder(path, tags, concepts, recursive)` | `ImportFolderResult` |
| `visual_update_asset` | **añadir** `visualUpdateAsset(...)` | title/tags/concepts/license/status |
| `visual_list_usage` | `visualListUsage(assetId?, limit?)` | |
| `visual_scan_missing` | `visualScanMissing()` | `number` (conteo actualizado a missing) |
| `visual_list_concepts` | `visualListConcepts` | Opcional; fallo no bloquea catálogo |

#### Wrapper obligatorio a añadir

```ts
export async function visualUpdateAsset(params: {
  id: string;
  title?: string | null;
  tags?: string[] | null;
  concepts?: string[] | null;
  license?: LicenseStatus | null;
  status?: AssetStatus | null;
}): Promise<MediaAsset> {
  return invoke("visual_update_asset", {
    id: params.id,
    title: params.title ?? null,
    tags: params.tags ?? null,
    concepts: params.concepts ?? null,
    license: params.license ?? null,
    status: params.status ?? null,
  });
}
```

Tipar también los wrappers existentes al tocarlos (`Promise<MediaAsset[]>` etc.). No cambiar nombres camelCase de args sin verificar el command.

**Import imagen y dedupe:** hoy `visual_import_image` no expone `New` vs `Duplicate` en el valor; solo el asset. Mensaje UX:

- Si el asset ya estaba en la lista local (mismo `id` o mismo `sha256`) → *“La imagen ya existía; abrimos el asset existente.”*
- Si es nuevo id → *“Imagen importada a la biblioteca.”*

---

## 8. Carga y ciclo de vida

### 8.1 Mount de `VisualLibraryPanel`

```text
loading = true
parallel:
  assets ← visualListAssets(null, 250)
  conceptsCount ← visualListConcepts(null, 1)  // o length de list; tolerar error
finally loading = false
si error assets → error banner + Reintentar
si assets.length > 0 y !selectedId → selectedId = assets[0].id
```

- **Sin polling.** Refresco solo tras import / guardar / archivar / scan.
- No generar ni encolar jobs.

### 8.2 Selección

```text
selectedId: string | null
selected = assets.find(a => a.id === selectedId)  // o filtered list
al filtrar: si selected sale del set, reasignar al primero filtrado o null
```

### 8.3 Estado del panel (resumen)

```ts
type PanelState = {
  loading: boolean;
  error: string | null;
  assets: MediaAsset[];
  selectedId: string | null;
  query: string;           // debounced hacia backend
  queryInput: string;      // inmediato en input
  filterStatus: AssetStatus | "all";
  filterLicense: LicenseStatus | "all";
  filterOrigin: "all" | "imported" | "ai";
  filterOrientation: "all" | "landscape" | "portrait" | "square";
  sort: "recent" | "used" | "quality" | "title";
  view: "grid" | "list";
  importing: boolean;
  message: string | null;  // aria-live
};
```

---

## 9. Búsqueda, filtros y orden

### 9.1 Buscador (server-side via SQLite)

| Propiedad | Valor |
|-----------|--------|
| Placeholder | `Buscar por título, etiqueta o concepto…` |
| Debounce | 300 ms |
| Límite | 250 |
| Escape | limpia input con foco |
| Cancelación | ignorar respuestas out-of-order (token monotónico) |

```ts
// pseudo
let seq = 0;
async function runSearch(q: string) {
  const my = ++seq;
  const list = await visualListAssets(q || null, 250);
  if (my !== seq) return;
  assets = list as MediaAsset[];
}
```

### 9.2 Filtros locales (sobre lista retornada)

Implementar en `libraryFilters.ts` como funciones puras:

```ts
export function filterAssets(
  assets: MediaAsset[],
  f: {
    status: AssetStatus | "all";
    license: LicenseStatus | "all";
    origin: "all" | "imported" | "ai";
    orientation: "all" | "landscape" | "portrait" | "square";
  },
): MediaAsset[];

export function sortAssets(
  assets: MediaAsset[],
  sort: "recent" | "used" | "quality" | "title",
): MediaAsset[];

export function normalizeTags(raw: string): string[];
// split por coma, trim, drop empty, dedupe case-insensitive, preservar primera forma

export function isAiOrigin(a: MediaAsset): boolean;
// source?.startsWith("ai:") || provenance?.source === "ai_generated"

export function isMockAsset(a: MediaAsset): boolean;
// provenance?.provider === "mock" || source === "ai:mock" || source?.includes("mock")

export function previewPath(a: MediaAsset): string | null;
// thumbnailPath || managedPath; null si missing
```

**Origen IA:** no inferir solo por “hay provider”.  
**Orientación:** preferir `orientation`; fallback `width`/`height`.

### 9.3 Orden

| Opción | Criterio |
|--------|----------|
| Más recientes | `createdAt` desc |
| Más usadas | `timesUsed` desc, luego `lastUsedAt` |
| Mejor calidad | `qualityScore` desc; nulls al final |
| Título A–Z | `localeCompare` es |

### 9.4 UI de filtros

- Conteo: `32 de 124 assets`.
- `Limpiar filtros` si algún filtro ≠ default o query no vacío.
- No persistir en SQLite (sesión en memoria OK).

---

## 10. Grid y `AssetCard`

### 10.1 Contenido de card

1. Thumbnail (`convertFileSrc` + `loading="lazy"`)  
2. Contenedor uniforme **aspect 4:3**, imagen `object-cover` o `object-contain` (preferir contain si letterbox mejora legibilidad)  
3. Título (máx. 2 líneas, `line-clamp-2`)  
4. Hasta 2 chips: conceptos o tags  
5. Badge licencia (texto corto ES)  
6. Badge estado solo si ≠ `active`  
7. Uso: `· 3 usos` con `aria-label="Usado 3 veces"`  
8. Chip `IA` si `isAiOrigin`  
9. Chip `Mock` violeta si `isMockAsset`  

### 10.2 Preview roto / missing

- `status === "missing"` o `onerror` en img → placeholder “Archivo no disponible”.  
- Error de una card **no** desmonta el grid.

### 10.3 Interacción

| Acción | Efecto |
|--------|--------|
| Click / Enter / Space | Selecciona (`aria-selected=true`) |
| Doble click | Ninguna acción destructiva (no-op o solo re-seleccionar) |
| Estilo seleccionado | borde `violet-500` / `brand`, fondo `surface-900` |

Card = `<button type="button">` (no botones anidados).

### 10.4 Vista lista (opcional MVP+)

Fila: thumb 48×48 · título · licencia · usos · estado. Misma selección.

---

## 11. Inspector (`AssetInspector`)

Visible con selección. Props mínimas:

```ts
{
  asset: MediaAsset;
  usage: AssetUsageRow[];
  usageLoading: boolean;
  busy: boolean;
  onSave: (patch: UpdatePatch) => Promise<void>;
  onArchive: () => Promise<void>;
  onRestore: () => Promise<void>;
  onBlock: () => Promise<void>;
}
```

### 11.1 Secciones

| Sección | Contenido | Editable |
|---------|-----------|----------|
| Preview | Imagen grande, WxH, MIME, tamaño legible, fechas | No |
| Identidad | Título, tags (chips + input coma), conceptos | Sí |
| Licencia | Select + warning si `unknown` | Sí |
| QA | Estado, scores %, contextos, exclusiones | Solo lectura |
| Uso | timesUsed, lastUsedAt, filas usage | Solo lectura |
| Procedencia | provider/model/prompt colapsable; badge mock | Solo lectura |
| Acciones | Guardar / Archivar / Restaurar / Bloquear | — |

### 11.2 Copy licencia unknown

> Uso comercial no verificado. Revisa la licencia antes de publicar.

Si `commercialUse === true`, mostrar como **dato** (no editable en MVP).

### 11.3 Copy mock

> Simulación (mock). Esta imagen no fue generada por una IA local real.

### 11.4 Guardar

- Botón `Guardar cambios` habilitado solo con dirty form.  
- No auto-save en cada tecla.  
- Tras éxito: reemplazar asset en array con respuesta backend; mantener selección; toast.  
- Tras error: conservar formulario; mensaje reintentable.

### 11.5 Archivar / Restaurar / Bloquear

| Acción | `status` enviado | Confirmación |
|--------|------------------|--------------|
| Archivar | `archived` | No (reversible) |
| Restaurar | `active` | No |
| Bloquear | `blocked` | Sí: “¿Bloquear este asset? No se sugerirá en matching.” |

**Sin borrado físico** (no hay command seguro).

### 11.6 Carga de uso

Al cambiar `selectedId` → `visualListUsage(id, 20)`. Independiente del catálogo. Empty: *“Aún no se ha usado en ningún video.”*

---

## 12. Importación

### 12.1 Importar imagen

1. `@tauri-apps/plugin-dialog` → `open({ multiple: false, filters: png/jpg/jpeg/webp })`  
2. `visualImportImage(path, null, [], [])`  
3. `importing=true` solo desactiva botones de import  
4. Refrescar lista o insertar/actualizar en memoria  
5. Seleccionar asset retornado  

### 12.2 Importar carpeta

1. `open({ directory: true })`  
2. Checkbox UI: “Incluir subcarpetas” (default false)  
3. `visualImportFolder(path, [], [], recursive)`  
4. Toast con resumen real:  
   `Carpeta: {imported} nuevas, {duplicates} duplicadas, {failed} errores (de {scanned} escaneadas).`  
5. Si `errors.length`, detalle colapsable (máx. 5 líneas + “y N más”).  
6. Recargar catálogo siempre (incluso fallo parcial).

### 12.3 Entorno web (`!isTauri()`)

- Botones import deshabilitados.  
- Texto: `Disponible en la aplicación de escritorio`.  
- Layout visible; **sin** assets inventados.

### 12.4 Buscar archivos ausentes

1. `visualScanMissing()` → N  
2. Toast: `Se marcaron N archivos ausentes.`  
3. Recargar catálogo  
4. Botón en toast o toolbar: `Ver ausentes` → `filterStatus = "missing"`  

No borrar registros.

---

## 13. Estados de pantalla

| Estado | UI |
|--------|-----|
| Loading inicial | Skeleton 8 cards; header/toolbar visibles |
| Vacía (0 assets, sin filtros) | `LibraryEmptyState` |
| Sin resultados (filtros) | “No encontramos assets con estos filtros.” + Limpiar |
| Error carga | Mensaje + Reintentar (+ detalle colapsable) |
| Card rota | Placeholder local |

### Empty state

- Título: `Tu biblioteca visual está vacía`  
- Texto: `Importa imágenes que quieras reutilizar en futuros videos.`  
- Primario: `Importar primera imagen`  
- Secundario: `Importar una carpeta`  
- Tres bullets:  
  - Busca antes de generar  
  - Conserva licencia y procedencia  
  - Reutiliza sin perder contexto  

No seed de conceptos automático. No generación.

---

## 14. Copy deck (ES)

| Clave | Texto |
|-------|--------|
| title | Biblioteca visual |
| subtitle | Imágenes reutilizables para tus videos |
| importImage | Importar imagen |
| importFolder | Importar carpeta |
| scanMissing | Buscar archivos ausentes |
| searchPh | Buscar por título, etiqueta o concepto… |
| save | Guardar cambios |
| archive | Archivar |
| restore | Restaurar |
| block | Bloquear |
| clearFilters | Limpiar filtros |
| retry | Reintentar |
| importedOk | Imagen importada a la biblioteca. |
| importedDup | La imagen ya existía; abrimos el asset existente. |
| folderSummary | Carpeta importada: {n} nuevas, {d} duplicadas. |
| saved | Cambios guardados. |
| archived | Asset archivado. |
| blocked | Asset bloqueado. |
| loadError | No se pudo cargar la biblioteca. Reintenta. |
| licenseWarn | Uso comercial no verificado. Revisa la licencia antes de publicar. |
| mockBadge | Simulación (mock) — no es IA |
| noUsage | Aún no se ha usado en ningún video |
| desktopOnly | Disponible en la aplicación de escritorio |

Licencias (labels):

| wire | UI |
|------|-----|
| unknown | Desconocida |
| owned | Propia |
| licensed | Licenciada |
| public_domain | Dominio público |
| attribution_required | Requiere atribución |

---

## 15. Accesibilidad

- Teclado completo (tabs, cards, filtros, drawer).  
- Focus visible (patrón global).  
- `aria-live="polite"` para mensajes.  
- `aria-selected` en card activa.  
- Labels en todos los selects.  
- Drawer: `role="dialog"`, `aria-labelledby`, Escape cierra, devuelve foco.  
- No solo color para estado/licencia (texto + badge).  
- Targets ≥ 36 px.  
- `alt={asset.title}`.  
- `npm run check` sin warnings nuevos en estos archivos.

---

## 16. Rendimiento

| Regla | Detalle |
|-------|---------|
| Límite MVP | 250 assets |
| Imágenes | `convertFileSrc` + `loading="lazy"`; nunca base64 |
| Derivados | `$derived` en panel; no recalcular por card |
| Debounce | 300 ms búsqueda |
| Virtualización | Solo si se mide jank; documentar paginación como next |

---

## 17. Diseño visual

| Token | Uso |
|-------|-----|
| `surface-950` | Fondo |
| `surface-900/90` | Paneles |
| `surface-800` | Bordes |
| `violet-500/600` | Selección Biblioteca / tab activo |
| `amber-*` | Warning licencia |
| `red-*` suave | Missing / error |
| `emerald-*` | QA approved |
| `violet-*` | Mock badge |

- Radio cards 12–14 px.  
- Toolbar compacta; controles principales ≥ 11 px.  
- Inspector metadata 11–12 px.  
- SVG inline; sin nueva icon library.

---

## 18. División de componentes

```text
VisualLibraryPanel
├── header (métricas + import actions)
├── LibraryToolbar (search, filters, sort, counts)
├── body
│   ├── AssetGrid
│   │   ├── skeletons | empty | no-results
│   │   └── AssetCard[]
│   └── AssetInspector
│       ├── preview
│       ├── form identidad/licencia
│       ├── QA / usage / provenance
│       └── actions
└── aria-live region
```

| Componente | Llama Tauri | Contiene |
|------------|-------------|----------|
| VisualLibraryPanel | Sí (orquesta) | Estado global pantalla |
| LibraryToolbar | No | Callbacks |
| AssetGrid | No | Layout + empty |
| AssetCard | No | Presentación |
| AssetInspector | Opcional solo usage vía callback del padre | Form + acciones |
| LibraryEmptyState | No | CTAs import |

---

## 19. Separación de dominios (importante)

```text
┌────────────────────┐     ┌──────────────────────────┐
│ Modo Biblioteca    │     │ Modo Visual → Imágenes IA│
│ catálogo assets    │     │ needs / jobs / candidates│
│ import / metadata  │     │ generate / approve / daily│
└─────────┬──────────┘     └────────────┬─────────────┘
          │                             │
          └──────────► SQLite library.db ◄─────────────┘
                       media_assets ──◄── promote_candidate
```

- Assets aprobados desde generación **aparecen** en esta Biblioteca tras promote.  
- Esta UI **no** lanza generación.  
- Daily feed se opera en Imágenes IA / supervisor, no aquí.

---

## 20. Plan de implementación (PRs / pasos)

| Paso | Entrega | Criterio |
|------|---------|----------|
| **P0** | `WorkspaceMode` + tab siempre visible + shell `VisualLibraryPanel` vacío | Abre sin video |
| **P1** | `libraryTypes` + wrappers tipados + listado real + grid | Assets de SQLite |
| **P2** | Toolbar búsqueda/filtros/orden + empty/error | Filtros correctos |
| **P3** | Inspector lectura + usage | Preview y uso |
| **P4** | Edición + guardar + archivar/bloquear | Persistencia real |
| **P5** | Import imagen/carpeta + scan missing | Dialogs desktop |
| **P6** | Responsive drawer + a11y + polish copy/mock/licencia | Check + build |

No mezclar generación/daily en estos PRs.

---

## 21. Pruebas

### 21.1 Funciones puras (`libraryFilters.ts`)

Si no hay vitest/jest, dejar funciones exportadas y, si el repo ya tiene tests TS, añadirlos; si no, no instalar framework sin acuerdo. Mínimo documentado:

- `normalizeTags(" A, a, ,B ")` → `["A","B"]`  
- Filtro status/license/origin  
- Orden quality con nulls al final  
- `isMockAsset` / `isAiOrigin`  
- `previewPath` fallback  

### 21.2 Smoke manual

1. Abrir app **sin** video → tab Biblioteca → empty o catálogo real.  
2. Importar PNG → card + inspector.  
3. Editar título/tags/conceptos/licencia → Guardar → reiniciar app → persiste.  
4. Archivar → restaurar.  
5. Reimportar misma imagen → mensaje de existente.  
6. Scan missing sobre fixture controlado.  
7. Teclado: tab, flechas opcionales, Enter en cards, Escape en drawer.  
8. Anchos: 1280×720, 1024×768, 720×800.  
9. Confirmar que **no** hay botones de generar ni worker tick en red (DevTools/invoke).

### 21.3 Checks CI locales

```bash
npm run check
npm run build
git diff --check
```

---

## 22. Criterios de aceptación

- [ ] Biblioteca abre sin video, EDL ni `projectKey`.  
- [ ] Cuarto modo en TopBar / ModeTabs.  
- [ ] Datos solo de `visual_list_assets` (no seed frontend).  
- [ ] Búsqueda, filtros y orden con conteos correctos.  
- [ ] Import imagen/carpeta con resultados reales.  
- [ ] Grid tolera thumbs rotos y `missing`.  
- [ ] Inspector edita vía `visual_update_asset`.  
- [ ] Licencia `unknown` con advertencia.  
- [ ] Mock = simulación, no “IA local”.  
- [ ] Sin borrado físico.  
- [ ] Sin `worker_tick` ni generación ni Supabase ni pagos.  
- [ ] `npm run check` y `npm run build` verdes.  
- [ ] Cambios no relacionados se conservan.

---

## 23. Definition of Done

1. Componentes según §6 y §18.  
2. Navegación global §4.  
3. Catálogo real §8–10.  
4. Import + edición §11–12.  
5. Estados loading/empty/error/missing.  
6. Responsive + teclado.  
7. Copy honesto (mock / licencia).  
8. Checks verdes.  
9. Informe breve: archivos, decisiones, comandos, capturas (vacía, grid, inspector, móvil).  

---

## 24. Fuera de alcance (backlog explícito)

| Item | Notas |
|------|--------|
| Generar desde Biblioteca | Usar modo Visual → Imágenes IA |
| Supabase sync | Diseño futuro |
| Marketplace | — |
| Borrado físico | Destructivo; sin command |
| Edición/crop de imagen | — |
| Drag-and-drop a VisualPlan | Requiere video + plan |
| Multi-selección / bulk | — |
| Paginación server-side | Si >250 se mide necesidad |
| Gestión avanzada themes/concepts | Solo conteo opcional |
| Atajos globales modo | Opcional P6+ |

No implementar a medias ni marcar como listos en UI.

---

## 25. Referencias de código

| Tema | Ruta |
|------|------|
| `MediaAsset` | `src-tauri/src/models/visual.rs` |
| `QaStatus` / `AssetProvenance` | `src-tauri/src/models/visual_intel.rs` |
| Commands | `src-tauri/src/commands/visual.rs` |
| Import folder result | `src-tauri/src/pipeline/visual/library.rs` → `ImportFolderResult` |
| Wrappers actuales | `src/lib/utils/tauri.ts` |
| Modos hoy | `src/App.svelte`, `ModeTabs.svelte` |
| Supervisión generación | `src/lib/components/visual/ImageGenerationPanel.svelte` |
| Spec generación | `docs/visual-library/supervision-ui.md` |

---

## 26. Ejemplo de props del panel raíz

```svelte
<script lang="ts">
  import VisualLibraryPanel from "$lib/components/visual/VisualLibraryPanel.svelte";
</script>

<VisualLibraryPanel
  onMessage={(m) => showToast(m)}
  onError={(e) => { projectStore.error = e; }}
/>
```

El panel **no** recibe `projectKey` ni `mediaPath`. Si en el futuro se añade “Usar en plan actual”, será prop opcional y deshabilitada sin video — **no MVP**.
