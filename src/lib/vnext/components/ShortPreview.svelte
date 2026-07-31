<script lang="ts">
  import { onDestroy } from "svelte";
  import { pathToPlayableUrl } from "$lib/vnext/media/fileUrl";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";

  interface Props {
    sourcePath: string | null;
    start?: number;
    end?: number | null;
    /** 0..1 framing for object-position cover crop */
    centerX?: number;
    centerY?: number;
    aspect?: "9/16" | "16/9";
    autoplay?: boolean;
    loopSegment?: boolean;
    label?: string;
  }

  let {
    sourcePath = null,
    start = 0,
    end = null,
    centerX = 0.5,
    centerY = 0.45,
    aspect = "9/16",
    autoplay = false,
    loopSegment = true,
    label = "",
    /** larger preview for workstation layouts */
    size = "md" as "sm" | "md" | "lg" | "full",
    /** green phone frame like the operator canvas wireframe */
    framed = false,
    /**
     * cover = crop source into frame (candidates/adjust).
     * contain = show full already-rendered 9:16 deliverable (result) without re-crop.
     */
    fit = "cover" as "cover" | "contain",
  }: Props & {
    size?: "sm" | "md" | "lg" | "full";
    framed?: boolean;
    fit?: "cover" | "contain";
  } = $props();

  const maxW = $derived(
    size === "full"
      ? "min(100%, 420px)"
      : size === "lg"
        ? framed
          ? "min(100%, 260px)"
          : fit === "contain"
            ? "min(100%, 240px)"
            : "280px"
        : size === "sm"
          ? "180px"
          : "220px",
  );
  const maxH = $derived(
    size === "full"
      ? "min(68vh, 560px)"
      : framed || fit === "contain"
        ? "min(42vh, 320px)"
        : "none",
  );

  let videoEl = $state<HTMLVideoElement | null>(null);
  let ready = $state(false);
  let playing = $state(false);
  let error = $state<string | null>(null);
  let current = $state(0);

  const src = $derived(pathToPlayableUrl(sourcePath));
  const endT = $derived(end != null && end > start ? end : null);
  const objectPos = $derived(
    `${(Math.min(0.95, Math.max(0.05, centerX)) * 100).toFixed(1)}% ${(Math.min(0.95, Math.max(0.05, centerY)) * 100).toFixed(1)}%`,
  );

  $effect(() => {
    const v = videoEl;
    const url = src;
    ready = false;
    error = null;
    playing = false;
    if (!v || !url) return;
    v.src = url;
    v.load();
    const onMeta = () => {
      ready = true;
      try {
        v.currentTime = start;
      } catch {
        /* ignore */
      }
      if (autoplay) void v.play().catch(() => {});
    };
    const onErr = () => {
      error = "No se pudo reproducir este video.";
      ready = false;
    };
    const onTime = () => {
      current = v.currentTime;
      if (endT != null && v.currentTime >= endT - 0.05) {
        if (loopSegment) {
          v.currentTime = start;
          if (!v.paused) void v.play().catch(() => {});
        } else {
          v.pause();
          playing = false;
        }
      }
    };
    const onPlay = () => (playing = true);
    const onPause = () => (playing = false);
    v.addEventListener("loadedmetadata", onMeta);
    v.addEventListener("error", onErr);
    v.addEventListener("timeupdate", onTime);
    v.addEventListener("play", onPlay);
    v.addEventListener("pause", onPause);
    return () => {
      v.removeEventListener("loadedmetadata", onMeta);
      v.removeEventListener("error", onErr);
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("play", onPlay);
      v.removeEventListener("pause", onPause);
      v.pause();
      v.removeAttribute("src");
      v.load();
    };
  });

  // Seek when start/end or source changes after ready
  $effect(() => {
    const v = videoEl;
    const s = start;
    if (!v || !ready) return;
    if (Math.abs(v.currentTime - s) > 0.35) {
      try {
        v.currentTime = s;
      } catch {
        /* ignore */
      }
    }
  });

  function toggle() {
    const v = videoEl;
    if (!v || !ready) return;
    if (v.paused) void v.play().catch(() => {});
    else v.pause();
  }

  onDestroy(() => {
    if (videoEl) {
      videoEl.pause();
      videoEl.removeAttribute("src");
    }
  });
</script>

<div
  class="flex flex-col items-center gap-2"
  data-testid="short-preview"
>
  <div
    class="relative w-full overflow-hidden bg-black shadow-xl
      {framed
      ? 'rounded-[1.25rem] border-[3px] border-vigil-500 ring-2 ring-vigil-500/30'
      : 'rounded-2xl border border-surface-700'}"
    style="max-width: {maxW}; aspect-ratio: {aspect === '9/16' ? '9/16' : '16/9'}; max-height: {maxH}; width: 100%"
  >
    {#if !src}
      <div class="flex h-full items-center justify-center p-4 text-center text-[11px] text-surface-500">
        Sin archivo de video para previsualizar.
      </div>
    {:else}
      <video
        bind:this={videoEl}
        class="h-full w-full bg-black"
        style="object-fit: {fit}; object-position: {fit === 'cover' ? objectPos : 'center'}"
        playsinline
        muted={false}
        onclick={toggle}
      ></video>
      {#if error}
        <div class="absolute inset-0 flex items-center justify-center bg-black/80 p-3 text-center text-[11px] text-red-300">
          {error}
        </div>
      {:else if !ready}
        <div class="absolute inset-0 flex items-center justify-center text-[11px] text-surface-400">
          Cargando…
        </div>
      {:else if !playing}
        <button
          type="button"
          class="absolute inset-0 flex items-center justify-center bg-black/25"
          onclick={toggle}
          aria-label="Reproducir"
        >
          <span
            class="flex h-14 w-14 items-center justify-center rounded-full bg-vigil-600/95 text-2xl text-white shadow-xl"
          >▶</span>
        </button>
      {/if}
    {/if}
  </div>
  <div class="flex items-center gap-2">
    <button
      type="button"
      class="rounded-full bg-white/10 px-4 py-1.5 text-xs font-semibold text-white hover:bg-white/20 disabled:opacity-40"
      disabled={!ready || !!error}
      onclick={toggle}
      data-testid="preview-play"
    >
      {playing ? "Pausa" : "Reproducir"}
    </button>
    <span class="font-mono text-[10px] text-surface-500">
      {#if endT == null && videoEl?.duration}
        {formatDuration(current)} / {formatDuration(videoEl.duration)}
      {:else}
        {formatDuration(Math.max(0, current - start))}{#if endT != null}<span class="text-surface-600"> / {formatDuration(endT - start)}</span>{/if}
      {/if}
    </span>
  </div>
  {#if label}
    <p class="text-center text-[10px] text-surface-500">{label}</p>
  {/if}
</div>
