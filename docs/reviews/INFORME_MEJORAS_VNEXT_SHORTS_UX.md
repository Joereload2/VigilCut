# Informe de mejoras — VigilCut vNext Shorts (operador)

**Fecha:** 2026-07-31  
**Rama:** `feat/independent-visual-library`  
**Alcance:** Shell de producto Short (vNext), flujo de análisis → momentos → encuadre 9:16 → export  

Este documento resume **qué se logró**, **cómo lo pediste** y **principios de diseño** para futuras iteraciones.

---

## 1. Problema de partida (tu feedback)

| Queja / pedido | Consecuencia en producto |
|----------------|--------------------------|
| UI “Factory” con Silencios / B-roll / Biblioteca | Confusión; no es el job del operador |
| Tras analizar, salto al final o a pantallas raras | Se pierden pasos de decisión humana |
| Play no responde / “Cargando…” eterno | No se puede validar el momento |
| Vista 9:16 negra | No se puede anticipar el short final |
| Export no genera / no dice dónde guarda | Sin cierre del trabajo |
| Letras raras en el MP4 | Burn-in de subtítulos basura |
| Reabrir video = solo resultado | No se pueden reelegir momentos |
| Marcos y proporción | Flexible sí, pero el contenido debe **caber** sin deformar |
| Partes de abajo no se ven | Layout debe caber en la ventana |

---

## 2. Flujo de producto acordado (y entregado)

```
Abrir video → Analizar
    → Parte 1: pestañas (un short a la vez)
         play · duración · texto · Usar / Descartar
    → Continuar con N
    → Parte 2: marcos verdes + preview 9:16
    → Generar Short 9:16
    → Resultado: preview + ruta + carpeta
         [Ir al video completo] vuelve al flujo
```

### Reglas de navegación (importantes)

1. **Después de analizar** no se salta a Resultado ni a encuadre sin decisión en Parte 1.  
2. **Momentos preseleccionados** tienen prioridad sobre un MP4 viejo (caché).  
3. **Con short ya generado**, reabrir el proyecto puede ir a Resultado; el botón **“Ir al video completo”** fuerza el flujo de momentos otra vez **sin borrar** el archivo.  
4. **Descartados** siguen visibles (↩) y se pueden volver a “Usar”.

---

## 3. Mejoras técnicas por área

### 3.1 Shell exclusivo Short
- `App.svelte` solo monta `VNextShell` (`data-shell="vnext-only"`).
- Acceso directo / launcher apuntan al **release** con UI embebida (no Factory).

### 3.2 Stage puro (`deriveProjectStage`)
- Una sola función pura: snapshot → etapa.
- Orden clave: pickable → (artefacto final) → adjusting → …
- Evita “salto al final” y “nunca ver resultado” por estados mezclados.

### 3.3 Parte 1 — un short por pestaña
- Lista multi-checkbox confusa → **pestañas #1…#N**.
- Cada pestaña: video del tramo, duración, texto propio, Descartar / Usar.
- Avance de pestaña **inmediato** (no espera al backend).
- Caché: reabrir el mismo archivo reutiliza análisis y decisiones.

### 3.4 Reproductor local robusto
- `bindLocalVideo` + `SegmentPlayer`: carga una vez, poll de `readyState`.
- Evita bucles de `$state` que reiniciaban el video.
- Play usable aunque el overlay diga “Cargando”.

### 3.5 Parte 2 — marcos verdes + preview
- Marcos **flexibles** (cualquier tamaño).
- Lo seleccionado se **reduce proporcionalmente** al 9:16 (letterbox, sin estirar).
- Preview y FFmpeg usan la misma idea: `force_original_aspect_ratio=decrease` + `pad`.
- Multi-sección (1–3) con vstack en export.

### 3.6 Export y entrega de archivo
- Errores de FFmpeg **no se silencian**.
- Ruta de entrega junto al original:
  ```
  {carpeta_del_video}/{NombreVideo}/shorts/{NombreVideo}_short_xxx.mp4
  ```
- Resultado: preview, ruta completa, Abrir carpeta / archivo / Copiar ruta.
- Sin burn-in de subtítulos por defecto (evita texto basura).

### 3.7 Layout a pantalla completa
- Header y barras de página **compactas**.
- Menos padding del body; `overflow-hidden` + `min-h-0` en flex.
- Controles inferiores visibles sin scroll “fantasma”.

---

## 4. Cómo pediste las cosas → cómo traducir a diseño

Patrones recurrentes en tus mensajes (útiles para futuros diseños):

| Cómo lo pedís | Interpretación de diseño |
|---------------|---------------------------|
| “Paso al final / se salta” | Priorizar etapa humana; no confiar solo en artefactos viejos |
| “Una pestaña por short” | Revisión **1 a 1**, no multi-select opaco |
| “Play no obedece” | Controles deben reflejar estado real; no bloquear Play |
| “No se ve el 9:16” | Preview = promesa del producto final |
| “No genera / no dice dónde” | Cierre del job: archivo + ruta + acciones |
| “Memoria del video” | Caché por proyecto/ruta; descartados re-elegibles |
| “Marcos flexibles pero que encaje” | Selección libre; **fit proporcional** al canvas de salida |
| “Todo entre en pantalla” | Densidad de chrome baja; el canvas manda |
| Capturas de pantalla | La UI real manda sobre la especificación abstracta |

### Principios para el futuro

1. **Un job claro:** producir un Short vertical usable, no un panel de herramientas.  
2. **Decisión humana visible** en cada paso (momento → encuadre → export).  
3. **Preview = verdad** del export (misma geometría / misma lógica de fit).  
4. **Estado durable + sesión:** SQLite para persistir; flags de sesión para no saltar pantallas.  
5. **Errores ruidosos:** si falla FFmpeg/ruta, el operador debe ver el mensaje.  
6. **Layout “operator workstation”:** todo lo crítico en un viewport sin esconder botones abajo.  
7. **Caché amable:** acelerar, no secuestrar el flujo (siempre hay “volver a momentos”).

---

## 5. Archivos / módulos principales

| Área | Ruta |
|------|------|
| Shell | `src/lib/vnext/VNextShell.svelte`, `src/App.svelte` |
| Stage | `src/lib/vnext/presentation/deriveStage.ts` |
| Sesión | `src/lib/vnext/stores/sessionStore.svelte.ts` |
| Parte 1 | `src/lib/vnext/pages/ShortPickPage.svelte` |
| Parte 2 | `src/lib/vnext/pages/ShortCanvasPage.svelte` |
| Crop / fit | `src/lib/vnext/types/crop.ts`, `MultiCropSource`, `SplitShortPreview` |
| Media | `src/lib/vnext/media/bindLocalVideo.ts`, `fileUrl.ts` |
| Resultado | `src/lib/vnext/pages/ResultPage.svelte` |
| Export path | `src-tauri/src/vnext/application/render.rs` |
| Fit FFmpeg | `src-tauri/src/pipeline/clipping/framing.rs` |
| Abrir OS | `src-tauri/src/commands/system_open.rs` |

---

## 6. Cómo probar (checklist operador)

1. Abrir release (`Abrir-VigilCut.vbs` / exe actualizado).  
2. Analizar un video → **Parte 1 pestañas**, no Resultado.  
3. Play en un momento; Usar / Descartar; Continuar.  
4. Parte 2: marcos flexibles; preview 9:16 muestra fit proporcional.  
5. Generar Short → Resultado con ruta `…/Nombre/shorts/…`.  
6. “Ir al video completo” → vuelve a pestañas.  
7. Reabrir el mismo video (caché) → más rápido; se ven descartados.

---

## 7. Pendientes / ideas futuras (no bloqueantes)

- Opción explícita “quemar subtítulos” cuando el texto sea bueno.  
- Hash de archivo (no solo ruta) para caché entre copias del mismo video.  
- Batch export de varios “Usar” en una cola.  
- Preferencia de fit: contain (actual) vs cover (llenar 9:16 recortando).  

---

## 8. Resumen en una frase

**Construimos un flujo de operador de un solo producto (Short vertical): decidir momentos uno a uno, componer el 9:16 con marcos libres que encajan proporcionalmente, exportar a una carpeta por video con ruta clara, y poder reabrir el trabajo sin perder memoria ni quedar atrapado en el resultado.**
