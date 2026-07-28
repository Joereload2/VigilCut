# CYCLE-005 — Scheduler persistente de revisión Codex

- Rol: Operaciones / Automatización
- Estado: RESUELTO POR CODEX
- Base HEAD: `9c04583`
- Fecha: 2026-07-28
- Prioridad: alta
- cycle_id: CYCLE-005

## Problema

El handoff documentaba horarios de Codex, pero no existía una tarea de
Windows verificable. La instalación global de Codex estaba incompleta y las
primeras versiones del ejecutor trataban stderr informativo como fallo o
truncaban el prompt multilínea.

## Implementación

- Se reparó la instalación oficial de `@openai/codex` y se verificó
  `codex-cli 0.145.0`, `codex exec` y autenticación ChatGPT.
- `scripts/run-codex-review-final.ps1` valida los estados acoplados, usa lock
  exclusivo, envía el prompt por stdin y guarda logs fuera del repositorio.
- `scripts/codex-review-prompt.md` contiene la revisión reproducible.
- `scripts/setup-codex-review-task.ps1` instala o elimina la tarea de Windows
  con seis disparadores diarios en horario local.
- La tarea usa una cuenta interactiva limitada, ignora ejecuciones solapadas,
  recupera corridas omitidas y limita cada ejecución a tres horas.

## Seguridad

- No contiene claves, tokens ni credenciales.
- Logs y lock viven en `%LOCALAPPDATA%\VigilCut\agent-runtime`.
- El runner sale sin trabajar si ambos estados están `DETENIDO` y falla de
  forma segura si los estados están desacoplados.
- Ningún agente puede reactivar el circuito; eso corresponde a la persona
  responsable.

## Verificación

- Dry-run correcto.
- Tarea `VigilCut-Codex-Grok-Review` registrada con horarios 00, 04, 08, 12,
  16 y 20 horas.
- Zona del sistema: `Pacific SA Standard Time`, Santiago, con DST.
- Corrida real desde Task Scheduler: resultado `0` y log/final generados.
- Corrida automática de las 12:00: resultado `0`; siguiente ejecución 16:00.

## Rollback

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-codex-review-task.ps1 -Remove
```
