/** Shared Visual panel types (frontend). */

export type Seg = {
  id?: string;
  text: string;
  span: { start: number; end: number };
};

export type PlacementLayout = {
  x: number;
  y: number;
  w: number;
  h: number;
  opacity: number;
};

export type VisualPlacement = {
  id: string;
  assetId: string;
  outputStart: number;
  outputEnd: number;
  mode: string;
  fit: string;
  status: string;
  provenance: string;
  suggestionId?: string | null;
  layout?: PlacementLayout;
  label?: string | null;
  thumbnailPath?: string | null;
};

export type ProtectedRange = {
  id: string;
  outputStart: number;
  outputEnd: number;
  reason: string;
};

export type VisualPlan = {
  id?: string;
  placements?: VisualPlacement[];
  protectedRanges?: ProtectedRange[];
  warnings?: string[];
  version?: number;
};

export type Suggestion = {
  id: string;
  assetId: string;
  matchScore: number;
  matchReasons: string[];
  status: string;
  assetTitle?: string;
  thumbnailPath?: string;
  outputSpan: { start: number; end: number };
  sourceSpan: { start: number; end: number };
};

export type Asset = {
  id: string;
  title: string;
  concepts: string[];
  tags: string[];
  thumbnailPath?: string;
  managedPath?: string;
  timesUsed?: number;
  licenseStatus?: string;
  status?: string;
};

export type DisplayMode = "completa" | "parcial" | "flotante";

export function fileUrl(path?: string | null): string | null {
  if (!path || typeof window === "undefined") return null;
  try {
    // dynamic import avoided — caller should use convertFileSrc
    return path;
  } catch {
    return null;
  }
}
