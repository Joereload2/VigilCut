# Informe técnico: comportamiento esperado de Biblioteca Visual y B-roll

Fecha: 2026-07-24
Rama revisada: `feat/independent-visual-library`
Commits relevantes: `d45ede9`, `1e164a4`

## 1. Interpretación de las instrucciones de producto

La Biblioteca Visual debe ser un dominio local-first, independiente de vídeo. Su unidad principal es el `MediaAsset` aprobado y reutilizable. La Biblioteca es propietaria de importación, búsqueda, generación, candidatos, QA, procedencia, licencia, metadatos y uso. B-roll no debe crear ni administrar assets; solamente consume assets de Biblioteca para cubrir necesidades visuales de un vídeo.

La separación conceptual es:

```text
Biblioteca
  LibraryImageRequest -> búsqueda -> GenerationJob -> GeneratedCandidate
  importación / QA / revisión humana -> MediaAsset
  búsqueda, metadatos, licencia, procedencia, usage

B-roll
  vídeo -> VisualNeed -> búsqueda en Biblioteca
  coincidencia -> SceneAssetAssignment -> VisualPlacement -> VisualPlan
  sin coincidencia -> solicita generación a Biblioteca
```

La regla de seguridad del dominio es que ningún candidato generado se convierte automáticamente en asset activo. El único ingreso utilizable es una operación de ingestión idempotente después de aprobación o importación validada. Un `VisualPlacement` solo debe referenciar un `media_asset_id`; nunca un candidato, prompt, ruta temporal o necesidad de vídeo.

## 2. Cómo debería funcionar Biblioteca

### 2.1 Entrada del usuario

La pantalla debe abrir sin `mediaPath`, `projectKey`, EDL, transcripción, `needId` o `VisualPlan`. El usuario escribe una intención simple, por ejemplo `dinero`. El sistema normaliza esa intención, deriva términos relacionados y ejecuta una búsqueda semántica/textual sobre título, descripción, conceptos, tags, tema, formato y orientación.

La interfaz debe mostrar tres resultados posibles sin lenguaje interno:

- Hay imágenes reutilizables: miniatura, motivo de coincidencia y acción `Usar`.
- No hay coincidencia suficiente: acción `Crear una nueva`.
- No se puede generar: explicación del proveedor, límite o configuración, sin fingir éxito.

Tema, descripción detallada, exclusiones, formato y estilo son parámetros opcionales. Si no se completan, el backend los deriva de la intención y aplica valores seguros por defecto (16:9, una imagen, revisión humana).

### 2.2 Generación

Antes de encolar se registra que se buscó. Si el usuario decide generar, se crea o actualiza `LibraryImageRequest`, se construye el prompt final y se encola en el worker residente. La UI no debe ejecutar ticks del worker ni bloquear esperando HTTP.

El proveedor debe recibir el prompt negativo cuando lo soporte. Si solo acepta un prompt, debe registrarse la estrategia de incorporación de exclusiones. El job persiste estado, proveedor, modelo, coste y timestamps. `mock` debe estar rotulado como simulación y `free_configured` nunca equivale a gratuidad verificada.

### 2.3 Revisión y aprobación

El resultado aparece en `Por revisar`, con preview, intención, prompt positivo/negativo, dimensiones, proveedor, modelo, coste, estado mock/real, QA, fecha y error. Las operaciones son `Aprobar para Biblioteca`, `Rechazar`, `Editar y regenerar` y `Cancelar` si aún está en cola.

Aprobar debe ser idempotente: dos clics o reintentos producen un solo `MediaAsset`, deduplicado por SHA-256. Rechazar no crea un asset activo. Regenerar conserva trazabilidad entre solicitud, candidato anterior y candidato nuevo.

### 2.4 Asset aprobado

`MediaAsset` conserva archivo administrado, thumbnail, hashes, título, tema, conceptos, tags, prompts, orientación, dimensiones, estilo, fuente, proveedor, modelo, licencia, permiso comercial, estado, fechas y usage. Licencia desconocida debe permanecer `unknown`/`NULL`; no se debe inventar `Owned` ni `commercial_use=true`.

## 3. Cómo debería funcionar B-roll

B-roll comienza con un vídeo ya abierto y sus necesidades visuales. Cada `VisualNeed` contiene lenguaje de edición, no detalles de almacenamiento. El adaptador llama a una interfaz de Biblioteca equivalente a `search`, `get_asset`, `request_generation` y `record_usage`.

Orden obligatorio:

1. Crear o cargar `VisualNeed`.
2. Buscar en Biblioteca usando términos, contexto, exclusiones, formato y orientación.
3. Si existe un asset elegible, seleccionarlo y crear `SceneAssetAssignment`.
4. Crear `VisualPlacement` usando únicamente `media_asset_id`.
5. Validar composición en `VisualPlan` y renderizar.
6. Si no existe asset, solicitar generación a Biblioteca.
7. Esperar candidato, QA y aprobación humana.
8. Tras aprobación, reutilizar el `media_asset_id` para la asignación y placement.

La búsqueda automática de B-roll debe ser más restrictiva que la búsqueda manual. Assets bloqueados, archivados, ausentes o con licencia desconocida no deben asignarse automáticamente. Una selección manual puede mostrar esos assets con advertencia explícita.

B-roll debe tener un selector compacto y un enlace `Abrir Biblioteca`, no una copia completa del explorador. La aprobación de un candidato es una operación de Biblioteca; `colocar en vídeo` es una acción posterior y secundaria.

## 4. Comportamiento actualmente implementado

La rama revisada ya implementa el flujo manual independiente mediante `LibraryImageRequest`, persistido en SQLite. Los comandos de Biblioteca no requieren vídeo, EDL, transcripción, `mediaPath` ni `needId` de vídeo.

El flujo actual es:

```text
LibraryImageRequest(searched)
  -> búsqueda de assets con coincidencias y score
  -> usar asset existente o continuar explícitamente
  -> generation_jobs en supervisor residente
  -> GeneratedCandidate(pending_review)
  -> aprobación humana
  -> LibraryService::ingest_asset
  -> MediaAsset activo
```

La interfaz actualizada permite escribir una intención simple como `dinero`; los detalles avanzados quedan ocultos y se infieren concepto y descripción cuando faltan. La búsqueda muestra miniaturas y puntuación. El candidato mock se marca como simulación. La aprobación es idempotente y la generación conserva prompts, dimensiones y trazabilidad.

En B-roll, `VisualNeed` continúa existiendo como adaptador de compatibilidad interno. `SceneAssetAssignment` ya transporta `media_asset_id`, y el pipeline de composición/render mantiene `VisualPlacement` separado del catálogo. El matching automático bloquea licencias desconocidas por defecto.

## 5. Concordancia con las instrucciones

| Requisito | Estado actual | Evaluación técnica |
|---|---|---|
| Biblioteca sin vídeo | Implementado | Comandos y flujo manual independientes |
| Intención simple antes de generar | Implementado | Búsqueda previa y botón explícito para continuar |
| Una única entrada aprobada a Biblioteca | Implementado | Aprobación llama a ingestión idempotente |
| Candidato no aprobado fuera de assets | Implementado | `pending_review` separado de `MediaAsset` |
| Dedupe por SHA-256 | Implementado | Importación/aprobación evita duplicados |
| Prompt negativo editable y persistido | Implementado | Se conserva en solicitud y candidato |
| Mock honesto | Implementado | UI indica simulación y no IA real |
| Licencia no inventada | Implementado | Generado/importado queda unknown sin evidencia |
| Worker independiente de Svelte | Parcialmente implementado | Supervisor residente reutilizado; conviene completar pruebas de reinicio/cancelación real |
| B-roll como consumidor | Parcial | Contratos y assignment están separados, pero el adaptador `VisualNeed` aún vive bajo `pipeline/visual` |
| Story Builder consumidor futuro | Contratos, no producto | Correcto para el alcance actual |
| Daily feed sin vídeo | Parcial | Existe infraestructura, pero no debe considerarse equivalente al flujo manual hasta validar scheduler y gratuidad verificada |
| OmniRoute real | No verificado | No se ejecutó por ausencia de credenciales/autorización de coste |
| Supabase runtime | Opcional/no operativo | SQLite sigue siendo la fuente local; no debe anunciarse sincronización real |

## 6. Brechas y riesgos restantes

1. El límite físico entre Biblioteca y B-roll todavía es gradual: `VisualNeed` y parte de la cola residen bajo `pipeline/visual`. Es una deuda de organización, no una dependencia del flujo manual.
2. La cancelación de una solicitud remota es cooperativa: cancela el future local, pero no garantiza que el proveedor externo haya abortado su trabajo.
3. La revisión GUI manual está documentada y el flujo backend está probado; falta automatización de navegador/foco para cubrir todos los estados de UX.
4. OmniRoute, licencias externas, coste real y redirects de proveedores deben verificarse en un entorno autorizado antes de activar generación automática.
5. Daily feed debe permanecer desactivado para proveedores cuya gratuidad no esté documentada y verificada.

## 7. Criterios técnicos de aceptación

La implementación debe considerarse correcta cuando:

- Biblioteca abre y busca con cero vídeos abiertos.
- `dinero` produce búsqueda relacionada sin completar una ficha técnica.
- Generar siempre crea candidato revisable, nunca asset aprobado automático.
- Aprobar dos veces produce un solo asset estable.
- El asset aprobado puede reutilizarse desde otro vídeo.
- B-roll nunca escribe directamente un `GeneratedCandidate` en un `VisualPlacement`.
- Cerrar y reabrir conserva solicitudes, jobs, candidatos y assets.
- Ningún proveedor pagado se activa por configuración ambigua.
- La UI diferencia búsqueda, generación, revisión y disponibilidad.

## 8. Conclusión

La dirección arquitectónica correcta es Biblioteca como catálogo y sistema de adquisición de imágenes; B-roll como consumidor temporal que selecciona y coloca assets en un vídeo. La implementación actual ya cumple el núcleo manual y local-first, incluyendo búsqueda previa, generación revisable, ingestión idempotente y separación de placement. Lo pendiente debe tratarse como endurecimiento operativo y extracción gradual del adaptador, no como motivo para volver a hacer depender Biblioteca de un vídeo.
