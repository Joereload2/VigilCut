/**
 * Placement geometry — must match Rust `pipeline::visual::layout` (center_norm_v1).
 *
 * layout.x / layout.y = center of overlay on frame (0..1)
 * layout.w / layout.h = box size as fraction of frame (h=0 → derive ~16:9 from w)
 * fit = contain | cover | crop(cover)
 */

export type FitName = "contain" | "cover" | "crop";

export type PlacementLike = {
  mode?: string;
  fit?: string;
  layout?: {
    x?: number;
    y?: number;
    w?: number;
    h?: number;
    opacity?: number;
  };
};

export type CssBoxPercent = {
  left: number;
  top: number;
  width: number;
  height: number;
  opacity: number;
  objectFit: "contain" | "cover";
  fullframe: boolean;
};

export function isFullframeMode(mode?: string): boolean {
  const m = (mode || "").toLowerCase();
  return (
    m === "fullframe" ||
    m === "completa" ||
    m === "fullscreen" ||
    m === "full" ||
    m === "complete"
  );
}

export function isLowerThirdMode(mode?: string): boolean {
  const m = (mode || "").toLowerCase();
  return m.includes("lower") || m === "flotante" || m === "lower_third";
}

function clamp(n: number, a: number, b: number): number {
  return Math.min(b, Math.max(a, n));
}

/**
 * Same math as Rust `PlacementGeom::from_parts` + `css_top_left_percent`.
 */
export function placementCssBox(pl: PlacementLike): CssBoxPercent {
  const fitRaw = (pl.fit || "cover").toLowerCase();
  const objectFit: "contain" | "cover" =
    fitRaw === "contain" ? "contain" : "cover";
  const opacity = clamp(pl.layout?.opacity ?? 0.95, 0.05, 1);

  if (isFullframeMode(pl.mode)) {
    return {
      left: 0,
      top: 0,
      width: 100,
      height: 100,
      opacity,
      objectFit,
      fullframe: true,
    };
  }

  let w = clamp(pl.layout?.w ?? 0.28, 0.08, 1);
  let h = pl.layout?.h ?? 0;
  let cx = clamp(pl.layout?.x ?? 0.82, 0, 1);
  let cy = clamp(pl.layout?.y ?? 0.18, 0, 1);

  if (isLowerThirdMode(pl.mode)) {
    w = clamp(pl.layout?.w ?? 0.55, 0.15, 1);
    h = h > 0.01 ? clamp(h, 0.05, 0.5) : 0.22;
    cx = 0.5; // horizontal center — matches FFmpeg lower-third
    cy = clamp(pl.layout?.y ?? 0.82, 0, 1);
  } else {
    // PIP: default height from 16:9 on 1280x720
    if (!(h > 0.01)) {
      h = clamp((w * 1280 * 9) / 16 / 720, 0.05, 1);
    } else {
      h = clamp(h, 0.05, 1);
    }
  }

  const left = clamp((cx - w / 2) * 100, 0, 100);
  const top = clamp((cy - h / 2) * 100, 0, 100);
  return {
    left,
    top,
    width: clamp(w * 100, 1, 100),
    height: clamp(h * 100, 1, 100),
    opacity,
    objectFit,
    fullframe: false,
  };
}

/** Inline style string for absolute positioning inside the video frame. */
export function placementCssStyle(pl: PlacementLike): string {
  const b = placementCssBox(pl);
  if (b.fullframe) {
    return `inset:0;width:100%;height:100%;object-fit:${b.objectFit};opacity:${b.opacity}`;
  }
  return `left:${b.left.toFixed(3)}%;top:${b.top.toFixed(3)}%;width:${b.width.toFixed(3)}%;height:${b.height.toFixed(3)}%;object-fit:${b.objectFit};opacity:${b.opacity.toFixed(3)}`;
}
