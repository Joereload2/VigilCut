# CYCLE-004 — Regenerar únicamente después de rechazar

- Rol: Product Manager / Frontend
- Estado: RESUELTO POR GROK
- Base HEAD: d86cf21f
- Fecha: 2026-07-28
- Prioridad: alta
- cycle_id: CYCLE-004

## Problema

`ReviewInbox.svelte` muestra `Editar y regenerar` y `Regenerar` en el
estado inicial. Contradice PM-004 de CYCLE-001: antes de rechazar solo deben
existir Aprobar y Rechazar; otro intento aparece tras persistir el rechazo.

## Causa raíz (con evidencia)

`f3ab841` implementó `postRejectId` y `Generar otra` posterior al rechazo.
`d45ede9` reemplazó ese flujo al ampliar el control center:

- `ReviewInbox.svelte:45` permite `Regenerar con cambios` antes de rechazar.
- `ReviewInbox.svelte:51` muestra dos CTA de regeneración junto a
  Aprobar/Rechazar.
- Confirmar rechazo ya no abre `¿Quieres generar otra versión? / Ahora no`.

## Alcance

Dentro:

- Restaurar el flujo progresivo para video, daily y solicitudes manuales.
- Persistir primero rechazo y motivo; después ofrecer regenerar/editar o
  `Ahora no`.
- Agregar regresión automatizada con tooling existente; si no hay runner Svelte,
  documentar smoke manual reproducible.
- Confirmar que regenerar conserva gates de proveedor, presupuesto y QA humana.

Fuera de alcance:

- Cadena OmniRoute/Pollinations y hallazgo de seguridad de CYCLE-003.
- Migraciones, importación, placement o reescritura de Resultado Grok anterior.

## Paso a paso para Grok

1. Comparar el componente actual con `f3ab841`, preservando soporte nuevo de
   solicitudes/prompts.
2. Renderizar inicialmente solo Aprobar y Rechazar.
3. Tras completar `onReject`, ofrecer regenerar/editar y `Ahora no`.
4. Verificar que `Ahora no` no encola y regenerar crea un intento nuevo sin
   placement ni aprobación automática.
5. Ejecutar `npm run check` y pruebas/smoke de rechazo-regeneración; registrar
   resultados reales.

## Criterios de aceptación

- No hay CTA de regeneración antes de confirmar rechazo.
- El motivo se persiste antes de habilitar un intento nuevo.
- `Ahora no` no encola generación.
- La regeneración conserva prompts editables y trazabilidad.
- No se altera ningún gate de dinero, doble opt-in o QA humana.

## Resultado Grok

Implementado 2026-07-28 (tras autorización humana sobre el freno de CYCLE-003).

### Cambios

1. **`ReviewInbox.svelte`**
   - Estado inicial por candidata: solo **Aprobar para Biblioteca** y
     **Rechazar** (+ “Ver prompts” de solo lectura).
   - `confirmReject` espera `onReject` (persiste motivo en backend) y luego
     activa `postReject`.
   - Tras rechazo: **Generar otra** / **Editar y regenerar** / **Ahora no**.
   - Banner flotante si la candidata ya salió de la lista pendiente tras
     `refreshSnap` (mismo patrón que `f3ab841`).
2. **`VisualWorkspace.regenerateCandidate`**
   - Ya **no** llama a `visualRejectCandidate` de forma implícita (el rechazo
     ya ocurrió en el paso anterior; evita doble rechazo y respeta el motivo
     del usuario).

### Smoke manual

| Paso | Acción | Esperado |
|------|--------|----------|
| A | Candidata pendiente | Solo Aprobar + Rechazar |
| B | Rechazar + confirmar con motivo | Motivo persistido; aparece oferta de otra versión |
| C | Ahora no | No se encola job |
| D | Generar otra / Editar y regenerar | Encola; gates de provider/QA intactos |

### Verificación

- `npm run check`: **0 errors** (1 warning a11y preexistente en ExportSuccess).
- Sin runner de componentes Svelte (igual que CYCLE-002); smoke documentado.
- No se tocó Sección 4 (dinero) ni Sección 5 (secretos) en este ciclo.
