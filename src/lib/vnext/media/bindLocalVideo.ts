/**
 * Robust local-file video bind for Tauri webview.
 * - Loads once per URL
 * - Polls readyState (asset:// often skips some events)
 * - Does not clear src on seek-only updates
 */

export type LocalVideoHandle = {
  markReady: () => void;
  destroy: () => void;
};

export function bindLocalVideo(
  video: HTMLVideoElement,
  url: string,
  opts: {
    onReady?: () => void;
    onError?: (message: string) => void;
    onTime?: () => void;
    onPlay?: () => void;
    onPause?: () => void;
  } = {},
): LocalVideoHandle {
  let destroyed = false;
  let readySent = false;

  const markReady = () => {
    if (destroyed || readySent) return;
    if (video.readyState < 1 && video.videoWidth === 0) return;
    readySent = true;
    opts.onReady?.();
  };

  const onError = () => {
    if (destroyed) return;
    const code = video.error?.code;
    const msg =
      code === 4
        ? "Formato no soportado o ruta inaccesible."
        : "No se pudo cargar el video (ruta o permisos).";
    opts.onError?.(msg);
  };

  const onTime = () => {
    if (!destroyed) opts.onTime?.();
  };
  const onPlay = () => {
    if (!destroyed) {
      markReady();
      opts.onPlay?.();
    }
  };
  const onPause = () => {
    if (!destroyed) opts.onPause?.();
  };

  video.addEventListener("loadedmetadata", markReady);
  video.addEventListener("loadeddata", markReady);
  video.addEventListener("canplay", markReady);
  video.addEventListener("canplaythrough", markReady);
  video.addEventListener("error", onError);
  video.addEventListener("timeupdate", onTime);
  video.addEventListener("play", onPlay);
  video.addEventListener("pause", onPause);

  // Force software-friendly paint in WebView2
  video.style.transform = "translateZ(0)";
  video.playsInline = true;
  video.preload = "auto";

  if (video.getAttribute("src") !== url && video.src !== url) {
    video.src = url;
    try {
      video.load();
    } catch {
      /* */
    }
  }

  // Poll: asset protocol sometimes never fires events even with a painted frame
  const poll = window.setInterval(() => {
    if (destroyed) {
      window.clearInterval(poll);
      return;
    }
    if (video.readyState >= 1 || video.videoWidth > 0) {
      markReady();
      window.clearInterval(poll);
    }
  }, 120);
  const timeout = window.setTimeout(() => window.clearInterval(poll), 15000);

  if (video.readyState >= 1 || video.videoWidth > 0) {
    markReady();
  }

  return {
    markReady,
    destroy() {
      destroyed = true;
      window.clearInterval(poll);
      window.clearTimeout(timeout);
      video.removeEventListener("loadedmetadata", markReady);
      video.removeEventListener("loadeddata", markReady);
      video.removeEventListener("canplay", markReady);
      video.removeEventListener("canplaythrough", markReady);
      video.removeEventListener("error", onError);
      video.removeEventListener("timeupdate", onTime);
      video.removeEventListener("play", onPlay);
      video.removeEventListener("pause", onPause);
    },
  };
}

export async function playVideo(video: HTMLVideoElement): Promise<void> {
  try {
    // Unmute on explicit user gesture so WebView actually shows frames
    video.muted = false;
    await video.play();
  } catch (e) {
    // Retry muted once (policy), then unmute on next gesture
    try {
      video.muted = true;
      await video.play();
    } catch {
      throw e;
    }
  }
}

export function seekVideo(video: HTMLVideoElement, t: number) {
  try {
    const target = Math.max(0, t);
    if (!Number.isFinite(target)) return;
    if (Math.abs(video.currentTime - target) > 0.05) {
      video.currentTime = target;
    }
  } catch {
    /* not ready */
  }
}
