# Especificación para Grok — Centro de Control de la Biblioteca Visual

## Objetivo

Construir una pantalla de Biblioteca que permita al usuario:

1. Acceder sin abrir un video.
2. Ver cuántas imágenes existen y cuánto cubren.
3. Dar instrucciones para conseguir imágenes de conceptos concretos.
4. Saber si el sistema está trabajando, esperando, bloqueado o terminó.
5. Saber si OmniRoute está disponible y si permite generar imágenes.
6. Distinguir el límite local de VigilCut de la cuota real del proveedor.
7. Revisar y aprobar resultados antes de incorporarlos a la Biblioteca.

No escribir esta funcionalidad como otro panel pequeño dentro de B-roll. Debe ser una pantalla operativa de ancho completo.

## Principio de producto

La pantalla debe responder en menos de cinco segundos a estas preguntas:

- ¿Está funcionando la Biblioteca automática?
- ¿Qué proveedor usaría ahora?
- ¿Puede generar imágenes o solo simular?
- ¿Cuántas puede intentar hoy?
- ¿Cuántas imágenes tengo?
- ¿Qué conceptos están cubiertos y cuáles faltan?
- ¿Qué está haciendo en este momento?

Si una respuesta no está disponible desde el backend o proveedor, mostrar `No informado`. Nunca inferir o inventar cuotas, gratuidad o licencias.

## Acceso

### Navegación

Dentro de `Visuales`, mantener las vistas:

- Este video.
- Biblioteca.
- Por revisar.

La vista Biblioteca debe ser accesible:

- Sin video.
- Sin `projectKey`.
- Sin análisis.
- Sin EDL.
- Con OmniRoute desactivado.

### Espacio

Biblioteca debe usar el ancho completo de la aplicación. No renderizarla dentro del panel derecho del layout 70/30.

## Estructura de la pantalla

```text
┌────────────────────────────────────────────────────────────────────┐
│ Biblioteca visual                                      [Importar] │
│ 184 imágenes · 37 conceptos cubiertos                               │
├────────────────────────────────────────────────────────────────────┤
│ Estado del servicio                                                 │
│ OmniRoute: disponible · Imágenes: verificadas · Gratis: no verificado│
│ VigilCut hoy: 3/10 intentos · Cuota OmniRoute: no informada         │
├────────────────────────────────────────────────────────────────────┤
│ ¿Qué imágenes quieres conseguir?                                   │
│ [ Personas usando transporte público                         ]      │
│ Cantidad [5]  Formato [16:9]  Contexto [Ciudad latinoamericana]    │
│ [Buscar primero en Biblioteca y completar faltantes]               │
├────────────────────────────────────────────────────────────────────┤
│ Actividad                                                           │
│ 2 en cola · 1 generando · 4 por revisar · último ciclo hace 8 min  │
├───────────────────────────────────┬────────────────────────────────┤
│ Cobertura por concepto            │ Imágenes de la Biblioteca      │
│ Economía  8 imágenes  Cubierto    │ [asset] [asset] [asset]        │
│ Inflación 0 imágenes  Falta       │ [asset] [asset] [asset]        │
└───────────────────────────────────┴────────────────────────────────┘
```

En móvil/tablet, apilar las secciones. No ocultar Estado del servicio ni Actividad detrás de hover.

## Sección 1 — Resumen de inventario

Mostrar cards simples:

- `Imágenes`: total de `media_assets` no eliminados.
- `Activas`: assets con estado active.
- `Por revisar`: candidatos pendientes de revisión humana.
- `Archivos ausentes`: assets con estado missing.
- `Conceptos cubiertos`: conceptos con al menos un asset activo utilizable.
- `Conceptos sin cobertura`: conceptos activos/prioritarios sin assets utilizables.

Opcional si se puede calcular correctamente:

- Espacio en disco usado por archivos administrados.

No contar candidatos como imágenes alojadas hasta que estén aprobados y promovidos a `media_assets`.

No contar assets missing, blocked o rejected como cobertura utilizable.

## Sección 2 — Estado de OmniRoute y proveedor

### Estados visibles

Mostrar cuatro estados independientes:

1. **Configuración**
   - Configurado.
   - No configurado.
   - Mock local.

2. **Conectividad**
   - Disponible.
   - No disponible.
   - Verificando.
   - Última verificación y latencia.

3. **Capacidad de imagen**
   - Verificada.
   - No verificada.
   - No soportada.

4. **Coste**
   - Local/mock.
   - Gratis verificado.
   - Gratis configurado, no verificado.
   - Pago.
   - Desconocido.

No reducir todo a un punto verde “OmniRoute conectado”. El endpoint `/models` accesible no demuestra que el modelo genere imágenes ni que sea gratuito.

### Acción de comprobación

Botón:

`Comprobar OmniRoute`

Debe llamar el probe existente o uno endurecido y actualizar:

- provider.
- model.
- reachability.
- supports_image.
- free_tier.
- free_verified.
- cost_kind.
- latency.
- error resumido.
- checked_at.

La comprobación no debe generar una imagen ni consumir una generación pagada.

### Texto recomendado

Ejemplo seguro:

> OmniRoute responde, pero la generación de imágenes y su gratuidad todavía no están verificadas. La Biblioteca automática permanecerá bloqueada.

## Sección 3 — Límites y cuota

Separar explícitamente:

### Límite de VigilCut

Datos controlados localmente:

- Máximo de intentos diarios configurado.
- Intentos consumidos hoy.
- Intentos restantes según VigilCut.
- Máximo por solicitud/instrucción.
- Máximo de reintentos por concepto.
- Si proveedores pagados están habilitados.

### Cuota de OmniRoute

Mostrar solamente datos entregados por una API o headers oficiales:

- Límite remoto.
- Usado.
- Restante.
- Fecha de reset.

Si OmniRoute no ofrece esos datos:

> Cuota de OmniRoute: no informada por el proveedor.

No usar `VIGILCUT_MAX_DAILY_GENERATIONS` como si fuera la cuota de OmniRoute. Es un límite de seguridad local.

### Estado operativo combinado

Derivar un mensaje:

- `Puede trabajar`: provider válido + imagen soportada + coste permitido + límite local restante.
- `Bloqueado por coste`: ruta no verificada o de pago no autorizada.
- `Límite local alcanzado`.
- `Proveedor no disponible`.
- `Solo simulación mock`.

## Sección 4 — Dar instrucciones a la Biblioteca

### No usar un chat libre como única interfaz

El usuario puede escribir una instrucción humana, pero antes de ejecutar debe ver una interpretación estructurada y editable.

Formulario:

- `Qué quieres representar` — obligatorio.
- `Cantidad deseada` — 1 a límite seguro definido por backend.
- `Formato` — 16:9, 9:16, 1:1 o cualquiera.
- `Contexto deseado` — opcional.
- `Evitar` — marcas, texto, lujo, violencia, etc.
- `Prioridad` — normal o alta.

Ejemplo:

```text
Qué quieres representar: Personas usando transporte público
Cantidad: 5
Formato: 16:9
Contexto: Ciudad latinoamericana, vida cotidiana
Evitar: Logos, texto visible, marcas comerciales
```

### Acción principal

Texto:

`Buscar y completar`

Comportamiento:

1. Buscar assets existentes que cubran el concepto.
2. Mostrar cuántos ya existen y son utilizables.
3. Calcular el déficit: `cantidad deseada - assets útiles`.
4. Pedir confirmación antes de encolar generaciones.
5. Encolar solamente el déficit.
6. No generar si la Biblioteca ya cumple la cantidad.
7. No generar si el provider/coste está bloqueado.

Confirmación:

> Ya existen 2 imágenes utilizables. Faltan 3. VigilCut encolará 3 generaciones mediante OmniRoute. Coste: gratis no verificado — bloqueado.

Si está bloqueado, ofrecer:

- Guardar instrucción para más tarde.
- Importar imágenes manualmente.
- Volver a comprobar proveedor.

No ofrecer “continuar de todas formas” cuando la gratuidad no está verificada.

### Persistencia de instrucciones

Crear un concepto o solicitud persistida, no una acción efímera del frontend.

Campos mínimos:

- id.
- título/label.
- cantidad objetivo.
- formato.
- contextos positivos.
- contextos negativos.
- exclusiones duras.
- prioridad.
- estado.
- assets útiles actuales.
- jobs activos.
- created_at / updated_at.

Estados visibles:

- Guardada.
- Buscando en Biblioteca.
- Cubierta.
- Esperando proveedor.
- En cola.
- Generando.
- Por revisar.
- Parcialmente cubierta.
- Bloqueada.
- Fallida.
- Cancelada.

## Sección 5 — Actividad en tiempo real

El usuario debe saber si el sistema está trabajando sin abrir detalles técnicos.

Mostrar:

- Jobs en cola.
- Job activo.
- Etapa humana: Preparando, Esperando proveedor, Generando, Descargando, Revisando.
- Progreso indeterminado si el proveedor no informa porcentaje.
- Concepto asociado.
- Intento actual / máximo.
- Tiempo desde inicio.
- Cancelación solicitada.
- Último error resumido.

No mostrar un porcentaje ficticio. Usar barra indeterminada si no hay progreso real.

### Indicador global

En el encabezado de Biblioteca:

- `En reposo`.
- `Trabajando · 1 activa, 2 en cola`.
- `4 imágenes esperan tu revisión`.
- `Bloqueada: OmniRoute no verificado`.
- `Límite diario alcanzado`.

### Actualización

- UI consulta snapshots o escucha eventos.
- UI nunca ejecuta `worker_tick`.
- Polling de snapshot aceptable cada 2–5 segundos solo mientras existan jobs no terminales.
- Detener polling al quedar en reposo.
- El supervisor Rust sigue trabajando aunque la pantalla se cierre.

## Sección 6 — Conceptos y cobertura

### Tabla/lista principal

Por concepto mostrar:

- Título.
- Estado: cubierto, parcial o sin cobertura.
- Objetivo de imágenes.
- Assets activos utilizables.
- Candidatos por revisar.
- Jobs en curso.
- Formatos disponibles.
- Última actualización.
- Acción contextual.

Ejemplo:

```text
Inflación
0 de 3 imágenes · Sin cobertura
16:9 requerido · 1 job en cola
[Ver actividad]
```

### Definición objetiva de cobertura

Un asset cubre un concepto solo si:

- Está relacionado explícitamente mediante `asset_concepts` o relación equivalente fiable.
- Estado active.
- Archivo existe.
- QA no está rejected.
- Licencia no está bloqueada para el uso previsto.

No usar únicamente coincidencia de strings en `tags` para el contador oficial.

### Filtros

- Todos.
- Sin cobertura.
- Parciales.
- Cubiertos.
- Con actividad.
- Por revisar.

Orden por prioridad y déficit de cobertura, no alfabético por defecto.

## Sección 7 — Assets alojados

Mostrar grid reutilizando la Biblioteca existente, pero añadir resumen claro:

```text
184 imágenes alojadas
172 activas · 7 archivadas · 5 ausentes
```

Cada card conserva título, preview y conceptos. Los detalles técnicos permanecen colapsados.

Filtros útiles:

- Concepto.
- Origen: importada, OmniRoute, mock.
- Formato.
- Licencia.
- Estado.

## Sección 8 — Revisión

Resultados generados no cuentan como alojados ni como cobertura final hasta aprobarlos.

La bandeja `Por revisar` debe mostrar:

- Instrucción/concepto que originó la imagen.
- Objetivo: por ejemplo `2 de 5 cubiertas`.
- Provider/model.
- Coste verificado o desconocido.
- QA técnica y semántica.
- Aprobar.
- Rechazar.
- Aprobar y continuar completando si todavía existe déficit.

Al aprobar:

- Promover una sola vez.
- Asociar al concepto.
- Recalcular cobertura.
- Actualizar inventario.

## Backend requerido

Las APIs actuales están fragmentadas. Grok debe preferir snapshots agregados para evitar que el frontend reconstruya estados con reglas distintas.

### `visual_library_dashboard`

Debe devolver, como mínimo:

```json
{
  "inventory": {
    "totalAssets": 0,
    "activeAssets": 0,
    "missingAssets": 0,
    "pendingCandidates": 0,
    "managedBytes": 0
  },
  "coverage": {
    "totalConcepts": 0,
    "coveredConcepts": 0,
    "partialConcepts": 0,
    "uncoveredConcepts": 0
  },
  "activity": {
    "queued": 0,
    "running": 0,
    "cancelling": 0,
    "failedToday": 0,
    "lastCycleAt": null
  },
  "provider": {
    "name": "omniroute",
    "model": null,
    "configured": false,
    "reachable": false,
    "supportsImage": false,
    "freeVerified": false,
    "costKind": "unknown",
    "lastCheckedAt": null,
    "latencyMs": null,
    "error": null
  },
  "limits": {
    "localDailyLimit": 0,
    "localUsedToday": 0,
    "localRemainingToday": 0,
    "providerLimit": null,
    "providerUsed": null,
    "providerRemaining": null,
    "providerResetAt": null
  }
}
```

Todos los conteos deben calcularse en backend/SQLite con semántica documentada.

### `visual_library_list_concept_coverage`

Devuelve conceptos con objetivo, assets útiles, candidatos, jobs, formatos y déficit.

### `visual_library_create_request`

Recibe la instrucción estructurada, busca primero y devuelve un preview sin encolar:

- assets encontrados.
- objetivo.
- déficit.
- provider elegido.
- coste permitido/bloqueado.
- máximo encolable.
- razón de bloqueo.

### `visual_library_confirm_request`

Encola el déficit aprobado de manera idempotente y devuelve IDs de jobs.

### `visual_library_list_requests`

Permite recuperar instrucciones y progreso tras reiniciar.

No hacer depender estas APIs de un `projectKey` de video. Usar un scope global de Biblioteca y relaciones de concepto.

## Reutilización de APIs existentes

Puede reutilizarse internamente:

- `visual_list_assets`.
- `visual_list_concepts`.
- `visual_probe_image_provider`.
- `visual_cost_policy`.
- `visual_supervision_global`.
- `visual_daily_feed_settings`.
- `visual_daily_week_summary`.

No exigir al frontend llamar todas y combinar resultados para decidir si se puede generar. Esa decisión debe salir del backend agregado.

## Seguridad y costes

- Nunca activar pago desde la pantalla.
- No mostrar una ruta como gratis solo por configuración.
- Daily y solicitudes globales requieren `free_verified=true` o mock explícito.
- Mock se etiqueta `Simulación; no es generación IA real`.
- No llamar probe continuamente; cachear con timestamp y botón manual.
- No exponer API keys.
- No mostrar prompts con datos sensibles en vista principal.
- Cancelar debe abortar trabajo cooperativamente según implementación del supervisor.

## Flujo mínimo de usuario

1. Abrir Visuales → Biblioteca.
2. Ver inventario, cobertura, actividad y estado de OmniRoute.
3. Escribir concepto y cantidad.
4. Pulsar Buscar y completar.
5. Ver assets existentes, déficit, provider y límites.
6. Confirmar si está permitido.
7. Observar cola/actividad.
8. Recibir badge Por revisar.
9. Aprobar/rechazar.
10. Ver inventario y cobertura actualizados.

## Estados vacíos

### Sin assets

> La Biblioteca está vacía. Importa imágenes o crea una instrucción para comenzar.

### Sin conceptos

> Aún no hay conceptos. Describe qué imágenes quieres conseguir.

### OmniRoute no configurado

> OmniRoute no está configurado. Puedes importar imágenes o usar el modo mock de desarrollo.

### OmniRoute accesible pero no verificado

> OmniRoute responde, pero no se verificó generación de imágenes gratuita. Las tareas automáticas están bloqueadas.

### Reposo

> No hay tareas activas.

## Orden de implementación para Grok

1. Definir semántica de inventario, cobertura y límites.
2. Implementar snapshot `visual_library_dashboard` con tests SQLite.
3. Implementar coverage por concepto con relación explícita y tests.
4. Implementar request persistida: preview y confirmación idempotente.
5. Construir encabezado, status de provider y resumen.
6. Construir formulario de instrucciones.
7. Integrar actividad y recuperación tras reinicio.
8. Integrar cobertura y grid de assets.
9. Integrar Por revisar y actualización de contadores.
10. Validar flujo completo solo con mock seguro; no llamar OmniRoute real en tests.

## Pruebas obligatorias

- Dashboard vacío devuelve ceros, no nulls inesperados.
- Candidate pendiente no incrementa totalAssets ni cobertura.
- Asset approved/active/existente incrementa inventario y cobertura una vez.
- Missing/blocked no cuenta como cobertura útil.
- Solicitud objetivo 5 con 2 assets útiles calcula déficit 3.
- Confirmar dos veces no crea más de 3 jobs.
- Provider reachable pero `supports_image=false` bloquea.
- `free_configured=true` y `free_verified=false` bloquea automatización.
- Límite local 10, usado 8, déficit 5: máximo encolable 2 y UI lo explica.
- Cuota remota ausente se serializa como null y UI muestra `No informada`.
- Reinicio recupera request y jobs.
- Aprobar candidato actualiza total, concepto y déficit atómicamente.
- Cancelar actualiza actividad sin contar resultado posterior.
- Biblioteca abre sin video/projectKey.
- `npm run check`, build, fmt, clippy, unit visual y smoke pasan.

## Criterios de aceptación

- Biblioteca accesible sin video.
- Usuario puede crear una instrucción estructurada y ver su interpretación antes de encolar.
- Se busca primero en assets existentes.
- Se genera solo el déficit.
- El usuario ve claramente si está trabajando.
- Se distingue OmniRoute configurado, accesible, compatible y gratis verificado.
- Se distinguen límite local y cuota remota.
- Si la cuota remota es desconocida, se declara así.
- Inventario no incluye candidatos pendientes.
- Cobertura usa relaciones fiables y solo assets utilizables.
- Conceptos muestran objetivo, cobertura, déficit y actividad.
- La UI no ejecuta worker.
- No se llama servicio pagado ni Supabase de producción.
- Mock se identifica siempre.

## Fuera de alcance

- Chat autónomo con LLM para interpretar instrucciones.
- Comprar créditos o habilitar pagos.
- Modificar configuración secreta desde UI.
- Supabase sync.
- Marketplace.
- Borrado físico masivo.
- Generación sin confirmación cuando existe coste desconocido.

No presentar estos puntos como implementados.
