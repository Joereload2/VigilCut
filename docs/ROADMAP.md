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

## Siguiente (v1.2 — modelos ML)

- [ ] Silero VAD real (ONNX sobre cache wav)  
- [ ] Whisper local → captions SRT  
- [ ] Detector muletillas sobre transcript  
- [ ] Policy packs por canal  

## Más adelante

- [ ] GPU opcional  
- [ ] Métricas human-seconds / media-minute  
- [ ] LLM local para CTA / frases memorables  
