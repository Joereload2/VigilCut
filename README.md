# VigilCut

**ES:** Editor de video de escritorio, local y open-source (MIT), pensado para creadores de contenido.  
**EN:** Local open-source (MIT) desktop video editor for content creators.

> **Human-in-the-loop first.** VigilCut propone cortes (silencios, huecos, candidatos de clip); **tú** revisas el timeline, activas/desactivas segmentos y apruebas antes de exportar.

| | |
|---|---|
| Stack | **Tauri 2** (Rust) · **Svelte 5** + TypeScript + Tailwind · **FFmpeg** sidecar · **Silero VAD** (cuando hay modelo) |
| Licencia | MIT |
| Estado | MVP scaffold — detección de silencios + timeline extensible |

---

## Características / Features

### MVP (v0.1)
- Detección de silencios (FFmpeg `silencedetect`; hook listo para Silero VAD)
- Timeline con segmentos **Keep / Cut / Pending**
- Toggle por segmento, split en playhead, zoom, waveform
- Preview **skip-cuts** (el playhead salta regiones cortadas)
- Export de rangos Keep con FFmpeg `filter_complex`
- Presets integrados (Default, Podcast, YouTube, Gentle, Clip Select)
- Proyectos JSON en el directorio de datos de la app
- Import de subtítulos SRT/VTT

### Arquitectura lista (stubs / partial)
- Mejora de audio (denoise, normalización LUFS, highpass)
- Color / iluminación básica (`eq`)
- Subtítulos auto con Whisper (local, pendiente de modelo)
- Modo preselección de clips
- Cola batch multi-archivo

Ver [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) y [docs/ROADMAP.md](docs/ROADMAP.md).

---

## Requisitos / Prerequisites

| Herramienta | Versión |
|-------------|---------|
| [Node.js](https://nodejs.org/) | 20+ (recomendado 22) |
| [Rust](https://rustup.rs/) | stable 1.77+ |
| FFmpeg + FFprobe | en `PATH` o en `src-tauri/binaries/` |
| OS | Windows 10+, macOS 12+, Linux (WebKitGTK) |

### Windows extra
- [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (para compilar Rust/Tauri)
- WebView2 (suele venir con Windows 11)

### Linux extra
```bash
# Debian/Ubuntu ejemplo
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

---

## Instalación y desarrollo / Setup & development

```bash
# 1. Clonar / entrar al repo
cd VigilCut

# 2. Dependencias JS
npm install

# 3. FFmpeg (copia binarios del sistema a sidecars si existen)
npm run setup:ffmpeg

# 4. Instalar Rust si falta: https://rustup.rs
#    rustup default stable
#    Windows: VS Build Tools con workload "Desktop development with C++"

# 5. App de escritorio (Vite + Tauri)
# Windows (recomendado — activa MSVC + PATH):
npm run dev:win
# o:
npm run tauri:dev
```

### Verificación local / Local checks

```bash
npm run check:all   # frontend build + cargo check + cargo test
```

Estado verificado en esta máquina de desarrollo:

| Componente | Estado |
|------------|--------|
| Node / npm | OK |
| Rust stable 1.97 | OK (`~/.cargo`) |
| VS Build Tools 2022 + MSVC | OK |
| FFmpeg 8.1 + sidecars | OK |
| `cargo check` + unit tests | OK (3 tests) |
| `npm run build` (Vite) | OK |

### Solo frontend (UI demo sin Rust)

```bash
npm run dev
# http://localhost:1420 — modo demo con timeline sintético
```

### Build de producción

```bash
# Windows
.\scripts\build.ps1

# macOS / Linux
chmod +x scripts/build.sh
./scripts/build.sh

# o
npm run tauri:build
```

Artefactos: `src-tauri/target/release/bundle/`

---

## Uso rápido / Quick usage

1. **Abrir** un video (mp4, mov, mkv, webm, …).
2. VigilCut **analiza** silencios y construye segmentos speech/silence.
3. En el **timeline**: clic = seek, doble clic = toggle Keep/Cut.
4. Ajusta **padding**, umbral y preset en el inspector.
5. Activa **Saltar cortes** en el preview para oír el resultado editado.
6. **Exportar** → eliges ruta; FFmpeg concatena solo segmentos Keep.

---

## Estructura del proyecto / Project layout

```
VigilCut/
├── src/                      # Frontend Svelte 5
│   ├── App.svelte
│   ├── main.ts
│   └── lib/
│       ├── components/       # Timeline, Preview, Inspector, …
│       ├── stores/           # project.svelte.ts (runes)
│       ├── types/
│       └── utils/
├── src-tauri/                # Backend Tauri 2 + Rust
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── commands/         # IPC commands
│   │   ├── ffmpeg/           # Sidecar + probe + filters
│   │   ├── pipeline/         # Silence → segments, export
│   │   └── models/
│   ├── binaries/             # ffmpeg / ffprobe (no se versionan)
│   ├── icons/
│   └── tauri.conf.json
├── scripts/
│   ├── setup-ffmpeg.mjs
│   ├── build.ps1
│   └── build.sh
├── docs/
│   ├── ARCHITECTURE.md
│   └── ROADMAP.md
├── package.json
├── LICENSE                   # MIT
└── README.md
```

---

## Comandos Tauri principales / Key IPC commands

| Command | Descripción |
|---------|-------------|
| `probe_media` | Metadatos vía ffprobe |
| `detect_silences` | VAD / silencedetect → segmentos |
| `apply_segment_edits` / `split_segment_at` | Edición de timeline |
| `preview_skip_cuts` | Rangos Keep para el player |
| `export_video` | Render FFmpeg |
| `list_presets` / `save_preset` | Presets |
| `queue_batch_job` | Batch (cola) |
| `import_subtitles` | SRT/VTT |
| `enhance_audio_preview` / `analyze_color_stats` | Filtros futuros |

---

## Presets incluidos

- **Default** — equilibrio para talking-head  
- **Podcast / Interview** — silencios más agresivos + audio enhance  
- **YouTube Talking Head** — color ligero + −14 LUFS  
- **Gentle / Conservador** — solo silencios largos  
- **Clip Select** — no auto-cut; modo preselección  

Los presets de usuario se guardan en el directorio de datos de la app (`presets/*.json`).

---

## Silero VAD y Whisper

Coloca modelos en el directorio de datos de la app:

- Windows: `%APPDATA%\VigilCut\models\`
- macOS: `~/Library/Application Support/VigilCut/models/`
- Linux: `~/.local/share/VigilCut/models/`

| Archivo | Uso |
|---------|-----|
| `silero_vad.onnx` | Detección de voz precisa (cuando se cablee el runner ONNX) |
| Whisper `*.bin` / ggml | Subtítulos auto (comando stub `generate_subtitles_whisper`) |

Sin modelo Silero, el MVP usa **FFmpeg silencedetect** de forma fiable.

---

## Filosofía de producto

1. **Automático propone, humano dispone** — nada se exporta sin revisión.  
2. **100% local** — sin cuenta, sin nube obligatoria, sin telemetría.  
3. **Extensible** — pipelines de audio, color, subtítulos y batch como capas sobre el mismo modelo de segmentos.  
4. **MIT** — úsalo, modifícalo, redistribúyelo.

---

## Scripts npm

| Script | Acción |
|--------|--------|
| `npm run dev` | Vite frontend |
| `npm run tauri:dev` | App completa |
| `npm run tauri:build` | Instalador / binario |
| `npm run setup:ffmpeg` | Preparar sidecars |
| `npm run check` | `svelte-check` |

---

## Contribuir / Contributing

1. Fork + branch (`feat/…`, `fix/…`)  
2. Mantén el enfoque human-in-the-loop  
3. `npm run check` + pruebas Rust (`cargo test` en `src-tauri`)  
4. PR con descripción clara (ES o EN)

---

## Licencia

[MIT](LICENSE) © 2026 VigilCut Contributors

---

## Créditos

- [FFmpeg](https://ffmpeg.org/)  
- [Silero VAD](https://github.com/snakers4/silero-vad)  
- [Tauri](https://tauri.app/) · [Svelte](https://svelte.dev/)  

---

**VigilCut** — *You stay vigilant. The cuts stay yours.*
