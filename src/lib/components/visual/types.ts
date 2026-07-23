/** Shared Visual / composition types (mirrors Rust VisualPlan — not Segment legacy). */

export type Seg = {
  id?: string;
  text: string;
  span: { start: number; end: number };
  words?: { text: string; span: { start: number; end: number } }[];
};

export type PlacementLayout = {
  x: number;
  y: number;
  w: number;
  h: number;
  opacity: number;
};

export type ReviewStatus = "pending" | "approved" | "conflict" | "rejected";

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
  /** Composition supervision fields */
  semanticEventId?: string | null;
  relatedText?: string | null;
  sourceStart?: number | null;
  sourceEnd?: number | null;
  confidence?: number;
  reviewStatus?: ReviewStatus;
  manualOverride?: boolean;
  avoidZones?: string[];
  suggestedLayout?: PlacementLayout | null;
  suggestedMode?: string | null;
};

export type ProtectedRange = {
  id: string;
  outputStart: number;
  outputEnd: number;
  reason: string;
};

export type SpatialZone = {
  id: string;
  kind: string;
  x: number;
  y: number;
  w: number;
  h: number;
  outputStart?: number | null;
  outputEnd?: number | null;
  label?: string | null;
  severity?: string;
};

export type CompositionIssue = {
  id: string;
  placementId: string;
  kind: string;
  severity: string;
  message: string;
  suggestedX?: number | null;
  suggestedY?: number | null;
  suggestedW?: number | null;
};

export type VisualPlan = {
  id?: string;
  placements?: VisualPlacement[];
  protectedRanges?: ProtectedRange[];
  spatialZones?: SpatialZone[];
  issues?: CompositionIssue[];
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

/** UI: fullscreen | overlay (parcial/flotante) */
export type DisplayMode = "completa" | "parcial" | "flotante" | "fullscreen" | "overlay";

export function isFullscreenMode(mode?: string): boolean {
  const m = (mode || "").toLowerCase();
  return m === "fullframe" || m === "completa" || m === "fullscreen" || m === "full";
}

export function isOverlayMode(mode?: string): boolean {
  return !isFullscreenMode(mode);
}

export function reviewLabel(rs?: ReviewStatus | string): string {
  switch (rs) {
    case "approved":
      return "Aprobado";
    case "conflict":
      return "Conflicto";
    case "rejected":
      return "Rechazado";
    default:
      return "Pendiente";
  }
}
