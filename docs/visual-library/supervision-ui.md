# Panel «Imágenes IA» — supervisión de generación

## Cómo probar

1. `npm run dev:win`
2. Abre un video y analiza silencios (opcional para daily).
3. Modo **Visual** → pestaña **Imágenes IA**.
4. **Detectar necesidades** (con video).
5. Elige una tarjeta «Falta una imagen» → **Generar imagen**.
   - **Mock = simulación (no es IA)**. Badge visible en preview.
   - Encola y retorna al instante; el **supervisor Rust** procesa la cola (no requiere polling de worker desde la UI).
6. Si aparece «Necesita tu revisión» → preview + **Aprobar y usar** (colocar exige EDL real; no se inventan 60 s).
7. **Doble clic** en aprobar no duplica el asset. Licencia por defecto: `unknown`, `commercial_use=false`.
8. **Rechazar** / **Regenerar** / **Cancelar** (en curso → «Cancelando…»).
9. **Biblioteca automática (daily)** funciona **sin video**: solo mock local o capability `free_verified` (nunca solo `free_configured`).

## Estados (etiquetas ES)

| Estado UI | Significado | Acción principal |
|-----------|-------------|------------------|
| uncovered | Falta una imagen | Generar |
| queued | Esperando turno | Cancelar |
| processing | Generando / etapa real | Cancelar |
| cancelling | Cancelación en curso | Esperar |
| needs_human_review | Necesita tu revisión | Aprobar / Regenerar / Rechazar |
| approved | Lista / cubierta | Ver en plan |
| failed | No se pudo generar | Reintentar |
| cancelled | Cancelada | Generar de nuevo |
| skipped | Sin imagen (usuario) | Generar |

## Arquitectura operativa

- Supervisor residente al arrancar Tauri (`generation::supervisor::ensure_started`).
- Claim atómico + lease; `recover_stale_running` al startup.
- UI: snapshot poll solo lectura (`visual_supervision` / `visual_supervision_global`).
- `visual_worker_tick` queda para CLI/debug, no para el panel.

## Coste

Nunca se muestra solo «Gratis». Etiquetas: verificado / configurado no verificado / simulación mock / pagado / desconocido.

## OmniRoute

- `negative_prompt` en el body + plegado en el prompt positivo.
- Descarga con tope en streaming, redirects manuales revalidados, SSRF (URL + DNS + IPs privadas/CGNAT/IPv6).
- Extensión png/jpg/webp según magic bytes.
- Probe de `/models` **no** marca `supports_image=true` ni `free_verified`.

## Supabase

**Fuera de alcance** de esta entrega. Ver `docs/visual-library/database.md`.
