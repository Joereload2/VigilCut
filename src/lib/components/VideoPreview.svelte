<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isTauri } from "$lib/utils/tauri";

  let videoEl = $state<HTMLVideoElement | null>(null);
  let loadError = $state<string | null>(null);
  let ready = $state(false);
  /** Avoid feedback loops when we set currentTime from code */
  let seekingProgrammatically = $state(false);

  const isEdited = $derived(projectStore.previewMode === "edited");
  const editedDuration = $derived(Math.max(projectStore.keptDuration, 0.001));
  const editedClock = $derived(projectStore.sourceToEdited(projectStore.currentTime));
  const displayDuration = $derived(isEdited ? editedDuration : Math.max(projectStore.duration, 0.001));
  const displayClock = $derived(isEdited ? editedClock : projectStore.currentTime);

  const src = $derived.by(() => {
    const p = projectStore.mediaPath;
    if (!p || p.startsWith("demo://")) return null;
    if (isTauri()) {
      try {
        // Keep native separators; convertFileSrc handles Windows paths
        const url = convertFileSrc(p);
        return url;
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
  });

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

  // External seek from timeline (source time) when paused
  $effect(() => {
    const v = videoEl;
    if (!v || !ready || projectStore.isPlaying) return;
    const t = projectStore.currentTime;
    if (Math.abs(v.currentTime - t) > 0.12) {
      setSourceTime(
        projectStore.previewMode === "edited"
          ? projectStore.advanceEditedPlayback(t).time
          : t,
      );
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
      projectStore.statusMessage = "Vista del video cortado";
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

<div class="panel flex min-h-0 flex-1 flex-col overflow-hidden">
  <div class="flex flex-wrap items-center justify-between gap-2 border-b border-surface-800 px-3 py-1.5">
    <span class="text-xs font-semibold text-surface-300">Vista previa</span>

    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="btn h-8 bg-vigil-600 px-3 text-xs font-bold text-white hover:bg-vigil-500 disabled:opacity-40"
        disabled={!canPreviewCut || !src}
        onclick={listenCutResult}
        title="Reproduce el resultado final (salta cortes)"
      >
        ▶ Oír video cortado
      </button>
      <div class="flex items-center rounded-lg border border-surface-700 bg-surface-950 p-0.5 text-[11px]">
        <button
          type="button"
          class="rounded-md px-2.5 py-1 font-medium transition
            {!isEdited ? 'bg-surface-700 text-white' : 'text-surface-400 hover:text-surface-200'}"
          onclick={() => setMode("original")}
        >
          Original
        </button>
        <button
          type="button"
          class="rounded-md px-2.5 py-1 font-medium transition
            {isEdited ? 'bg-vigil-600 text-white' : 'text-surface-400 hover:text-surface-200'}"
          onclick={() => setMode("edited")}
          disabled={!canPreviewCut}
          title={canPreviewCut
            ? "Resultado final saltando cortes"
            : "Marca tramos a mantener"}
        >
          Cortado
        </button>
      </div>
    </div>
  </div>

  <div
    class="relative flex min-h-[240px] flex-1 items-center justify-center overflow-hidden bg-black lg:min-h-[320px]"
  >
    {#if src}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={videoEl}
        class="h-full max-h-full w-full max-w-full cursor-pointer object-contain outline-none"
        playsinline
        preload="auto"
        controls={false}
        onclick={togglePlay}
      ></video>

      {#if !projectStore.isPlaying && ready}
        <button
          type="button"
          class="pointer-events-auto absolute inset-0 flex items-center justify-center bg-black/20"
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
          class="pointer-events-none absolute left-3 top-3 rounded-full border border-vigil-600/50 bg-vigil-950/90 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide text-vigil-300"
        >
          Resultado · pausas fuera
          {#if projectStore.autoCutCount > 0}
            · {projectStore.autoCutCount} cortes
          {/if}
        </div>
      {:else if ready}
        <div
          class="pointer-events-none absolute left-3 top-3 rounded-full border border-surface-600/50 bg-surface-950/80 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wide text-surface-300"
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

  <!-- Scrubber: edited timeline when in cut preview -->
  <div class="border-t border-surface-800 px-3 pt-2">
    <input
      type="range"
      class="w-full accent-vigil-500 disabled:opacity-40"
      min={0}
      max={displayDuration}
      step={0.05}
      value={displayClock}
      disabled={!src || !ready}
      oninput={onScrub}
    />
    <div class="mt-0.5 flex justify-between text-[10px] text-surface-500">
      <span>{isEdited ? "Línea del video final" : "Línea del original"}</span>
      <span class="font-mono"
        >{formatTime(displayClock)} / {formatTime(displayDuration)}</span
      >
    </div>
  </div>

  <div class="flex items-center gap-2 px-3 py-2">
    <button class="btn-ghost px-2 text-xs" onclick={() => seekDisplay(-5)} disabled={!src}
      >−5s</button
    >
    <button
      class="btn-secondary min-w-[4.5rem] font-semibold"
      onclick={togglePlay}
      disabled={!src || (isEdited && !canPreviewCut)}
      title="Espacio"
    >
      {projectStore.isPlaying ? "Pausa" : "Play"}
    </button>
    <button class="btn-ghost px-2 text-xs" onclick={() => seekDisplay(5)} disabled={!src}
      >+5s</button
    >

    <span class="font-mono text-xs text-surface-300">
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
