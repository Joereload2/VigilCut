<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isTauri } from "$lib/utils/tauri";
  import VisualLiveOverlay from "./visual/VisualLiveOverlay.svelte";

  interface Props {
    /**
     * Compact: fixed max height so timeline + tools always fit on screen.
     * Video scales with object-contain (not 1:1 pixels).
     */
    compact?: boolean;
  }
  let { compact = true }: Props = $props();

  let videoEl = $state<HTMLVideoElement | null>(null);
  let loadError = $state<string | null>(null);
  let ready = $state(false);
  /** Avoid feedback loops when we set currentTime from code */
  let seekingProgrammatically = $state(false);

  const isEdited = $derived(projectStore.previewMode === "edited");
  const hasLiveVisual = $derived(
    projectStore.visualPlacements.some((p) => p.status === "active"),
  );
  const editedDuration = $derived(Math.max(projectStore.keptDuration, 0.001));
  const editedClock = $derived(
    projectStore.localKeepRanges().length > 0
      ? projectStore.sourceToEdited(projectStore.currentTime)
      : projectStore.currentTime,
  );
  const displayDuration = $derived(
    isEdited ? editedDuration : Math.max(projectStore.duration, 0.001),
  );
  const displayClock = $derived(isEdited ? editedClock : projectStore.currentTime);

  function fileToSrc(p: string | null): string | null {
    if (!p || p.startsWith("demo://")) return null;
    if (isTauri()) {
      try {
        // Keep native separators; convertFileSrc handles Windows paths
        return convertFileSrc(p);
      } catch (e) {
        console.error("convertFileSrc failed", e, p);
        try {
          return convertFileSrc(p.replace(/\\/g, "/"));
        } catch (e2) {
          console.error("convertFileSrc fallback failed", e2);
          return null;
        }
      }
    }
    if (p.startsWith("blob:") || p.startsWith("http")) return p;
    return null;
  }

  /** One video only: the project media (cut preview skips silences in “Cortado”). */
  const src = $derived.by(() => fileToSrc(projectStore.mediaPath));

  const canPreviewCut = $derived(
    projectStore.localKeepRanges().length > 0 ||
      (projectStore.segments.length > 0 && projectStore.keepCount > 0) ||
      (projectStore.keepRanges?.length ?? 0) > 0,
  );

  $effect(() => {
    const v = videoEl;
    const url = src;
    loadError = null;
    ready = false;
    if (!v || !url) return;
    v.pause();
    v.removeAttribute("src");
    v.load();
    v.src = url;
    v.load();
  });

  // When switching to edited mode, snap out of any CUT region
  $effect(() => {
    if (!ready || !videoEl) return;
    if (projectStore.previewMode !== "edited") return;
    const { time } = projectStore.advanceEditedPlayback(projectStore.currentTime);
    if (Math.abs(time - projectStore.currentTime) > 0.04) {
      setSourceTime(time);
    }
  });

  function setSourceTime(t: number) {
    const v = videoEl;
    projectStore.currentTime = t;
    if (!v || !ready) return;
    seekingProgrammatically = true;
    try {
      v.currentTime = t;
    } catch {
      /* ignore */
    }
    // release flag on next frame
    requestAnimationFrame(() => {
      seekingProgrammatically = false;
    });
  }

  $effect(() => {
    const v = videoEl;
    if (!v) return;

    const onTime = () => {
      if (seekingProgrammatically) return;
      let t = v.currentTime;

      if (projectStore.previewMode === "edited" && projectStore.isPlaying) {
        const { time, ended } = projectStore.advanceEditedPlayback(t);
        if (ended) {
          v.pause();
          setSourceTime(time);
          projectStore.isPlaying = false;
          projectStore.statusMessage = "Fin de la previsualización cortada";
          return;
        }
        if (Math.abs(time - t) > 0.04) {
          seekingProgrammatically = true;
          v.currentTime = time;
          t = time;
          requestAnimationFrame(() => {
            seekingProgrammatically = false;
          });
        }
      }

      projectStore.currentTime = t;
    };

    const onPlay = () => {
      projectStore.isPlaying = true;
      if (projectStore.previewMode === "edited") {
        const { time, ended } = projectStore.advanceEditedPlayback(v.currentTime);
        if (ended) {
          // Restart edited preview from beginning
          const ranges = projectStore.localKeepRanges();
          const start = ranges[0]?.[0] ?? 0;
          setSourceTime(start);
        } else if (Math.abs(time - v.currentTime) > 0.04) {
          setSourceTime(time);
        }
      }
    };
    const onPause = () => {
      projectStore.isPlaying = false;
    };
    const onLoaded = () => {
      ready = true;
      loadError = null;
      if (projectStore.previewMode === "edited") {
        const { time } = projectStore.advanceEditedPlayback(projectStore.currentTime);
        setSourceTime(time);
      } else if (projectStore.currentTime > 0.05) {
        setSourceTime(projectStore.currentTime);
      }
    };
    const onError = () => {
      ready = false;
      const code = v.error?.code;
      const map: Record<number, string> = {
        1: "Carga cancelada",
        2: "No se pudo leer el archivo",
        3: "Error al decodificar",
        4: "Formato no compatible. Usa MP4 (H.264 + AAC).",
      };
      loadError = map[code ?? 0] ?? "No se pudo cargar el video";
    };
    const onEnded = () => {
      // Source file ended — in edited mode we usually end earlier via advanceEditedPlayback
      projectStore.isPlaying = false;
    };

    v.addEventListener("timeupdate", onTime);
    // denser ticks while playing cut preview (WebView sometimes throttles timeupdate)
    let raf = 0;
    let rafLive = true;
    const tick = () => {
      if (!rafLive) return;
      if (projectStore.isPlaying && projectStore.previewMode === "edited") {
        onTime();
      }
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);

    v.addEventListener("play", onPlay);
    v.addEventListener("pause", onPause);
    v.addEventListener("loadeddata", onLoaded);
    v.addEventListener("error", onError);
    v.addEventListener("ended", onEnded);
    return () => {
      rafLive = false;
      cancelAnimationFrame(raf);
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("play", onPlay);
      v.removeEventListener("pause", onPause);
      v.removeEventListener("loadeddata", onLoaded);
      v.removeEventListener("error", onError);
      v.removeEventListener("ended", onEnded);
    };
  });

  // External seek from timeline when paused
  $effect(() => {
    const v = videoEl;
    if (!v || !ready || projectStore.isPlaying) return;
    const t = projectStore.currentTime;
    if (Math.abs(v.currentTime - t) > 0.12) {
      if (projectStore.previewMode === "edited") {
        setSourceTime(projectStore.advanceEditedPlayback(t).time);
      } else {
        setSourceTime(t);
      }
    }
  });

  async function togglePlay() {
    const v = videoEl;
    if (!v || !src) return;
    try {
      if (v.paused) {
        if (projectStore.previewMode === "edited") {
          if (!canPreviewCut) {
            projectStore.statusMessage = "Marca tramos a mantener para previsualizar el corte";
            return;
          }
          const { time, ended } = projectStore.advanceEditedPlayback(v.currentTime);
          if (ended) {
            const start = projectStore.localKeepRanges()[0]?.[0] ?? 0;
            setSourceTime(start);
          } else {
            setSourceTime(time);
          }
        }
        await v.play();
      } else {
        v.pause();
      }
    } catch (e) {
      loadError = `No se pudo reproducir: ${e}`;
      projectStore.isPlaying = false;
    }
  }

  $effect(() => {
    const handler = () => void togglePlay();
    window.addEventListener("vigilcut:toggle-play", handler);
    return () => window.removeEventListener("vigilcut:toggle-play", handler);
  });

  function seekDisplay(delta: number) {
    if (isEdited) {
      const next = Math.max(0, Math.min(editedDuration, editedClock + delta));
      setSourceTime(projectStore.editedToSource(next));
    } else {
      const next = Math.max(0, Math.min(projectStore.duration, projectStore.currentTime + delta));
      setSourceTime(next);
    }
  }

  function onScrub(e: Event) {
    const val = Number((e.currentTarget as HTMLInputElement).value);
    if (isEdited) {
      setSourceTime(projectStore.editedToSource(val));
    } else {
      setSourceTime(val);
    }
  }

  function setMode(mode: "original" | "edited") {
    if (mode === "edited" && !canPreviewCut) {
      projectStore.statusMessage = "Necesitas tramos en “Mantener” para ver el video cortado";
      return;
    }
    projectStore.previewMode = mode;
    if (mode === "edited") {
      const { time } = projectStore.advanceEditedPlayback(projectStore.currentTime);
      setSourceTime(time);
      projectStore.statusMessage = hasLiveVisual
        ? "Resultado (cortes + imágenes en vivo)"
        : "Vista del video cortado";
    } else {
      projectStore.statusMessage = "Vista del original";
    }
  }

  /** Primary trust CTA: force cut preview from start and play. */
  async function listenCutResult() {
    if (!canPreviewCut || !videoEl || !src) {
      projectStore.statusMessage = "Marca tramos a mantener para oír el resultado";
      return;
    }
    projectStore.previewMode = "edited";
    const start = projectStore.localKeepRanges()[0]?.[0] ?? 0;
    setSourceTime(start);
    projectStore.statusMessage = "Oyendo el video cortado…";
    try {
      await videoEl.play();
    } catch (e) {
      loadError = `No se pudo reproducir: ${e}`;
    }
  }

  // External request from ActionBar
  $effect(() => {
    const handler = () => void listenCutResult();
    window.addEventListener("vigilcut:listen-result", handler);
    return () => window.removeEventListener("vigilcut:listen-result", handler);
  });

  // Clipping panel: jump to candidate start (optional end → auto-pause)
  let clipEndStop = $state<number | null>(null);

  $effect(() => {
    const handler = (ev: Event) => {
      const detail = (ev as CustomEvent<{ t?: number; end?: number; play?: boolean }>).detail;
      const t = detail?.t;
      if (typeof t !== "number" || !Number.isFinite(t)) return;
      projectStore.previewMode = "original";
      setSourceTime(t);
      clipEndStop =
        typeof detail?.end === "number" && Number.isFinite(detail.end) ? detail.end : null;
      const shouldPlay = detail?.play !== false;
      if (shouldPlay && videoEl && src) {
        void videoEl.play().catch(() => {
          /* autoplay policies */
        });
      }
    };
    window.addEventListener("vigilcut:play-from", handler);
    return () => window.removeEventListener("vigilcut:play-from", handler);
  });

  $effect(() => {
    const v = videoEl;
    if (!v) return;
    const onTimeClip = () => {
      if (clipEndStop == null) return;
      if (v.currentTime >= clipEndStop - 0.05) {
        v.pause();
        setSourceTime(clipEndStop);
        clipEndStop = null;
        projectStore.statusMessage = "Fin del clip";
      }
    };
    v.addEventListener("timeupdate", onTimeClip);
    return () => v.removeEventListener("timeupdate", onTimeClip);
  });
</script>

<div
  class="panel flex h-full min-h-0 w-full min-w-0 flex-col overflow-hidden"
  style="box-sizing:border-box; height:100%"
>
  <div
    class="flex shrink-0 flex-wrap items-center justify-between gap-1 border-b border-surface-800 px-2 py-1 sm:px-3 sm:py-1.5"
  >
    <span class="text-[11px] font-semibold text-surface-300 sm:text-xs">Vista previa</span>

    <div class="flex flex-wrap items-center gap-1 sm:gap-2">
      <button
        type="button"
        class="btn h-7 border border-surface-600 bg-surface-800 px-2 text-[11px] font-bold text-white hover:bg-surface-700 disabled:opacity-40 sm:h-8 sm:px-3 sm:text-xs"
        disabled={!src || !ready}
        onclick={togglePlay}
        title="Espacio — play / pausa"
      >
        {projectStore.isPlaying ? "⏸ Pausa" : "▶ Play"}
      </button>
      <button
        type="button"
        class="btn h-7 bg-vigil-600 px-2 text-[11px] font-bold text-white hover:bg-vigil-500 disabled:opacity-40 sm:h-8 sm:px-3 sm:text-xs"
        disabled={!canPreviewCut || !src}
        onclick={listenCutResult}
        title="Reproduce el resultado final (salta cortes)"
      >
        ▶ Oír cortado
      </button>
      <div class="flex items-center rounded-lg border border-surface-700 bg-surface-950 p-0.5 text-[10px] sm:text-[11px]">
        <button
          type="button"
          class="rounded-md px-2 py-0.5 font-medium transition
            {!isEdited ? 'bg-surface-700 text-white' : 'text-surface-400 hover:text-surface-200'}"
          onclick={() => setMode("original")}
        >
          Original
        </button>
        <button
          type="button"
          class="rounded-md px-2 py-0.5 font-medium transition
            {isEdited ? 'bg-vigil-600 text-white' : 'text-surface-400 hover:text-surface-200'}"
          onclick={() => setMode("edited")}
          disabled={!canPreviewCut}
          title={canPreviewCut
            ? "Un solo resultado: cortes + imágenes superpuestas"
            : "Marca tramos a mantener"}
        >
          {hasLiveVisual ? "Resultado" : "Cortado"}
        </button>
      </div>
    </div>
  </div>

  <!-- Stage fills remaining panel height (parent controls size — 70% column layout) -->
  <div
    class="relative flex min-h-0 w-full flex-1 items-center justify-center overflow-hidden bg-black"
  >
    {#if src}
      <!-- Single video layer -->
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={videoEl}
        class="h-full max-h-full w-full max-w-full cursor-pointer object-contain outline-none"
        playsinline
        preload="auto"
        controls={false}
        onclick={togglePlay}
      ></video>

      <!-- Live B-roll: same composition model as FFmpeg export -->
      <VisualLiveOverlay
        issues={projectStore.visualIssues}
        spatialZones={projectStore.visualSpatialZones}
        onLayoutChange={(id, x, y, w, h) => {
          // Optimistic local update for snappy drag
          projectStore.visualPlacements = projectStore.visualPlacements.map((p) =>
            p.id === id
              ? {
                  ...p,
                  layout: {
                    x,
                    y,
                    w,
                    h: h ?? p.layout?.h ?? 0,
                    opacity: p.layout?.opacity ?? 0.95,
                  },
                }
              : p,
          );
          window.dispatchEvent(
            new CustomEvent("vigilcut:visual-layout", {
              detail: { id, x, y, w, h },
            }),
          );
        }}
      />

      {#if !projectStore.isPlaying && ready}
        <!-- Keep play chrome under B-roll (z-15+) so fullscreen images stay visible -->
        <button
          type="button"
          class="pointer-events-auto absolute inset-0 z-[8] flex items-center justify-center bg-black/15"
          class:opacity-0={hasLiveVisual &&
            projectStore.visualPlacements.some(
              (p) =>
                p.status === "active" &&
                projectStore.outputClock() >= p.outputStart &&
                projectStore.outputClock() < p.outputEnd,
            )}
          class:pointer-events-none={hasLiveVisual &&
            projectStore.visualPlacements.some(
              (p) =>
                p.status === "active" &&
                projectStore.outputClock() >= p.outputStart &&
                projectStore.outputClock() < p.outputEnd,
            )}
          onclick={togglePlay}
          aria-label="Reproducir"
        >
          <span
            class="flex h-14 w-14 items-center justify-center rounded-full bg-vigil-600/90 text-2xl text-white shadow-xl"
            >▶</span
          >
        </button>
      {/if}

      {#if isEdited && canPreviewCut}
        <div
          class="pointer-events-none absolute left-3 top-3 z-20 rounded-full border border-vigil-600/50 bg-vigil-950/90 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide text-vigil-300"
        >
          {#if hasLiveVisual}
            Resultado · en vivo
            {#if projectStore.autoCutCount > 0}
              · {projectStore.autoCutCount} cortes
            {/if}
            · {projectStore.visualPlacements.filter((p) => p.status === "active").length} img
          {:else}
            Resultado · pausas fuera
            {#if projectStore.autoCutCount > 0}
              · {projectStore.autoCutCount} cortes
            {/if}
          {/if}
        </div>
      {:else if ready}
        <div
          class="pointer-events-none absolute left-3 top-3 z-20 rounded-full border border-surface-600/50 bg-surface-950/80 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide text-surface-300"
        >
          Original (con silencios)
        </div>
      {/if}

      {#if !ready && !loadError}
        <div class="pointer-events-none absolute inset-0 flex items-center justify-center">
          <span class="rounded-full bg-surface-950/70 px-3 py-1 text-xs text-surface-300"
            >Cargando video…</span
          >
        </div>
      {/if}
      {#if loadError}
        <div
          class="absolute inset-x-4 bottom-4 rounded-lg border border-cut/40 bg-surface-950/95 p-3 text-center text-xs text-cut"
        >
          {loadError}
          <p class="mt-1 break-all text-[10px] text-surface-500">{projectStore.mediaPath}</p>
        </div>
      {/if}
    {:else}
      <div class="p-6 text-center text-sm text-surface-500">
        Abre un video para previsualizar el original y el resultado cortado.
        {#if projectStore.mediaPath}
          <p class="mt-2 text-xs text-cut">No se pudo crear URL de previsualización.</p>
        {/if}
      </div>
    {/if}
  </div>

  <!-- One compact transport row (no second tall control band) -->
  <div
    class="flex shrink-0 flex-wrap items-center gap-1.5 border-t border-surface-800 px-2 py-1 sm:gap-2 sm:px-3"
  >
    <button class="btn-ghost px-1.5 text-[10px]" onclick={() => seekDisplay(-5)} disabled={!src}
      >−5s</button
    >
    <button
      class="btn-secondary min-w-[3.5rem] px-2 py-1 text-[11px] font-semibold"
      onclick={togglePlay}
      disabled={!src || (isEdited && !canPreviewCut)}
      title="Espacio"
    >
      {projectStore.isPlaying ? "Pausa" : "Play"}
    </button>
    <button class="btn-ghost px-1.5 text-[10px]" onclick={() => seekDisplay(5)} disabled={!src}
      >+5s</button
    >
    <input
      type="range"
      class="min-w-[6rem] max-w-full flex-1 accent-vigil-500 disabled:opacity-40"
      min={0}
      max={displayDuration}
      step={0.05}
      value={displayClock}
      disabled={!src || !ready}
      oninput={onScrub}
    />
    <span class="shrink-0 font-mono text-[10px] text-surface-300">
      {formatTime(displayClock, true)}
      <span class="text-surface-600">/</span>
      {formatTime(displayDuration)}
    </span>

    {#if isEdited}
      <span class="ml-auto text-[10px] text-vigil-400/90">
        −{formatTime(projectStore.cutDuration)} vs original
      </span>
    {/if}
  </div>
</div>
