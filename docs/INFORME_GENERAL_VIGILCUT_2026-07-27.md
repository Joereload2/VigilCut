# Informe general de VigilCut

**Fecha de revisión:** 27 de julio de 2026  
**Versión declarada:** 1.1.0  
**Rama revisada:** `feat/independent-visual-library`  
**Estado general:** beta funcional avanzada; motor sólido, interfaz e integraciones todavía en consolidación.

## 1. Resumen ejecutivo

VigilCut es una aplicación de escritorio para automatizar la postproducción de contenido audiovisual. No busca ser un editor tradicional como Premiere o DaVinci, sino una fábrica local de contenido donde la aplicación ejecuta el trabajo repetitivo y la persona supervisa las excepciones.

El proyecto ya cuenta con un motor real para analizar vídeos, detectar silencios, construir una EDL, exportar vídeos, generar clips verticales, procesar lotes y administrar una biblioteca visual. El backend presenta un nivel de madurez mayor que la interfaz. Los principales pendientes están en la estabilidad del recorrido completo de usuario, las pruebas de interfaz, la simplificación del frontend y el endurecimiento de integraciones externas.

Estimación general de avance:

| Área | Avance estimado |
|---|---:|
| Motor local | 80–85% |
| Biblioteca visual | 70–80% |
| B-roll y composición | 60–70% |
| UX para público general | 55–65% |
| Integraciones cloud | 25–40% |
| Producto total frente a la visión | Aproximadamente 70% |

## 2. Qué busca ser la aplicación

La cadena principal del producto es:

```text
Vídeo → análisis → eventos → políticas → EDL → exportaciones
```

Objetivos principales:

- Eliminar silencios automáticamente.
- Detectar decisiones dudosas y enviarlas a supervisión humana.
- Crear clips verticales para Shorts, Reels y TikTok.
- Generar capítulos, metadatos y artefactos auxiliares.
- Procesar varios vídeos mediante lotes e inbox.
- Añadir imágenes y B-roll sin modificar la edición base.
- Mantener el funcionamiento local y offline siempre que sea posible.
- Incorporar generación externa y sincronización como opciones controladas.

VigilCut no pretende convertirse en un NLE completo con múltiples pistas, keyframes y edición manual exhaustiva.

## 3. Funcionalidades implementadas

### 3.1 Edición automática

- Análisis de medios con FFmpeg y FFprobe.
- Detección de silencios.
- Silero VAD mediante ONNX, con fallback.
- Detección de respiraciones, micropausas y muletillas.
- Eventos estructurados con namespaces.
- Políticas de corte configurables.
- EDL como representación canónica de la edición.
- Cola de excepciones.
- Modos seguro, supervisado y agresivo.
- Preview que omite regiones cortadas.
- Recompilación del EDL después de revisar excepciones.

### 3.2 Exportación

La aplicación puede producir:

- Vídeo editado MP4.
- Manifiesto JSON.
- Eventos y EDL.
- Capítulos JSON.
- Capítulos TXT para YouTube.
- Información de shorts.
- Clips verticales reales.
- Vídeos con imágenes o B-roll renderizados mediante FFmpeg.

Medidas de seguridad implementadas:

- No sobrescribe el vídeo original.
- Utiliza archivos temporales y renombrado final.
- Valida el archivo exportado.
- Genera nombres únicos si el destino existe.
- Rechaza una exportación cuya entrada y salida sean iguales.

### 3.3 Shorts

- Obtención de candidatos.
- Scoring basado en hooks y contenido hablado.
- Parsing de SRT y VTT.
- Generación de títulos.
- Eliminación de candidatos solapados.
- Preselección.
- Clasificación humana.
- Preview 9:16.
- Crop, blur y encuadre manual.
- Exportación vertical.

### 3.4 Biblioteca visual y B-roll

- Biblioteca independiente de un vídeo abierto.
- Base SQLite local.
- Importación individual y por carpeta.
- Hash SHA-256 para evitar duplicados.
- Miniaturas.
- Conceptos y etiquetas.
- Historial de uso.
- Detección de archivos faltantes.
- Búsqueda y ranking explicable.
- Colocación manual en timeline.
- Modos completo, parcial, flotante y lower third.
- Plan visual separado del EDL.
- Cola de generación y revisión humana.
- Worker residente y scheduler.
- Proveedor mock local.
- OmniRoute opcional.
- Adaptador experimental para Pollinations.
- Render final con overlays FFmpeg.

La separación Biblioteca/B-roll es adecuada: la Biblioteca administra activos y B-roll decide cómo utilizarlos dentro de un vídeo.

### 3.5 Automatización

- Procesamiento batch.
- Carpeta inbox vigilada.
- Carpeta outbox.
- CLI para analizar, exportar y procesar lotes.
- CLI visual para importar, transcribir, enriquecer y renderizar.
- Cache de audio por hash.

## 4. Arquitectura y tecnologías

### 4.1 Frontend

- Svelte 5.
- TypeScript.
- Vite 6.
- Tailwind CSS.
- PostCSS.
- Autoprefixer.
- API de Tauri.
- Plugins Tauri de diálogo, filesystem, shell y almacenamiento.

Existen componentes y stores grandes, especialmente `VisualWorkspace.svelte`, `ImageGenerationPanel.svelte`, `project.svelte.ts`, `tauri.ts` y `VideoPreview.svelte`. Esto demuestra una cantidad importante de funcionalidad, pero también una concentración de responsabilidades que aumenta el riesgo de regresiones.

### 4.2 Backend

- Rust 2021.
- Tauri 2.
- Tokio para concurrencia.
- Serde y serde_json.
- anyhow y thiserror.
- UUID y Chrono.
- rusqlite con SQLite embebido.
- ONNX Runtime mediante `ort`.
- reqwest con Rustls.
- `image` para JPEG, PNG y WebP.
- SHA-256.
- regex, walkdir, dirs, tracing y base64.

El backend se divide en comandos Tauri, modelos, análisis, detectores, políticas, EDL, TimeMap, exportación, clipping, biblioteca visual, generación, sincronización y CLI.

La arquitectura del motor es más madura que la del frontend. La nueva biblioteca visual ya separa dominio, aplicación e infraestructura, aunque todavía convive con parte del código visual anterior.

### 4.3 Dependencias operativas

- **FFmpeg y FFprobe:** esenciales.
- **Silero VAD ONNX:** opcional, con fallback.
- **Whisper CLI:** opcional y dependiente del PATH.
- **SQLite:** almacenamiento local real.
- **OmniRoute:** opcional.
- **Pollinations:** experimental y protegido por controles de coste.
- **Supabase:** opcional y desactivado por defecto.
- **Codecov, Dependabot y CodeRabbit:** herramientas del repositorio.
- **Sentry:** solamente documentado como decisión arquitectónica; no instalado.

## 5. Resultado de las verificaciones

### 5.1 Frontend

Resultado de `npm.cmd run check`:

- 0 errores.
- 1 advertencia de accesibilidad.

La advertencia se encuentra en `ExportSuccess.svelte`: un elemento con `role="dialog"` no tiene `tabindex`.

El build Vite no pudo verificarse completamente durante la auditoría porque esbuild intentó leer un directorio superior restringido por el entorno. Esto parece una limitación de permisos del entorno de revisión y no demuestra por sí solo un defecto de la aplicación.

### 5.2 Backend Rust

`cargo fmt --check` terminó correctamente.

La suite contiene 105 pruebas unitarias. La mayoría de las pruebas observadas pasó, pero la ejecución completa superó el límite de 180 segundos y dos pruebas fallaron:

- `daily_then_video_reuses`.
- `search_before_generate_and_mock_pipeline`.

Las dos fallaron al guardar el plan visual:

```text
IO error: Acceso denegado. (os error 5)
```

La causa inmediata es de filesystem o permisos. También revela que el entorno temporal de estas pruebas no está suficientemente aislado o utiliza una ruta que puede no ser escribible.

Varias pruebas tardaron más de 60 segundos, por lo que también existe un problema de rendimiento en la suite.

## 6. Problemas y riesgos principales

### 6.1 Prioridad alta

1. **No existen E2E reales de interfaz.**

   No se prueban automáticamente los clics en Tauri, el drag de B-roll, el preview visual, los modales, la navegación, el estado posterior a exportar ni posibles bloqueos de la WebView.

2. **Los E2E de CI pueden fallar sin bloquear una entrega.**

   El job `e2e` utiliza `continue-on-error: true`. Una regresión integral puede quedar registrada sin impedir que CI finalice.

3. **Preview y exportación todavía pueden divergir.**

   Existe un contrato geométrico compartido y pruebas parciales, pero falta validar de extremo a extremo la paridad entre el overlay HTML y el resultado FFmpeg.

4. **Rutas temporales poco robustas en pruebas.**

   Dos pruebas actuales fallan por acceso denegado. Deben utilizar una carpeta temporal explícitamente escribible y hacer una limpieza determinista.

5. **TimeMap incompleto en la experiencia visual.**

   El backend posee TimeMap, pero la UI aún mezcla en algunos recorridos el tiempo original y el tiempo de salida. Esto puede desalinear imágenes, transcript y vídeo exportado.

### 6.2 Prioridad media

6. **Dependencias externas de instalación.**

   Whisper depende del PATH, mientras que Silero, FFmpeg y los modelos requieren preparación adicional. Esto dificulta una experiencia de instalación simple.

7. **Archivos frontend demasiado grandes.**

   Los stores y componentes principales deberían dividirse por responsabilidad.

8. **Código visual legacy coexistente.**

   Componentes anteriores conviven con el nuevo modelo Biblioteca/B-roll, aumentando la deuda y la confusión sobre la fuente de verdad.

9. **Cancelación y progreso incompletos.**

   La cancelación durante VAD puede ser lenta, falta progreso FFmpeg basado en `time=` y la sesión ONNX podría reutilizarse mejor.

10. **Dos flujos de shorts conceptualmente cercanos.**

    Los artefactos de shorts del pipeline y el workspace de clipping pueden resultar confusos para el usuario.

11. **Documentación parcialmente desactualizada.**

    La versión continúa en 1.1.0, el roadmap menciona trabajo v1.2 terminado y el informe QA anterior registra menos pruebas que las existentes actualmente.

## 7. Supabase y seguridad

Supabase está desactivado por defecto y la aplicación local no depende de esta integración. El cliente evita utilizar claves secretas en la aplicación de escritorio.

Aspectos positivos de la migración:

- RLS habilitado en las tablas públicas.
- Políticas basadas en `auth.uid()` y ownership.
- Uso de `USING` y `WITH CHECK` en varias operaciones.
- Bucket configurado como privado.

Bloqueos antes de producción:

- Existe una función pública `SECURITY DEFINER` que debe revisarse y restringirse.
- Las políticas de Storage permiten operar a cualquier usuario autenticado dentro del bucket, sin ownership por carpeta.
- No se han verificado migraciones, advisors o políticas contra un proyecto Supabase real.
- La sincronización requiere la activación explícita de `VIGILCUT_SUPABASE_RLS_VERIFIED=1`.

La integración debe considerarse experimental y bloqueada para producción.

## 8. Calidad y cobertura

### Fortalezas

- Buen volumen de pruebas Rust.
- Pruebas de EDL, políticas y rutas seguras.
- Pruebas de clipping.
- Pruebas de TimeMap.
- Pruebas de biblioteca, generación y supervisión.
- Smoke tests con FFmpeg.
- E2E del motor para export y batch.
- Formatting y Clippy incluidos en CI.
- Typecheck Svelte.
- Cobertura configurada mediante Codecov.

### Debilidades

- Cobertura Rust global documentada: aproximadamente 44,67%.
- Cobertura de rutas visuales documentada: aproximadamente 59,44%.
- Sin pruebas unitarias frontend relevantes.
- Sin E2E de UI.
- Whisper y Silero real no se prueban en CI.
- Codecov todavía no ha sido validado remotamente.
- CodeRabbit no está autorizado.
- Los E2E no bloquean CI.

## 9. Estado por área

| Área | Estado |
|---|---|
| Motor de silencios y EDL | Avanzado y bastante sólido |
| Seguridad de exportación | Buena |
| Batch y CLI | Funcionales |
| Shorts 9:16 | Funcional; necesita mejoras de calidad y rendimiento |
| Biblioteca local | Funcional y bien encaminada |
| B-roll supervisado | Beta avanzada |
| Generación de imágenes | Mock funcional; proveedores reales experimentales |
| Supabase | Preparación local; no apto para producción |
| UX general | Funcional; aún inestable en recorridos complejos |
| Automatización QA | Buena en backend; insuficiente en UI |
| Preparación para usuario final no técnico | Parcial |

## 10. Orden recomendado de trabajo

1. Corregir las dos pruebas que fallan por rutas y permisos.
2. Hacer que los E2E bloqueen CI.
3. Crear un smoke E2E mínimo de la interfaz.
4. Garantizar la paridad preview/export de B-roll.
5. Unificar completamente tiempo source/output en la UI.
6. Separar los componentes frontend más grandes.
7. Eliminar o archivar componentes visuales legacy.
8. Mejorar cancelación, progreso FFmpeg y reutilización ONNX.
9. Resolver la relación entre los dos flujos de shorts.
10. Actualizar README, QA, roadmap y versión.
11. Endurecer Supabase antes de cualquier activación real.
12. Construir y probar el instalador en una máquina Windows limpia.

## 11. Veredicto

VigilCut tiene una base técnica seria y una visión de producto coherente. No es un prototipo vacío: existe un motor considerable y funciones reales de procesamiento, exportación, clipping y biblioteca visual.

El motor está más cerca de producción que la interfaz. La fase adecuada ahora es estabilizar y simplificar, en lugar de continuar agregando integraciones. El objetivo inmediato debería ser que el recorrido completo —abrir vídeo, analizar, supervisar, añadir B-roll y exportar— sea predecible, fácil de instalar y verificable de principio a fin.

