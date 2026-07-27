# User flow — Biblioteca visual inteligente

## Usuario común (sin jerga)

1. Abre un video y analiza silencios.  
2. En modo **Visual** → pestaña **Biblioteca**.  
3. **Detectar necesidades** (usa el texto del video).  
4. Ve el resumen: *Cobertura visual: X de Y*.  
5. **Usar biblioteca** — reutiliza imágenes existentes y las pone en el plan.  
6. Si faltan: **Completar faltantes** (mock offline o OmniRoute gratis; nunca pago por defecto).  
7. Revisa solo los dudosos (OK / No).  
8. Supervisa timeline/preview y **Exportar con imágenes**.

La generación **nunca es obligatoria**. Puedes saltar y seguir con lo reutilizado o sin B-roll.

## Operador / fábrica

- Seed de conceptos: botón **Seed economía** o API `visual_seed_theme_economy`.  
- Worker sin UI: `visual_worker_tick` / futuro CLI.  
- Coste: `visual_cost_policy` — `paid_providers_enabled=false` por defecto.  
- Probe OmniRoute: `visual_probe_image_provider` (no gasta generación de imagen).  
