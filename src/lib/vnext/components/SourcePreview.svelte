<script lang="ts">
  /**
   * Source stage (16:9) with a single 9:16 selection window.
   * Dim outside crop · play/pause · drag to reframe.
   */
  import { onDestroy } from "svelte";
  import { pathToPlayableUrl } from "$lib/vnext/media/fileUrl";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";

  let {
    sourcePath = null as string | null,
    start = 0,
    end = null as number | null,
    centerX = 0.5,
    centerY = 0.45,
    autoplay = false,
    interactive = true,
    onCenterChange,
  }: {
    sourcePath?: string | null;
    start?: number;
    end?: number | null;
    centerX?: number;
    centerY?: number;
    autoplay?: boolean;
    interactive?: boolean;
    onCenterChange?: (x: number, y: number) => void;
  } = $props();

  let videoEl = $state<HTMLVideoElement | null>(null);
  let wrapEl = $state<HTMLDivElement | null>(null);
  let ready = $state(false);
  let playing = $state(false);
  let dragging = $state(false);
  let current = $state(0);

  const src = $derived(pathToPlayableUrl(sourcePath));
  const endT = $derived(end != null && end > start ? end : null);

  /** 9:16 strip width on a 16:9 stage ≈ 31.6% of width; full height. */
  const cropW = (9 / 16) / (16 / 9) * 100;
  const cropLeft = $derived(
    Math.min(100 - cropW, Math.max(0, centerX * 100 - cropW / 2)),
  );

  $effect(() => {
    const v = videoEl;
    const url = src;
    ready = false;
    playing = false;
    if (!v || !url) return;
    v.src = url;
    v.load();
    const onMeta = () => {
      ready = true;
      try {
        v.currentTime = start;
      } catch {
        /* */
      }
      if (autoplay) void v.play().catch(() => {});
    };
    const onTime = () => {
      current = v.currentTime;
      if (endT != null && v.currentTime >= endT - 0.05) {
        v.currentTime = start;
        if (!v.paused) void v.play().catch(() => {});
      }
    };
    const onPlay = () => (playing = true);
    const onPause = () => (playing = false);
    v.addEventListener("loadedmetadata", onMeta);
    v.addEventListener("timeupdate", onTime);
    v.addEventListener("play", onPlay);
    v.addEventListener("pause", onPause);
    return () => {
      v.removeEventListener("loadedmetadata", onMeta);
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("play", onPlay);
      v.removeEventListener("pause", onPause);
      v.pause();
      v.removeAttribute("src");
      v.load();
    };
  });

  $effect(() => {
    const v = videoEl;
    if (!v || !ready) return;
    // Only snap to start when segment changes, not every tick
    if (Math.abs(v.currentTime - start) > 0.5 && v.paused) {
      try {
        v.currentTime = start;
      } catch {
        /* */
      }
    }
  });

  function toggle() {
    const v = videoEl;
    if (!v || !ready) return;
    if (v.paused) void v.play().catch(() => {});
    else v.pause();
  }

  function pointerToCenter(e: PointerEvent) {
    const el = wrapEl;
    if (!el || !onCenterChange) return;
    const r = el.getBoundingClientRect();
    const x = Math.min(0.92, Math.max(0.08, (e.clientX - r.left) / r.width));
    // Keep Y available for vertical object-position in 9:16 preview
    const y = Math.min(0.92, Math.max(0.08, (e.clientY - r.top) / r.height));
    onCenterChange(x, y);
  }

  function onPointerDown(e: PointerEvent) {
    if (!interactive || !onCenterChange) return;
    if ((e.target as HTMLElement).closest("button")) return;
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    pointerToCenter(e);
  }
  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    pointerToCenter(e);
  }
  function onPointerUp(e: PointerEvent) {
    dragging = false;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* */
    }
  }

  onDestroy(() => {
    if (videoEl) {
      videoEl.pause();
      videoEl.removeAttribute("src");
    }
  });
</script>

<div class="flex h-full min-h-0 w-full flex-col gap-1.5" data-testid="source-preview">
  <div
    bind:this={wrapEl}
    class="relative min-h-0 w-full flex-1 cursor-crosshair overflow-hidden rounded-xl border border-surface-700 bg-black"
    style="aspect-ratio: 16/9; max-height: 100%"
    role="presentation"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
  >
    {#if !src}
      <div class="flex h-full items-center justify-center text-xs text-surface-500">
        Sin video fuente
      </div>
    {:else}
      <video
        bind:this={videoEl}
        class="h-full w-full object-contain bg-black"
        playsinline
        muted={false}
      ></video>
      {#if !ready}
        <div class="absolute inset-0 flex items-center justify-center text-xs text-surface-400">
          Cargando fuente…
        </div>
      {:else}
        <!-- Dim left of crop -->
        <div
          class="pointer-events-none absolute inset-y-0 left-0 bg-black/55"
          style="width: {cropLeft}%"
        ></div>
        <!-- Dim right of crop -->
        <div
          class="pointer-events-none absolute inset-y-0 right-0 bg-black/55"
          style="width: {Math.max(0, 100 - cropLeft - cropW)}%"
        ></div>
        <!-- Active 9:16 selection -->
        <div
          class="pointer-events-none absolute top-0 bottom-0 rounded-md border-[3px] border-vigil-500 shadow-[0_0_0_1px_rgba(0,0,0,0.6)]"
          style="left: {cropLeft}%; width: {cropW}%"
          data-testid="crop-active"
        >
          <span
            class="absolute left-1 top-1 rounded bg-vigil-600/90 px-1.5 py-0.5 text-[9px] font-bold text-white"
            >9:16</span
          >
        </div>
      {/if}
    {/if}
  </div>
  <div class="flex shrink-0 items-center gap-2">
    <button
      type="button"
      class="rounded-lg bg-white/10 px-3 py-1 text-xs font-semibold text-white hover:bg-white/20 disabled:opacity-40"
      disabled={!ready}
      onclick={toggle}
      data-testid="source-play"
    >
      {playing ? "Pausa" : "Play"}
    </button>
    <span class="font-mono text-[10px] text-surface-500">
      {formatDuration(Math.max(0, current - start))}
      {#if endT != null}
        <span class="text-surface-600"> / {formatDuration(endT - start)}</span>
      {/if}
    </span>
    {#if interactive && onCenterChange}
      <span class="text-[10px] text-surface-600">Arrastrá en el video para elegir el recorte 9:16</span>
    {/if}
  </div>
</div>
