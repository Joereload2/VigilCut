# Codex Library/Factory Implementation Report

## Implementado en esta fase

La separación de interfaz de B-roll fue endurecida: el picker ya no genera candidatos ni importa imágenes como efecto lateral. Busca assets, permite seleccionar uno aprobado, omitir la necesidad y navegar a Biblioteca. Biblioteca conserva el flujo independiente de solicitud, búsqueda previa, generación mock, revisión humana e ingestión idempotente.

Se añadieron contratos documentales para dominios, Smart Search y Theme Factory. La fábrica temática queda especificada, no falsamente declarada como implementada.

## Verificación

- `npm run check`: 0 errores; queda una advertencia preexistente en `ExportSuccess.svelte`.
- `git diff --check`: requerido antes del commit.
- Las pruebas visuales previas cubren Biblioteca sin vídeo, aprobación, rechazo, regeneración y deduplicación.
- No se ejecutó OmniRoute real ni proveedor de texto por ausencia de credenciales/autorización.

## Estado

- Biblioteca: funcional local-first.
- Smart Search: existente como matching textual con contrato documentado.
- B-roll: selección/placement de assets aprobados; generación retirada de la UI.
- Fábrica: especificación y límites definidos; plan temático con aprobación aún pendiente.
- Costes: mock/local permitido; rutas pagadas o coste desconocido bloqueadas por política.

## Próximos commits recomendados

1. Crear `UncoveredVisualNeed` pasivo y probar que no encola.
2. Extraer `SmartSearch` como interfaz de solo lectura.
3. Persistir `LibraryTheme`, propuestas y `ThemeGenerationPlan` con aprobación transaccional.
4. Añadir UI de Fábrica y pruebas de no-generación antes de aprobación.
5. Extraer físicamente el adaptador B-roll sin cambiar contratos ni datos.
