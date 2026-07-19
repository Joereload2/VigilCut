# VigilCut — Mandato del Arquitecto Principal

**Vigencia:** esta sesión y siguientes  
**Naturaleza:** herramienta interna de fábrica de contenido IA (no comercial)  
**Norte de producto:** minimizar tiempo `grabar → materiales listos para publicar`  
**Filosofía:** *La IA trabaja. El humano únicamente supervisa.*

---

## 1. Qué es el producto (y qué no es)

| Es | No es |
|----|--------|
| Motor de comprensión de vídeo | Editor NLE comercial |
| Pipeline fábrica: Events → Policy → EDL → Artifacts | Timeline creativa libre |
| Cola de excepciones de baja confianza | UI que obliga a decidir cada tramo |
| CLI + batch + watch + desktop thin client | SaaS, onboarding, marketing |

**Verdad del sistema (canónica, 5 años):**

```
Media → Features (cache) → Events (L1) → Policies (L2) → EDL (L3) → Renders / Artifacts
```

- **Detectors** solo emiten `Event` (evidencia). Nunca deciden corte.
- **Policies** convierten evidencia en `EditOp` + `ExceptionItem`.
- **EDL** es la proyección exportable (keep ranges).
- **Segment** en UI es **vista derivada**, no el modelo canónico.
- **Desktop** es un cliente de supervisión, no el centro de gravedad.

Si una feature se implementa como “otro panel de editor”, se rechaza.

---

## 2. Dónde estamos (honesto, no el documento de ayer)

### Ya existe (salvable)

- Stack local: Tauri 2 + Rust + FFmpeg + Silero ONNX (opcional Whisper CLI).
- Motor `pipeline/engine.rs`: silence/silero → events → policy → EDL → segments.
- Detectores: silence, silero, breath (heurística), filler (SRT), structure (chapters/shorts).
- Export multi-corte (`select`/`aselect`), preview skip-cuts, batch + inbox watch.
- Artefactos: MP4 principal + `chapters.txt` + carpeta `*-meta/` (JSON/EDL).
- CLI `vigilcut-cli`.

### Deuda estructural (actualizado post-P0)

1. ~~Export Segment-first~~ → **EDL / keep-ranges first** (`export_keep_ranges`, batch usa `export_from_edl`).
2. ~~Policies ad-hoc en engine~~ → **`pipeline/policy.rs`** registry por `event_type` (Silence + Filler).
3. ~~Detectors sueltos~~ → **`Detector` trait + `run_secondary`** (breath, chapters, shorts, filler/srt).
4. **UI fábrica** — ExceptionQueue al frente; ActionBar prioriza Oír/Exportar; K/X solo con excepciones; SidePanel = stats + pendientes; timeline diagnóstico.
5. **Confianza** — umbral `autoApproveMinScore` en policy/presets/UI; sin feedback loop de correcciones (P2).
6. **Tests** — 11 unit tests engine/policy/export; faltan integration con media real.
7. **Segment legacy** — proyección UI; export EDL-first; event types unificados en `models/event.rs`.
8. **CLI** — `analyze` / `export` / `batch` con `--policy`; export headless acepta out path.

### Veredicto

> El **esqueleto Event→Policy→EDL está bien**. El **centro de gravedad del producto sigue demasiado en la UI de tramos**.  
> El riesgo a 5 años no es “falta de detectores”: es **duplicar lógica por cada detector** y **dejar que Segment vuelva a ser la verdad**.

---

## 3. Desafíos al mandato del propietario (obligatorios)

No se acepta el brief sin contrapeso:

1. **“Todo automático si hay alta confianza”**  
   Sin calibración y sin métrica de error, la auto-decisión **destruye confianza de fábrica más rápido que un clic**.  
   Regla: auto solo con (a) umbral por policy pack, (b) telemetría local de correcciones, (c) modo “solo excepciones” por defecto.

2. **“Cualquier evento útil”**  
   Sin taxonomía cerrada + payload schema por tipo, el event bus se convierte en basurero JSON.  
   Regla: nuevos `event_type` requieren nombre namespaced + schema de payload documentado en un solo sitio.

3. **“No es un editor”**  
   Correcto como metáfora de producto. Incorrecto como “cero superficie de verificación”.  
   La Exception Queue **es** el producto UI. El timeline es opcional de diagnóstico, no el flujo principal.

4. **Reescritura total ahora**  
   Rechazada. El stack y el pipeline actual son el vehículo correcto.  
   Se **evoluciona el dominio** (EDL-first, detector trait, policy registry) sin reescribir Tauri/Svelte.

---

## 4. Protocolo de trabajo (sesión)

Para cada tarea, sin pedir permiso intermedio:

`Plan → Implement → Compile → Fix → Test → Fix → Run app → Smoke flow → Review (como otro dev) → Fix objective debt → Re-compile/test → Stop solo si: (1) sin mejoras objetivas, (2) decisión de producto del dueño, (3) límite externo, (4) falta información no deducible.`

Autorizado sin consulta: refactor, renames, splits, delete dead code, tests, UX de menos clics, perf, docs — **sin cambiar comportamiento esperado**.

Cambios de comportamiento de fábrica (umbrales, qué se auto-corta, qué artefactos salen) se tratan como **policy**, no como whim de UI.

---

## 5. Principios de diseño no negociables

1. **Un motor, muchos detectores** — mismo `Event`, misma feature cache, misma policy pipeline.
2. **Detectors no conocen export** — export solo ve EDL / keep ranges.
3. **CLI y batch son ciudadanos de primera** — si solo funciona en GUI, está mal.
4. **Artefacto principal = MP4 (+ chapters.txt)** — JSON en `*-meta/`, nunca como entrega principal.
5. **Cero ventanas de consola** — procesos hijos ocultos en Windows.
6. **Menos clics que el día anterior** — cada control nuevo debe matar ≥1 decisión humana o morir.
7. **Pregunta CTO de 5 años** antes de merge mental: *¿lo implementaría igual si lo mantengo yo?* Si no, rehacer.

---

## 6. Roadmap de arquitectura (orden de verdad, no de hype)

| Prioridad | Trabajo | Por qué |
|-----------|---------|---------|
| P0 | EDL-first en export (dejar de depender de Segment como fuente de verdad) | Cierra la mentira del dominio |
| P0 | Trait `Detector` + registry + features cache única | Escala a 15 detectores sin copiar |
| P0 | Policy registry por `event_type` + policy packs como única config | Automatización real |
| P1 | UI default = Exception Queue + “Oír resultado” + Export; timeline colapsado | Menos pensamiento |
| P1 | Tests engine: policy, merge ranges, export filter, signal-15 recovery | Estabilidad fábrica |
| P2 | Feedback de excepciones → scores (local) | Confianza calibrada |
| P2 | Más detectores (emotion, CTA, names) **solo** vía trait | Visión sin caos |
| P3 | API local / jobs folder-watch maduro | Escala de carpeta, no de app |

---

## 7. Criterio de parada por tarea

No se declara “listo” por “compila”. Listo cuando:

- el flujo fábrica (abrir → analizar → excepciones → export MP4+meta) es estable,
- no hay deuda objetiva barata sin arreglar,
- y un CTO de 5 años firmaría el diff.

---

*Documento vivo. Actualizar cuando el dominio canónico o las P0 cambien.*
