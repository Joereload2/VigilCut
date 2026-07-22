# Panel «Imágenes IA» — supervisión de generación

## Cómo probar

1. `npm run dev:win`
2. Abre un video y analiza silencios.
3. Modo **Visual** → pestaña **Imágenes IA**.
4. **Detectar necesidades**.
5. Elige una tarjeta «Falta una imagen» → **Generar imagen** (mock offline).
6. Si aparece «Necesita tu revisión» → preview + **Aprobar y usar**.
7. **Doble clic** en aprobar no duplica el asset.
8. **Rechazar** / **Regenerar** / **Cancelar** (si está en cola).
9. Opcional: activa **Alimentación diaria** (solo gratis/local, con app abierta).

## Estados (etiquetas ES)

| Estado UI | Significado | Acción principal |
|-----------|-------------|------------------|
| uncovered | Falta una imagen | Generar |
| queued | Esperando turno | Cancelar |
| processing | Generando / etapa real | Cancelar |
| needs_human_review | Necesita tu revisión | Aprobar / Regenerar / Rechazar |
| approved | Lista / cubierta | Ver en plan |
| failed | No se pudo generar | Reintentar |
| cancelled | Cancelada | Generar de nuevo |
| skipped | Sin imagen (usuario) | Generar |

## Coste

Nunca se muestra solo «Gratis». Etiquetas: verificado / configurado no verificado / local / pagado / desconocido.

## OmniRoute

- `negative_prompt` en el body + plegado en el prompt positivo.
- Descarga con tope en streaming, redirects limitados, bloqueo SSRF básico.
- Extensión png/jpg/webp según magic bytes.
- Probe de `/models` **no** marca `supports_image=true`.
