<script lang="ts">
  /**
   * Reproductor de un tramo [start,end] del video original.
   * Carga el archivo una sola vez; al cambiar pestaña solo hace seek.
   * boundUrl/handle NO son $state (evita re-ejecutar el effect en bucle).
   */
  import { onDestroy } from "svelte";
  import { pathToPlayableUrl } from "$lib/vnext/media/fileUrl";
  import {
    bindLocalVideo,
    playVideo,
    seekVideo,
    type LocalVideoHandle,
  } from "$lib/vnext/media/bindLocalVideo";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";

  let {
    sourcePath = null as string | null,
    start = 0,
    end = 10,
    autoplay = false,
  }: {
    sourcePath?: string | null;
    start?: number;
    end?: number;
    autoplay?: boolean;
  } = $props();

  let videoEl = $state<HTMLVideoElement | null>(null);
  let ready = $state(false);
  let playing = $state(false);
  let current = $state(0);
  let loadError = $state<string | null>(null);

  // NO $state — si no, asignar boundUrl re-dispara el effect y destruye el video
  let boundUrl: string | null = null;
  let handle: LocalVideoHandle | null = null;
  const spanLive = { start: 0, end: 10 };

  const src = $derived(pathToPlayableUrl(sourcePath));
  const span = $derived(Math.max(0.1, end - start));
  const localT = $derived(Math.max(0, Math.min(span, current - start)));

  $effect(() => {
    spanLive.start = start;
    spanLive.end = end;
  });

  // Solo re-bind cuando cambia la URL del archivo (deps: videoEl + src)
  $effect(() => {
    const v = videoEl;
    const url = src;

    if (!v) return;

    if (!url) {
      ready = false;
      loadError = sourcePath ? "No se pudo abrir el archivo de video" : "Sin ruta de video";
      handle?.destroy();
      handle = null;
      boundUrl = null;
      return;
    }

    if (boundUrl === url && handle) {
      return;
    }

    handle?.destroy();
    handle = null;
    ready = false;
    playing = false;
    loadError = null;
    boundUrl = url;

    handle = bindLocalVideo(v, url, {
      onReady: () => {
        ready = true;
        loadError = null;
        seekVideo(v, spanLive.start);
        if (autoplay) void playVideo(v).catch(() => {});
      },
      onError: (msg) => {
        loadError = msg;
      },
      onTime: () => {
        current = v.currentTime;
        // Si hay frames / tiempo, considerar listo (WebView a veces no dispara play/ready)
        if (!ready && (v.readyState >= 2 || v.videoWidth > 0 || v.currentTime > 0)) {
          ready = true;
          loadError = null;
        }
        if (v.currentTime >= spanLive.end - 0.04) {
          seekVideo(v, spanLive.start);
          if (!v.paused) void playVideo(v).catch(() => {});
        }
      },
      onPlay: () => {
        playing = true;
        ready = true;
        loadError = null;
      },
      onPause: () => {
        playing = false;
      },
    });

    return () => {
      // Solo al desmontar o cambiar URL real — no tocar boundUrl aquí
      // (el próximo run lo actualiza). Destruir listeners del handle actual.
      handle?.destroy();
      handle = null;
    };
  });

  // Solo seek al cambiar tramo (pestaña)
  $effect(() => {
    const v = videoEl;
    const s = start;
    void end;
    if (!v) return;
    if (v.paused || Math.abs(v.currentTime - s) > 0.35) {
      seekVideo(v, s);
      current = s;
    }
  });

  async function onToggleClick() {
    const v = videoEl;
    if (!v || !src) return;
    if (v.paused) {
      seekVideo(v, start);
      try {
        await playVideo(v);
        ready = true;
        loadError = null;
        playing = true;
      } catch (err) {
        loadError = String(err);
        playing = false;
      }
    } else {
      v.pause();
      playing = false;
    }
  }

  onDestroy(() => {
    handle?.destroy();
    handle = null;
    boundUrl = null;
    if (videoEl) {
      videoEl.pause();
      videoEl.removeAttribute("src");
      try {
        videoEl.load();
      } catch {
        /* */
      }
    }
  });
</script>

<div class="flex h-full min-h-0 flex-1 flex-col gap-1" data-testid="segment-player">
  <div
    class="relative min-h-0 flex-1 overflow-hidden rounded-lg border border-surface-800 bg-black"
    style="max-height: 100%"
  >
    <video
      bind:this={videoEl}
      class="h-full w-full bg-black object-contain"
      playsinline
      preload="auto"
    ></video>
    {#if loadError}
      <div
        class="absolute inset-0 flex items-center justify-center bg-black/80 px-4 text-center text-xs text-cut"
      >
        {loadError}
      </div>
    {:else if !ready}
      <div
        class="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/20 text-xs text-surface-400"
      >
        Cargando… (Play)
      </div>
    {/if}
  </div>
  <div class="flex shrink-0 items-center gap-1.5">
    <button
      type="button"
      class="rounded px-3 py-0.5 text-[11px] font-semibold transition
        {playing
        ? 'bg-vigil-600 text-white hover:bg-vigil-500'
        : 'bg-white/15 text-white hover:bg-white/25'}
        disabled:opacity-40"
      disabled={!src}
      data-testid="segment-play"
      onclick={() => void onToggleClick()}
    >{playing ? "Pausa" : "Play"}</button>
    <span class="font-mono text-[9px] text-surface-400">
      {formatDuration(localT)} / {formatDuration(span)}
      <span class="text-surface-600">
        · {formatDuration(start)}–{formatDuration(end)}
      </span>
    </span>
  </div>
</div>
