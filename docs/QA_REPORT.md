# VigilCut — Informe de QA (2026-07-22)

**Versión:** 1.1.0  
**Entorno:** Windows · Rust cargo · FFmpeg sidecar · svelte-check  
**Alcance:** motor (unit/smoke/e2e) + frontend typecheck + revisión de arquitectura y UX reciente  

---

## 1. Resumen ejecutivo

| Capa | Resultado | Detalle |
|------|-----------|---------|
| **Unit (`cargo test --lib`)** | **PASS** | **64** tests, 0 fallos |
| **Smoke pipeline** | **PASS** | 2 tests |
| **Smoke clipping** | **PASS** | 2 tests |
| **Smoke visual** | **PASS** | 2 tests |
| **E2E factory** | **PASS** | 2 tests (export + batch) |
| **E2E clipping** | **PASS** | 1 test (export 9:16) |
| **Frontend (`svelte-check`)** | **PASS** | 0 errors (1 warning a11y menor) |
| **E2E UI Tauri/WebView** | **NO EXISTE** | Sin Playwright/Cypress/WebDriver |
| **Total motor** | **73** tests OK | Tras fix de compilación en smoke/e2e |

**Conclusión:** el **motor** (Events → Policy → EDL → Export / Clipping / Visual render) está en buen estado de regresión automatizada. La **UI** no tiene E2E de clics; la calidad UX se valida manualmente y por typecheck.

---

## 2. Pirámide de tests (inventario)

### 2.1 Unit (lib)

Cubren sin media real o con fixtures mínimos:

- **Policy / EDL / engine** — auto-cut, excepciones, recompile  
- **Export** — keep ranges, merge, filtros FFmpeg, audio enhance chain  
- **TimeMap** — source↔output, spans, roundtrip  
- **Safe paths** — no overwrite del original, unique path, exports inválidos  
- **Clipping** — score, preselect, framing, titles, SRT parse, dedupe  
- **Detectors** — fillers, structure/chapters  
- **Visual** — match_rank, library (import/SHA/usage), compose (overlap/snap)  
- **Transcript / semantic** — SRT/TXT, conceptos  

### 2.2 Smoke (FFmpeg + sintético)

| Test | Qué valida |
|------|------------|
| `smoke_analyze_synthetic_video…` | Análisis silencios → eventos/EDL/segmentos |
| `smoke_policy_auto_cut…` | Auto-cuts en silencio sintético |
| `smoke_clipping_with_srt…` | Candidatos de shorts con SRT |
| `smoke_sidecar_srt…` | Detección SRT al lado del vídeo |
| `smoke_visual_overlay…` | Import imagen → placement → encode overlay + manifiesto |
| `smoke_visual_refuses_empty_plan` | Plan vacío no renderiza en silencio |

### 2.3 E2E motor (no GUI)

| Test | Qué valida |
|------|------------|
| `e2e_export_mp4_and_meta_folder_layout` | MP4 editado + carpeta `*-meta` con artefactos |
| `e2e_batch_process_one_file` | Worker batch → outbox |
| `e2e_clipping_finds_and_exports_vertical` | Clip 9:16 exportado |

### 2.4 Lo que **no** se automatiza aún

- Clics en Tauri (modos Silencios / Shorts / Visual)  
- Drag de B-roll en timeline / overlay live  
- Whisper / Silero ONNX en CI  
- Abrir carpeta post-export, modal de éxito, hang de UI  
- Responsive de layout 70/30  

---

## 3. Fallo detectado y corregido en esta corrida

### Compilación de tests (`prefer_whisper`)

**Síntoma:** `smoke_pipeline` y `e2e_factory` no compilaban:

```text
error[E0063]: missing field `prefer_whisper` in initializer of `PolicyConfig`
```

**Causa:** se añadió `prefer_whisper` al struct y los tests antiguos no lo inicializaban.

**Fix aplicado:** `prefer_whisper: false` en:

- `src-tauri/tests/smoke_pipeline.rs` (2 sitios)  
- `src-tauri/tests/e2e_factory.rs` (1 sitio)  

**Mejora de harness:** `scripts/test-all.ps1` ahora incluye también `smoke_visual` (antes se saltaba en el script “todo”).

---

## 4. Mapa de funciones de VigilCut

### 4.1 Filosofía de producto

> **La IA trabaja. El humano supervisa excepciones.**

Cadena canónica:

```text
Media → Features → Events → Policy → EDL → Export / Artifacts
```

Paralelo visual (no muta EDL):

```text
Transcript → SemanticEvent → Library match → VisualPlan → Live preview / FFmpeg bake
```

### 4.2 Modo Silencios

| Función | Estado motor | Estado UI | Riesgo |
|---------|--------------|-----------|--------|
| Abrir / probe media | OK (tests + uso) | OK | Bajo |
| Detectar silencios (FFmpeg / Silero) | OK smoke | OK | Medio (Silero no en CI) |
| Policy auto-cut vs excepciones | OK unit+smoke | Cola excepciones | Bajo motor / Medio UX |
| Timeline keep/cut proporcional | Parcial (export sí) | Timeline 100% ancho | Medio (mapping source clock) |
| Preview cortado (skip cuts) | Lógica store | VideoPreview | Medio |
| Export MP4 + meta | OK e2e | Modal éxito | Medio (busy/modal — mitigado) |
| Batch / inbox | OK e2e batch | BatchPanel | Medio |

### 4.3 Modo Shorts 9:16

| Función | Estado motor | Estado UI | Riesgo |
|---------|--------------|-----------|--------|
| Generar candidatos (score/hooks) | OK unit+smoke | ClippingPanel | Medio |
| Preselect / dedupe | OK unit | UI clasificar | Medio |
| Framing / blur / crop | OK unit | ShortPlayer | Medio |
| Export vertical | OK e2e | Panel export | Bajo-Medio |
| Títulos / transcript | OK unit | Parcial UI | Medio |

### 4.4 Modo Visual / B-roll

| Función | Estado motor | Estado UI | Riesgo |
|---------|--------------|-----------|--------|
| Biblioteca SQLite + SHA | OK unit | Lista en tools | Bajo |
| Placement manual | OK smoke | Colocar form | Medio |
| Live overlay (preview) | No smoke de “live HTML” | VisualLiveOverlay | **Alto** (desalineación preview vs bake) |
| Supervised timeline | compose unit (snap/overlap) | SupervisedTimeline | Medio |
| Zonas espaciales / issues | compose evaluate | Parcial (zonas demo quitadas) | Medio-Alto |
| Render FFmpeg overlay | OK smoke | Export con imágenes | Medio |
| Inicio → Fin +1s | N/A | VisualPlaceForm | Bajo (reciente) |
| Layout 70/30 + tools | N/A | VisualPanel | Medio (regresiones de build/UI) |

### 4.5 Infra transversal

| Función | Estado |
|---------|--------|
| FFmpeg sidecar | OK (requerido smoke) |
| CLI `vigilcut-cli` | Existe; no corrido exhaustivo en esta sesión |
| Safe paths / no tocar original | OK unit + smoke visual |
| Progress events | UI depende; no test e2e |
| Whisper CLI | Manual / PATH; no CI |

---

## 5. Fallas y riesgos conocidos (producto + sesión reciente)

### Críticos / altos (producto)

1. **Sin E2E de UI**  
   Layout, drag B-roll, export modal, hang post-export no se capturan en CI.

2. **Preview live ≠ bake FFmpeg** — **mitigado (2026-07-22)**  
   Contrato `center_norm_v1` en `pipeline/visual/layout.rs` + mirror TS + render FFmpeg + tests de paridad CSS/px. Ver `docs/VISUAL_LAYOUT_CONTRACT.md`.

3. **TimeMap UI incompleto en Visual**  
   Playhead source vs output; transcript en source mapeado a output de forma aproximada; fricciones de supervisión.

4. **Builds release / proceso colgado**  
   Usuarios pueden seguir viendo UI antigua si el `.exe` no se reemplaza (file lock). Mitigación: kill + rebuild limpio documentado.

### Medios

5. **Zonas protegidas espaciales**  
   Detección real de rostros/subtítulos no implementada (heurísticas / zonas manuales).

6. **Whisper / Silero**  
   Dependencia de entorno; CI evita Silero para determinismo.

7. **Busy / modal export**  
   Mitigado en código reciente (status visible, clear busy); conviene smoke/manual checklist.

8. **`test-all.ps1` incompleto (antes)**  
   No corría `smoke_visual` — ya corregido en esta sesión.

9. **Componentes legacy de UI**  
   `ModeNav`, `PlacementInspector`, `VisualTrack`, `CompositionInspector`, `AuxTabShell` pueden coexistir con el layout nuevo sin usarse → deuda y confusión.

### Bajos

10. Warnings a11y Svelte (dialog tabindex).  
11. Linker warning Windows en build.  

---

## 6. Partes mejorables (priorizadas)

### P0 — Calidad / confianza

| Mejora | Por qué |
|--------|---------|
| **E2E UI mínimo** (Playwright + Tauri o script de humo manual checklist en CI) | Cubre layout, export, B-roll place |
| **Contrato único layout preview/export** | Misma geometría normalizada 0–1 → CSS y FFmpeg |
| **Test de regresión PolicyConfig** con `..Default` en fixtures | Evita roturas por campos nuevos |
| **Smoke post-export UI state** | busy=false, statusMessage, dismiss modal |

### P1 — Producto supervisión

| Mejora | Por qué |
|--------|---------|
| Cola de excepciones visuales en el panel derecho | Alineado a “solo excepciones” |
| Snap magnético real a palabras del transcript en output | Ya hay anclas en compose; unir a drag UI |
| Detección real de face/subtitle o desactivar claims | Honestidad del producto |
| Persistencia VisualPlan con run/EDL fingerprint en UI | Invalidar si cambian cortes |

### P2 — UX / arquitectura UI

| Mejora | Por qué |
|--------|---------|
| Eliminar o archivar componentes no usados | Menos deuda |
| Un solo shell de layout (silencios/shorts/visual) | Consistencia |
| Panel derecho tools: estados vacíos mejores + atajos | Onboarding |
| Telemetría local de errores (log file) | Debug post-export hang |

### P3 — Fábrica / escala

| Mejora | Por qué |
|--------|---------|
| Batch visual (overlay pack) | Cierra fábrica multi-artefacto |
| CLI e2e en CI (analyze + export + visual) | Sin GUI |
| Benchmarks de encode | CRF/preset tradeoffs |

---

## 7. Checklist de QA manual (recomendado hasta tener E2E UI)

### Silencios

- [ ] Abrir MP4 real  
- [ ] Ver auto-cuts + excepciones  
- [ ] Oír video cortado  
- [ ] Exportar → modal → status bar visible → Seguir editando  
- [ ] Original intacto  

### Shorts

- [ ] Generar candidatos con SRT  
- [ ] Clasificar keep/cut  
- [ ] Export 9:16  

### Visual

- [ ] Layout 70/30 (video+timeline | herramientas)  
- [ ] Tools: botones sin “+ Abrir panel” ni × confuso  
- [ ] Inicio N → Fin N+1 automático  
- [ ] Colocar imagen → aparece en timeline B-roll y en preview en el intervalo  
- [ ] Export con imágenes → modal + status  
- [ ] Archivo `*-con-imagenes*.mp4` junto al original  

### Regresión build

- [ ] Cerrar todos los `vigilcut.exe` antes de rebuild  
- [ ] Lanzar solo `src-tauri/target/release/vigilcut.exe`  

---

## 8. Comandos de regresión

```powershell
# Todo (script)
npm test

# Por capa
npm run test:unit
npm run test:smoke
npm run test:e2e
npm run check   # svelte-check
```

---

## 9. Veredicto QA

| Dimensión | Nota | Comentario |
|-----------|------|------------|
| Motor fábrica | **A** | 73 tests verdes; pipeline sólido |
| Cobertura visual bake | **B+** | Smoke overlay OK; live UI sin test |
| UX / layout | **B-** | Iteración fuerte reciente; falta E2E y estabilidad de release |
| Supervisión B-roll (visión producto) | **C+** | Base de modelo y UI; falta paridad preview/export y zonas reales |
| CI readiness | **B** | Unit/smoke/e2e motor listos; fix prefer_whisper aplicado |

**Recomendación inmediata:**  
1) Merge del fix de tests `prefer_whisper` + `smoke_visual` en `test-all.ps1`.  
2) Checklist manual Visual tras cada release.  
3) Añadir un test de paridad layout (unit: misma función Rust para rect FFmpeg y serialización JSON consumida por el overlay).  

---

*Generado en sesión de QA local · 2026-07-22*
