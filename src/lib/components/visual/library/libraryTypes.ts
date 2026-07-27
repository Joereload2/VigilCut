/** Tipos catálogo biblioteca (camelCase desde Rust MediaAsset). */

export type AssetStatus = "active" | "archived" | "blocked" | "missing" | "invalid";

export type LicenseStatus =
  | "owned"
  | "licensed"
  | "public_domain"
  | "attribution_required"
  | "unknown";

export type QaStatus =
  | "none"
  | "pending"
  | "automated_review"
  | "needs_human_review"
  | "approved"
  | "rejected";

export interface AssetProvenance {
  source: string;
  provider?: string | null;
  model?: string | null;
  prompt?: string | null;
  negativePrompt?: string | null;
  seed?: number | null;
  generatedAt?: string | null;
}

export interface MediaAsset {
  id: string;
  kind: string;
  managedPath: string;
  thumbnailPath?: string | null;
  sha256: string;
  title: string;
  description?: string | null;
  tags: string[];
  concepts: string[];
  category?: string | null;
  width: number;
  height: number;
  orientation: string;
  mimeType: string;
  fileSize: number;
  licenseStatus: LicenseStatus;
  source?: string | null;
  attribution?: string | null;
  timesUsed: number;
  lastUsedAt?: string | null;
  allowSameVideoRepeat: boolean;
  minimumVideosBeforeReuse: number;
  qualityScore?: number | null;
  status: AssetStatus;
  originalPath?: string | null;
  createdAt: string;
  updatedAt: string;
  literalDescription?: string[];
  meanings?: string[];
  positiveContexts?: string[];
  negativeContexts?: string[];
  hardExclusions?: string[];
  aspectRatio?: string | null;
  safeArea?: string | null;
  perceptualHash?: string | null;
  qaStatus?: QaStatus;
  technicalScore?: number | null;
  semanticScore?: number | null;
  provenance?: AssetProvenance | null;
  commercialUse?: boolean | null;
}

export interface ImportFolderResult {
  scanned: number;
  imported: number;
  duplicates: number;
  failed: number;
  assetIds: string[];
  errors: string[];
}

export interface AssetUsageRow {
  id: string;
  assetId: string;
  mediaPath: string;
  runId?: string | null;
  usedAt: string;
  outputStart?: number | null;
  outputEnd?: number | null;
}

export type VisualsViewId = "video" | "library" | "review";

export function licenseLabel(s?: string | null): string {
  switch (s) {
    case "owned":
      return "Propia";
    case "licensed":
      return "Licenciada";
    case "public_domain":
      return "Dominio público";
    case "attribution_required":
      return "Requiere atribución";
    default:
      return "Desconocida";
  }
}

export function formatBytes(n: number): string {
  if (!n || n < 0) return "—";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}
