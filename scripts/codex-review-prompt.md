Ejecuta la revision programada Codex-Grok de VigilCut.

Lee completamente AGENTS.md y docs/reviews/CODEX_TO_GROK.md. Revisa todos
los ciclos marcados RESUELTO POR GROK que aun no tengan verificacion de
Codex. Para cada ciclo usa al menos tres angulos distintos exigidos por el
handoff y contrasta siempre con codigo, Git y pruebas reales.

Si un ciclo esta correcto, registra una seccion Revision Codex con fecha y
evidencia concreta. Si existe una mejora, devuelve el ciclo a PENDIENTE o
crea el siguiente ciclo segun el protocolo; no implementes directamente la
mejora de producto. Respeta estrictamente los gates de dinero y secretos.

Codex y Grok forman un solo circuito. Si no queda ninguna mejora verificable
para Grok, o una condicion de seguridad exige detenerse, cambia ambos estados
a DETENIDO en la misma edicion. Nunca dejes uno activo y el otro detenido.
Nunca los reactives: solo la persona responsable puede hacerlo.

Deja registro de la corrida en docs/reviews/CODEX_TO_GROK.md. No hagas push,
no uses force y no escribas credenciales en archivos ni logs. Si rg no esta
instalado, usa Select-String o Get-ChildItem en vez de detener la revision.
