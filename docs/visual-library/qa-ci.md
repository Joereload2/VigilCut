# QA & CI — Intelligent Visual Library

## Pirámide

| Capa | Comando | Gate |
|------|---------|------|
| Format | `npm run test:fmt` | hard |
| Clippy | `npm run test:clippy` | hard (`-D warnings`) |
| Unit | `npm run test:unit` / `test:unit:visual` | hard |
| Smoke FFmpeg | `smoke_pipeline`, `smoke_clipping`, `smoke_visual` | hard (CI smoke job) |
| Smoke intel | `smoke_visual_intel` | hard (mock, no network) |
| E2E factory/clips | `npm run test:e2e` | soft in CI (`continue-on-error`) |
| Frontend | `npm run check` + `build` | hard |
| Supabase SQL | workflow `supabase-sql` | hard (sanity, no secrets) |

## Casos QA cubiertos

- Match reutiliza asset antes de generar  
- Job mock genera PNG y pasa QA técnico  
- Paid providers bloqueados por defecto  
- Dedupe SHA-256 / phash near-dup  
- Migración SQL con RLS + WITH CHECK  
- `.env.example` sin secretos  

## Casos aún manuales

- OmniRoute real en `localhost:20128`  
- UI Biblioteca (clics WebView)  
- Sync Supabase live  

## Regenerar confianza local

```powershell
npm run test:fmt
npm run test:clippy
npm run test:unit:visual
npm run test:smoke
# opcional: npm test  (test-all.ps1)
```
