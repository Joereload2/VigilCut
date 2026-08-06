# VigilCut en el ecosistema faceless

**Rol de esta app:** postproducción y **shorts** (pipeline Events → Policy → EDL → artefactos, clipping 9:16).

## No confundir

| Nombre | Qué es |
|--------|--------|
| **VisuaLibrary** (repo `VisuaLibrary`) | App de **librería de imágenes** por conceptos para lecciones long (YouToMagic → package → FC) |
| **visual-library** en VigilCut | Módulo/diseño de enriquecimiento visual **dentro del pipeline de recorte/shorts** |

Son dominios distintos. No unificar UIs.

## Flujo del ecosistema

```text
YouToMagic     → decide nicho + guion + package
VisuaLibrary   → imágenes approved (long)
FacelessCreator→ montaje long horizontal MP4
VigilCut       → short / recorte / 9:16 / excepciones de edición
```

Si el long ya está en FacelessCreator, VigilCut entra cuando necesitas **versión short** o fábrica de clips desde vídeo fuente.

## Fuente de verdad de roles

`YouToMagic/docs/18-ECOSISTEMA-APPS.md`
