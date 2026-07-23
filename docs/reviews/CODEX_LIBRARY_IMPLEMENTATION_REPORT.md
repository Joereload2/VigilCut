# Codex Library Implementation Report

## Resultado

La Biblioteca funciona como modo principal sin video, proyecto, EDL ni transcripción. El flujo manual crea una solicitud persistente, busca assets reutilizables, permite continuar voluntariamente, encola una sola generación, presenta el candidato para revisión humana y solo lo incorpora mediante aprobación idempotente.

## Implementación

- Formulario visible `+ Nueva imagen` con tema, concepto, descripción, exclusiones, formato y estilo.
- Valores iniciales: 16:9, fotografía realista, una imagen y revisión humana obligatoria.
- Prompt positivo construido por backend y ambos prompts editables antes de generar o regenerar.
- Búsqueda previa persistida (`searched_at`) con coincidencias, miniatura, puntuación y acceso al asset existente.
- Entidad local ampliada con origen, tema, descripción, prompts, dimensiones, estilo, estado y timestamps.
- Migraciones SQLite v6 y v7 aditivas e idempotentes; filas anteriores reciben valores compatibles.
- Cola y supervisor residentes reutilizados; no existe segundo worker ni dependencia de Svelte para ejecutar jobs.
- Dimensiones reales por formato: 1280×720, 720×1280, 1024×1024 y 1024×1280.
- Revisión con preview, tema, concepto, prompts, dimensiones, proveedor, modelo, coste, mock/real, fecha y QA.
- Regeneración crea un job y candidato nuevos, conserva el candidato anterior y persiste los prompts revisados.
- Aprobación ingresa mediante `LibraryService::ingest_asset`; licencia y uso comercial quedan desconocidos si no hay evidencia.
- Importación manual ya no inventa propiedad ni permiso comercial.
- Búsqueda manual puede mostrar licencias desconocidas; matching automático de B-roll mantiene el bloqueo.
- Límite de importación de 100 MB y 80 megapíxeles, además de decodificación y formatos existentes.
- Buscador ampliado a descripción, tema, origen y relación de aspecto; filtros de tema, formato, orientación, origen y estado.
- Refresco residente de la Biblioteca mantiene actividad y bandeja global actualizadas sin proyecto.

## Archivos principales

- `src/lib/components/visual/LibraryControlCenter.svelte`
- `src/lib/components/visual/LibraryView.svelte`
- `src/lib/components/visual/ReviewInbox.svelte`
- `src/lib/components/visual/VisualWorkspace.svelte`
- `src-tauri/src/pipeline/visual/library_requests.rs`
- `src-tauri/src/pipeline/visual/generation/worker.rs`
- `src-tauri/src/pipeline/visual/generation/supervision.rs`
- `src-tauri/src/pipeline/visual/schema.rs`

## Decisiones arquitectónicas

1. Biblioteca es el dueño funcional de solicitud, búsqueda, generación, revisión e ingestión.
2. `VisualNeed` sigue siendo un adaptador interno para la cola existente; no aparece en la UI ni se exige a comandos manuales.
3. Las coincidencias con licencia desconocida son visibles solo bajo supervisión humana; nunca se habilitan silenciosamente para automatización.
4. Mock se identifica como simulación y nunca como IA real.
5. `OMNIROUTE_FREE_TIER` no convierte la ruta en gratuidad verificada ni habilita ciclos automáticos.
6. Supabase permanece opcional y fuera del camino operativo.

## Prueba integral sin video

Caso automatizado: `complete_manual_flow_without_video_persists_and_requires_approval`.

Verifica solicitud manual, búsqueda previa, persistencia, enqueue sin media, generación mock, dimensiones 16:9, prompts, candidato fuera de la biblioteca, aprobación idempotente, licencia desconocida, metadatos y persistencia final. Un segundo caso verifica rechazo sin asset y regeneración trazable.

Comprobación manual reproducible:

1. Abrir VigilCut sin video y entrar en Biblioteca.
2. Pulsar `+ Nueva imagen`.
3. Tema Economía; concepto Inflación; descripción “Familia comparando precios en un supermercado”.
4. No debe contener: texto, logos, marcas, billetes estadounidenses, manos deformes.
5. Mantener Horizontal 16:9 y Fotografía realista.
6. Pulsar Buscar coincidencias; elegir una existente o continuar.
7. Con `VIGILCUT_IMAGE_PROVIDER=mock`, pulsar Continuar con simulación.
8. Abrir Por revisar, comprobar la etiqueta `SIMULACIÓN · NO ES IA REAL`, revisar prompts y aprobar para Biblioteca.
9. Cerrar y abrir VigilCut; buscar Inflación y confirmar el asset persistido.

## Pruebas y resultados

- Prueba focal integral: 3 aprobadas, 0 fallidas.
- `cargo check --all-targets`: aprobado.
- `npm run check`: 0 errores; permanece una advertencia preexistente en `ExportSuccess.svelte`.
- `npm run build`: aprobado.
- `npm run test:fmt`: aprobado.
- `npm run test:clippy`: aprobado con `-D warnings`.
- `npm run test:unit:visual`: 45 aprobadas, 0 fallidas.
- `npm run test:smoke`: 7 aprobadas, 0 fallidas.
- `git diff --check`: aprobado.

## Riesgos conocidos

- El adaptador interno todavía vive físicamente bajo `pipeline/visual`; moverlo no aporta independencia funcional inmediata y se pospone.
- La cancelación HTTP es cooperativa: el worker abandona el future mediante `tokio::select!`; un proveedor remoto puede completar trabajo en su extremo.
- No se realizó generación OmniRoute real: requeriría credenciales y autorización de coste.
- La prueba manual GUI queda documentada de forma reproducible; el equivalente backend integral sí fue ejecutado.

## Trabajo futuro

- Edición avanzada de atribución y política de licencia por asset.
- Eliminación confirmada de candidatos rechazados con papelera recuperable.
- Extraer el adaptador `VisualNeed` a infraestructura de compatibilidad cuando B-roll migre por completo.
- Pruebas UI automatizadas de teclado/foco para el diálogo.
- Story Builder y video compuesto por imágenes permanecen fuera de alcance.
