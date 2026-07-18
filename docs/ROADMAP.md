# Roadmap — VigilCut Factory

## Hecho (v1 / v1.1 factory)

- [x] Engine silence: Events + Policy + EDL  
- [x] Auto-approve alta confianza + cola de excepciones  
- [x] Preview video cortado  
- [x] Export multi-artefacto (mp4, chapters, shorts JSON, events, edl, manifest)  
- [x] **Export real de shorts** (top 5 MP4 en carpeta `*-shorts/`)  
- [x] Batch worker async + UI lote  
- [x] CLI `vigilcut-cli` (analyze / export / batch)  
- [x] Detectores: capítulos, short candidates, breath/micro-pause  
- [x] Feature cache wav 16 kHz por hash  
- [x] **Watch inbox** + procesar inbox ahora  
- [x] Documentación arquitectura / CTO  

## Hecho (v1.2 ML path)

- [x] Silero VAD ONNX real (`ort`, model `silero_vad.onnx`) + fallback FFmpeg  
- [x] Whisper CLI opcional → SRT + detector de muletillas  
- [x] `npm run setup:models`  

## Más adelante

- [ ] Policy packs por canal  
- [ ] GPU / DirectML  
- [ ] Métricas human-seconds / media-minute  
- [ ] LLM local para CTA / frases memorables  
- [ ] whisper.cpp embebido (sin CLI externa)  
