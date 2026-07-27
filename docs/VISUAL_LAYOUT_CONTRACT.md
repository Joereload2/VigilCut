# Visual layout contract — `center_norm_v1`

**Goal:** live preview (CSS) and FFmpeg bake use the **same geometry**.

## Fields (`PlacementLayout`)

| Field | Meaning |
|-------|---------|
| `x` | **Center** X of overlay on frame, 0..1 |
| `y` | **Center** Y of overlay on frame, 0..1 |
| `w` | Width as fraction of frame width |
| `h` | Height as fraction of frame height; `0` → derive (PIP ≈ 16:9 on 1280×720; lower-third ≈ 0.22) |
| `opacity` | 0.05..1 |

## Modes

| Mode | Geometry |
|------|----------|
| `fullframe` / completa | Full frame; fit contain/cover only |
| `picture_in_picture` / parcial / overlay | Center `(x,y)`, size `(w,h)` |
| `lower_third` / flotante | **cx forced to 0.5**, cy=`y`, size `(w,h)` |

## Fit

| `fit` | CSS `object-fit` | FFmpeg |
|-------|------------------|--------|
| `contain` | contain | `force_original_aspect_ratio=decrease` |
| `cover` / `crop` | cover | `increase` + crop to box |

## Code

| Layer | Path |
|-------|------|
| Rust source of truth | `src-tauri/src/pipeline/visual/layout.rs` |
| FFmpeg consumer | `src-tauri/src/pipeline/visual/render.rs` |
| TS mirror | `src/lib/components/visual/layout.ts` |
| Live overlay | `src/lib/components/visual/VisualLiveOverlay.svelte` |

## Frame for bake

Export normalizes the cut video to **1280×720** (pad/letterbox) so absolute overlay pixels match `box_px(1280,720)`.

## CSS vs FFmpeg

- CSS uses **top-left %** of the video element:  
  `left = (cx - w/2)*100`, etc. (no `translate(-50%)`).
- FFmpeg uses **absolute pixels** from the same formula on 1280×720.

## Tests

```powershell
cargo test --manifest-path src-tauri/Cargo.toml layout:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test smoke_visual -- --nocapture
```
