# Codex Library Independence Review

Fecha: 2026-07-23
Rama: `feat/independent-visual-library`
HEAD inicial: `4b6b2da255d6bee73c9d323f226840c715103a49`

## Diagnóstico verificado

- La navegación superior ya exponía Biblioteca sin video y `LibraryWorkspace` cargaba `VisualWorkspace` en modo `libraryOnly`.
- El flujo visible estaba orientado a “completar conceptos”, pedía cantidad y prioridad, y no presentaba el modelo mental simple de crear una imagen.
- `library_requests` era persistente y no exigía `mediaPath`, pero carecía de tema, descripción, prompt editable, estilo, dimensiones, origen y marca explícita de búsqueda.
- La búsqueda previa solo devolvía conteos; no mostraba coincidencias ni miniaturas.
- El worker residente y la cola eran reutilizables, pero toda generación usaba 1280×720 aunque se solicitara 9:16 o 1:1.
- Los candidatos sí requerían aprobación humana por defecto y la aprobación era idempotente, pero la bandeja no mostraba prompts ni estrategia del proveedor.
- Mock y coste no verificado ya tenían distinciones de dominio; la nueva UI debía hacerlas más visibles.
- La actualización global de candidatos no observaba necesidades manuales y podía dejar “Por revisar” desactualizado sin un refresco residente de la vista.
- La importación manual asumía licencia propia y uso comercial; eso no estaba demostrado.

## Decisión

Mantener SQLite, `VisualNeed` como adaptador interno transitorio y el supervisor residente. Introducir semántica propia en `LibraryImageRequest` (tabla compatible `library_requests`), búsqueda manual explícita, prompts persistentes y una UI centrada en Biblioteca. B-roll conserva sus contratos y sus reglas automáticas más estrictas.
