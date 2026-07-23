# VigilCut Factory

**ES:** Motor local de postproducción para una **fábrica de contenido con IA**.  
**EN:** Local post-production engine for an **AI content factory**.

> **La IA trabaja. El humano solo supervisa excepciones.**  
> No es un editor NLE: es un pipeline Events → Policy → EDL → Artefactos.

| | |
|---|---|
| Stack | Tauri 2 · Rust · Svelte 5 · TypeScript · Tailwind · FFmpeg |
| Licencia | MIT |
| Estado | **v1.1+ visual** — engine + excepciones + batch + clipping 9:16 + biblioteca visual local + CLI |

Documentación:

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — mapa técnico  
- [docs/ARCHITECTURAL_REVIEW.md](docs/ARCHITECTURAL_REVIEW.md) — mandato CTO / visión 5 años  
- [docs/HARDENING_1_1.md](docs/HARDENING_1_1.md) — endurecimiento 1.1 (seguridad, modos, tests)  
- [docs/VISUAL_LIBRARY_DESIGN.md](docs/VISUAL_LIBRARY_DESIGN.md) — diseño enriquecimiento visual  
- [docs/VISUAL_LIBRARY_IMPLEMENTATION_REPORT.md](docs/VISUAL_LIBRARY_IMPLEMENTATION_REPORT.md) — reporte implementación  
- [docs/visual-library/architecture.md](docs/visual-library/architecture.md) — biblioteca inteligente (conceptos, generación, QA)  
- [docs/visual-library/omniroute.md](docs/visual-library/omniroute.md) — OmniRoute opcional  
- [`.env.example`](.env.example) — variables OmniRoute / costes / Supabase  
- [docs/ROADMAP.md](docs/ROADMAP.md) — roadmap  
- [docs/BACKLOG_NEXT.md](docs/BACKLOG_NEXT.md) — backlog post-MVP  

---

## Qué hace (v1)

1. **Analiza** un vídeo → eventos (`audio.silence`, `audio.speech`, `structure.chapter`, `short.candidate`)  
2. **Política** auto-corta silencios de alta confianza  
3. **Cola de excepciones** solo para casos dudosos  
4. **Preview** del resultado cortado  
5. **Export** multi-artefacto:
   - `*-editado.mp4`
   - `*.json` manifiesto
   - `*.events.json` · `*.edl.json`
   - `*.chapters.json` + `*.chapters.txt` (YouTube)
   - `*.shorts.json` + carpeta `*-shorts/short-01.mp4` … (clips reales)
6. **Batch** + **watch inbox** (auto-procesa crudos)  
7. **CLI** sin UI para automatización  
8. Cache de audio 16 kHz + detectores breath / chapters / shorts  
9. **Modo Visual** — transcripción canónica, biblioteca local de imágenes, sugerencias supervisadas, VisualPlan ≠ EDL, overlay FFmpeg

---

## Arranque rápido

```powershell
cd VigilCut
npm install
npm run setup:ffmpeg
npm run setup:models    # Silero VAD ONNX (~2 MB)
# Opcional: whisper o whisper-cli en PATH → subtítulos + muletillas
# Rust + VS Build Tools en Windows
npm run dev:win
```

### Un vídeo (supervisión)

1. Abrir video  
2. Revisar **Supervisión** (excepciones; el resto ya está auto-cortado)  
3. **Oír video cortado**  
4. **Exportar** → carpeta con pack de artefactos  

### Lote fábrica

- UI: pestaña **Lote** → archivos o inbox  
- **Modo Seguro (default):** cortes claros automáticos; dudas **conservadas** en el export  
- **Supervisado:** no exporta si hay dudas  
- **Agresivo:** corta dudas (pide confirmación en UI; CLI `--aggressive`)  
- CLI:

```powershell
npm run cli -- analyze .\clip.mp4
npm run cli -- export .\clip.mp4
npm run cli -- batch .\inbox .\outbox
# opc. --aggressive en export/batch para forzar cortes dudosos
npm run cli -- visual import .\imagenes --concepts inflacion,economia
npm run cli -- visual transcript .\clip.mp4 .\out
```

Carpetas de app: `%APPDATA%\VigilCut\inbox`, `outbox` y `library` (metadatos SQLite + copias administradas).

---

## Biblioteca Visual independiente

La pestaña **Biblioteca** funciona sin abrir un video. Permite buscar, importar una imagen o carpeta y crear una imagen mediante `+ Nueva imagen`. La creación siempre busca primero en SQLite, muestra alternativas reutilizables y, si el usuario continúa, encola una sola generación para revisión humana.

Con el proveedor mock la interfaz muestra **SIMULACIÓN · NO ES IA REAL**. Ningún candidato generado entra en la Biblioteca antes de aprobarse. OmniRoute es opcional, los proveedores pagados están desactivados por defecto y Supabase no es necesario para el flujo local.

Recorrido: `Biblioteca → Nueva imagen → Buscar coincidencias → Generar → Por revisar → Aprobar para Biblioteca`.

---
## Arquitectura (resumen)

```
Media → Detectors (events) → Policy (edit ops / exceptions)
      → EDL → Render (mp4) + Artifacts (json/txt)
```

- **Segmentos en UI** = proyección legacy, no fuente de verdad  
- **Nuevos detectores** = más `event.type` namespaced (ver `pipeline/detectors/`)  
- Ver mandato en `docs/ARCHITECTURAL_REVIEW.md`  

---

## Scripts

| Script | Uso |
|--------|-----|
| `npm run dev:win` | App desktop (MSVC env) |
| `npm run tauri:dev` | Tauri dev |
| `npm run tauri:build` | Instalador |
| `npm run cli -- …` | CLI fábrica |
| `npm run check` | svelte-check |
| `npm run setup:ffmpeg` | Sidecars FFmpeg |

---

## Requisitos

- Node 20+  
- Rust stable + (Windows) VS Build Tools C++  
- FFmpeg / FFprobe  

---

## Licencia

[MIT](LICENSE) © 2026 VigilCut Contributors
