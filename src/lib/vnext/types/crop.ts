/**
 * Green vision frame on the source (16:9 stage, object-fit:cover).
 * Frames are flexible (any size). Content is shown in the vertical short by
 * scaling DOWN proportionally to fit the panel (contain + bars if needed).
 */

export interface CropRegion {
  id: string;
  /** Center of the selection in source stage (0..1). */
  centerX: number;
  centerY: number;
  /** Size of selection as fraction of source stage (free / moldable). */
  widthFrac: number;
  heightFrac: number;
}

export const MIN_SIZE = 0.1;
export const MAX_W = 0.95;
export const MAX_H = 1;
export const MAX_SECTIONS = 3;

/** Stage is 16:9 — physical AR of a UV box. */
export function physicalAspectOfBox(r: CropRegion): number {
  const w = Math.max(0.05, r.widthFrac);
  const h = Math.max(0.05, r.heightFrac);
  return (w / h) * (16 / 9);
}

/** Default free section (talking-head strip). */
export function defaultSection(index: number): CropRegion {
  const xs = [0.32, 0.68, 0.5];
  return clampSection({
    id: `sec-${index}-${Math.random().toString(36).slice(2, 8)}`,
    centerX: xs[index % 3] ?? 0.5,
    centerY: 0.45,
    widthFrac: 0.32,
    heightFrac: 0.9,
  });
}

/** Flexible clamp — no aspect lock. */
export function clampSection(r: CropRegion): CropRegion {
  const widthFrac = Math.min(MAX_W, Math.max(MIN_SIZE, r.widthFrac));
  const heightFrac = Math.min(MAX_H, Math.max(MIN_SIZE, r.heightFrac));
  const halfW = widthFrac / 2;
  const halfH = heightFrac / 2;
  return {
    ...r,
    widthFrac,
    heightFrac,
    centerX: Math.min(1 - halfW, Math.max(halfW, r.centerX)),
    centerY: Math.min(1 - halfH, Math.max(halfH, r.centerY)),
  };
}

export function relockSections(regions: CropRegion[], panelCount?: number): CropRegion[] {
  const n = Math.min(MAX_SECTIONS, Math.max(1, panelCount ?? regions.length));
  return regions.slice(0, n).map(clampSection);
}

/** CSS % box for overlay on source. */
export function sectionBox(r: CropRegion): { left: number; top: number; w: number; h: number } {
  const c = clampSection(r);
  const w = c.widthFrac * 100;
  const h = c.heightFrac * 100;
  const left = c.centerX * 100 - w / 2;
  const top = c.centerY * 100 - h / 2;
  return { left, top, w, h };
}

/**
 * Inner "fit box" style: same physical aspect as the green selection,
 * max 100%×100% of the panel, centered. Selection maps 1:1 into this box
 * (uniform) so the panel can letterbox around it.
 */
export function regionToFitBoxStyle(r: CropRegion): string {
  const ar = physicalAspectOfBox(r);
  return [
    "position:relative",
    "max-width:100%",
    "max-height:100%",
    "width:100%",
    "height:100%",
    `aspect-ratio:${ar}`,
    "margin:auto",
    "overflow:hidden",
    // Prefer width or height so box fits inside panel
    "object-fit:contain",
  ].join(";");
}

/**
 * Map green UV region to fill a container that already has the selection's
 * physical aspect (the fit box). Uniform — no stretch.
 */
export function regionToPanelStageStyle(r: CropRegion): string {
  const c = clampSection(r);
  const w = Math.max(0.05, c.widthFrac);
  const h = Math.max(0.05, c.heightFrac);
  const left = c.centerX - w / 2;
  const top = c.centerY - h / 2;
  return [
    "position:absolute",
    "max-width:none",
    "max-height:none",
    `width:${(100 / w).toFixed(4)}%`,
    `height:${(100 / h).toFixed(4)}%`,
    `left:${((-left / w) * 100).toFixed(4)}%`,
    `top:${((-top / h) * 100).toFixed(4)}%`,
  ].join(";");
}

/** @deprecated */
export function regionToPanelVideoStyle(r: CropRegion): string {
  return `${regionToPanelStageStyle(r)};object-fit:cover`;
}
