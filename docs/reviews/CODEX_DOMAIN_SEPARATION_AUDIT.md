# Codex Domain Separation Audit

Fecha: 2026-07-24
Rama: `feat/independent-visual-library`

## Comportamiento real observado

- Biblioteca abre sin vídeo y permite buscar, importar, solicitar, revisar y aprobar imágenes.
- `LibraryImageRequest` y generación están expuestos en el modo Biblioteca.
- `VisualNeed` y el supervisor viven físicamente bajo `pipeline/visual` como compatibilidad.
- `SceneAssetAssignment` y `VisualPlacement` transportan `media_asset_id`.
- El selector de B-roll consultaba Biblioteca y también exponía botones para importar/generar; esos botones se retiraron en esta fase. Ahora ofrece coincidencias, selección, omitir y abrir Biblioteca.
- OmniRoute real no está verificado; mock es la ruta probada.

## Deuda organizativa

La ubicación física de `VisualNeed`, generación y supervisor aún mezcla módulos. La extracción debe hacerse después de estabilizar contratos y sin mover tablas destructivamente.

## Acoplamiento funcional encontrado

El adaptador de compatibilidad conserva funciones de generación para otros flujos y pruebas. Debe quedar inaccesible desde B-roll. La UI de revisión y administración permanece en Biblioteca/Fábrica, no en el selector de vídeo.

## Riesgo de regresión

Eliminar generación del selector puede descubrir consumidores que dependían de `pickerGenerate`; el check Svelte debe ser obligatorio y el flujo de colocación de assets aprobados debe probarse con render. Jobs antiguos queued/running requieren una política de migración y cancelación independiente.

## Cambios aplicados

- Selector B-roll sin `onGenerate` ni `onImport`.
- Mensaje de ausencia orientado a Biblioteca, no a generación inmediata.
- Acción `Abrir Biblioteca` como navegación independiente.
- Contratos/documentación de dependencias y buscador de solo lectura.

## Fuera de alcance de esta fase

Theme Factory completa, `UncoveredVisualNeed` persistente, proveedor de texto OmniRoute, embeddings, Supabase runtime y futuro creador visual.
