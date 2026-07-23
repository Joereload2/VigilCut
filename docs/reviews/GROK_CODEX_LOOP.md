# Bucle Codex ↔ Grok (VigilCut)

## Roles

| Actor | Acción |
|-------|--------|
| **Codex** | Revisa código, escribe un ciclo `PENDIENTE` en `CODEX_TO_GROK.md` |
| **Grok** | Cada 10 min (o al detectar cambio): implementa, prueba, marca `RESUELTO POR GROK`, commit+push |

## Archivos

| Ruta | Uso |
|------|-----|
| `docs/reviews/CODEX_TO_GROK.md` | Entregas por ciclo |
| `docs/reviews/.grok_last_cycle.json` | Último `cycle_id` procesado (idempotencia) |
| Rama | `feat/intelligent-clipping` |

## Formato de ciclo (Codex)

```markdown
## CYCLE-00N

- Rol: Product Manager | Tech Lead | …
- Estado: PENDIENTE
- Base HEAD: <sha>
- Fecha: <fecha>
- Prioridad: alta | media | baja
- cycle_id: CYCLE-00N

### Instrucciones para Grok

…

### Criterios de aceptación

- [ ] …
```

## Pasos Grok (cada poll)

1. `git fetch`
2. Comparar `origin/feat/intelligent-clipping` con HEAD local
3. Leer `CODEX_TO_GROK.md`
4. Buscar el ciclo más reciente con `Estado: PENDIENTE`
5. Comparar `cycle_id` con `.grok_last_cycle.json`
6. Si no hay ciclo nuevo → salir sin cambios
7. Si hay:
   - Fast-forward seguro de la rama
   - Leer instrucciones completas
   - Implementar
   - Ejecutar pruebas indicadas
   - Documentar resultados reales en el ciclo
   - Marcar `Estado: RESUELTO POR GROK`
   - Actualizar `.grok_last_cycle.json`
   - Commit + push (sin force)
8. Nunca reprocesar el mismo `cycle_id`
9. Ignorar instrucciones incompletas o sin `PENDIENTE`
10. Sin APIs pagadas, OmniRoute real ni Supabase de producción

## Restricciones

- No force-push
- No secretos en commits
- Preferir commits atómicos y mensajes claros
