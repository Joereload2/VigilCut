import type { LicenseStatus, MediaAsset } from "./libraryTypes";

export function normalizeTags(raw: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const part of raw.split(/[,;]/)) {
    const t = part.trim();
    if (!t) continue;
    const key = t.toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(t);
  }
  return out;
}

export function isAiOrigin(a: MediaAsset): boolean {
  const src = (a.source ?? "").toLowerCase();
  if (src.startsWith("ai:") || src.includes("ai_generated")) return true;
  const provenance = (a.provenance?.source ?? "").toLowerCase();
  if (provenance === "ai_generated" || provenance.endsWith("_generation")) return true;
  if (a.provenance?.provider) return true;
  return false;
}

export function isMockAsset(a: MediaAsset): boolean {
  const p = (a.provenance?.provider ?? "").toLowerCase();
  const src = (a.source ?? "").toLowerCase();
  return p === "mock" || src === "ai:mock" || src.includes("mock");
}

export function previewPath(a: MediaAsset): string | null {
  if (a.status === "missing") return null;
  return a.thumbnailPath || a.managedPath || null;
}

export type QuickFilter = "all" | "landscape" | "portrait" | "ai" | "imported";

export function filterAssets(
  assets: MediaAsset[],
  f: {
    quick: QuickFilter;
    status?: string;
    license?: LicenseStatus | "all";
    theme?: string;
    aspect?: string;
  },
): MediaAsset[] {
  return assets.filter((a) => {
    if (f.status && f.status !== "all" && a.status !== f.status) return false;
    if (f.license && f.license !== "all" && a.licenseStatus !== f.license) return false;
    if (f.theme && f.theme !== "all" && (a.category ?? "") !== f.theme) return false;
    if (f.aspect && f.aspect !== "all" && (a.aspectRatio ?? "") !== f.aspect) return false;
    switch (f.quick) {
      case "landscape":
        return a.orientation === "landscape" || a.width >= a.height;
      case "portrait":
        return a.orientation === "portrait" || a.height > a.width;
      case "ai":
        return isAiOrigin(a);
      case "imported":
        return !isAiOrigin(a);
      default:
        return true;
    }
  });
}

export function sortAssets(
  assets: MediaAsset[],
  sort: "recent" | "used" | "quality" | "title",
): MediaAsset[] {
  const arr = [...assets];
  arr.sort((a, b) => {
    switch (sort) {
      case "used":
        return (b.timesUsed ?? 0) - (a.timesUsed ?? 0);
      case "quality": {
        const qa = a.qualityScore;
        const qb = b.qualityScore;
        if (qa == null && qb == null) return 0;
        if (qa == null) return 1;
        if (qb == null) return -1;
        return qb - qa;
      }
      case "title":
        return (a.title || "").localeCompare(b.title || "", "es");
      case "recent":
      default:
        return (b.createdAt || "").localeCompare(a.createdAt || "");
    }
  });
  return arr;
}
