import { convertFileSrc } from "@tauri-apps/api/core";

/** Convert a local filesystem path to a URL the webview can play. */
export function pathToPlayableUrl(path: string | null | undefined): string | null {
  if (!path || path.startsWith("demo://")) return null;
  if (
    path.startsWith("blob:") ||
    path.startsWith("http://") ||
    path.startsWith("https://") ||
    path.startsWith("asset:") ||
    path.startsWith("asset://")
  ) {
    return path;
  }
  const isTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  if (!isTauri) return null;
  try {
    // Prefer original Windows path — convertFileSrc handles it
    return convertFileSrc(path);
  } catch {
    try {
      return convertFileSrc(path.replace(/\\/g, "/"));
    } catch {
      return null;
    }
  }
}
