<script lang="ts">
  /**
   * Vista 9:16 = lo que se ve dentro de los marcos verdes.
   * Mismo espacio UV que MultiCropSource (object-cover en stage 16:9).
   * Un video por panel, sincronizados; recarga solo si cambia el archivo.
   */
  import { onDestroy } from "svelte";
  import { pathToPlayableUrl } from "$lib/vnext/media/fileUrl";
  import { playVideo, seekVideo } from "$lib/vnext/media/bindLocalVideo";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";
  import {
    type CropRegion,
    clampSection,
    physicalAspectOfBox,
    regionToPanelStageStyle,
  } from "$lib/vnext/types/crop";

  let {
    sourcePath = null as string | null,
    start = 0,
    end = null as number | null,
    regions = [] as CropRegion[],
    autoplay = true,
  }: {
    sourcePath?: string | null;
    start?: number;
    end?: number | null;
    regions?: CropRegion[];
    autoplay?: boolean;
  } = $props();

  let host = $state<HTMLDivElement | null>(null);
  let playing = $state(false);
  let current = $state(0);
  let ready = $state(false);
  let loadError = $state<string | null>(null);

  let boundUrl: string | null = null;
  const spanLive = { start: 0, end: null as number | null };

  const src = $derived(pathToPlayableUrl(sourcePath));
  const endT = $derived(end != null && end > start ? end : null);
  const panels = $derived(
    (regions.length > 0
      ? regions
      : [{ id: "0", centerX: 0.5, centerY: 0.5, widthFrac: 0.3, heightFrac: 0.95 }]
    )
      .slice(0, 3)
      .map(clampSection),
  );

  function vids(): HTMLVideoElement[] {
    if (!host) return [];
    return Array.from(host.querySelectorAll("video.panel-vid"));
  }

  $effect(() => {
    spanLive.start = start;
    spanLive.end = end != null && end > start ? end : null;
  });

  $effect(() => {
    const url = src;
    // re-run when panel count changes so new <video> nodes get src
    const n = panels.length;
    void n;
    if (!host) return;

    if (!url) {
      ready = false;
      loadError = "Sin video";
      return;
    }

    // Wait a frame so {#each} videos exist in DOM
    let cancelled = false;
    const raf = requestAnimationFrame(() => {
      if (cancelled) return;
      const list = vids();
      if (!list.length) return;

      const needLoad = boundUrl !== url;
      if (needLoad) {
        ready = false;
        loadError = null;
        boundUrl = url;
        for (const v of list) {
          v.src = url;
          v.style.transform = "translateZ(0)";
          v.playsInline = true;
          try {
            v.load();
          } catch {
            /* */
          }
        }
      } else {
        // Ensure every panel has src (new section added)
        for (const v of list) {
          if (!v.src || v.getAttribute("data-url") !== url) {
            v.setAttribute("data-url", url);
            v.src = url;
            try {
              v.load();
            } catch {
              /* */
            }
          }
        }
      }

      const primary = list[0];
      for (const v of list) v.setAttribute("data-url", url);

      const seekAll = (t: number) => {
        for (const v of list) seekVideo(v, t);
      };

      const markReady = () => {
        if (cancelled) return;
        if (primary.readyState >= 1 || primary.videoWidth > 0) {
          ready = true;
          loadError = null;
          seekAll(spanLive.start);
          if (autoplay && primary.paused) {
            void playVideo(primary).catch(() => {});
          }
        }
      };

      const onTime = () => {
        if (cancelled) return;
        current = primary.currentTime;
        if (!ready && (primary.videoWidth > 0 || primary.currentTime > 0)) {
          ready = true;
        }
        const e = spanLive.end;
        if (e != null && primary.currentTime >= e - 0.08) {
          seekAll(spanLive.start);
          if (!primary.paused) void playVideo(primary).catch(() => {});
          return;
        }
        for (let i = 1; i < list.length; i++) {
          const v = list[i];
          if (v && Math.abs(v.currentTime - primary.currentTime) > 0.2) {
            seekVideo(v, primary.currentTime);
          }
        }
      };

      const onPlay = () => {
        playing = true;
        ready = true;
        for (let i = 1; i < list.length; i++) void list[i]?.play().catch(() => {});
      };
      const onPause = () => {
        playing = false;
        for (let i = 1; i < list.length; i++) list[i]?.pause();
      };
      const onErr = () => {
        loadError = "No se pudo cargar el preview 9:16";
      };

      primary.addEventListener("loadedmetadata", markReady);
      primary.addEventListener("loadeddata", markReady);
      primary.addEventListener("canplay", markReady);
      primary.addEventListener("timeupdate", onTime);
      primary.addEventListener("play", onPlay);
      primary.addEventListener("pause", onPause);
      primary.addEventListener("error", onErr);

      const poll = window.setInterval(() => {
        if (primary.readyState >= 1 || primary.videoWidth > 0) {
          markReady();
          window.clearInterval(poll);
        }
      }, 120);

      if (primary.readyState >= 1) markReady();

      // store cleanups on host dataset via closure
      (host as HTMLDivElement & { __cleanup?: () => void }).__cleanup = () => {
        window.clearInterval(poll);
        primary.removeEventListener("loadedmetadata", markReady);
        primary.removeEventListener("loadeddata", markReady);
        primary.removeEventListener("canplay", markReady);
        primary.removeEventListener("timeupdate", onTime);
        primary.removeEventListener("play", onPlay);
        primary.removeEventListener("pause", onPause);
        primary.removeEventListener("error", onErr);
      };
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(raf);
      const h = host as (HTMLDivElement & { __cleanup?: () => void }) | null;
      h?.__cleanup?.();
    };
  });

  $effect(() => {
    const s = start;
    void end;
    const list = vids();
    if (!list[0]) return;
    if (list[0].paused) {
      for (const v of list) seekVideo(v, s);
      current = s;
    }
  });

  async function toggle() {
    const p = vids()[0];
    if (!p) return;
    if (p.paused) {
      seekVideo(p, start);
      try {
        await playVideo(p);
        ready = true;
        playing = true;
        loadError = null;
      } catch (e) {
        loadError = String(e);
      }
    } else {
      p.pause();
      playing = false;
    }
  }

  onDestroy(() => {
    for (const v of vids()) {
      v.pause();
      v.removeAttribute("src");
    }
  });
</script>

<div class="flex h-full min-h-0 w-full flex-col items-center gap-1" data-testid="split-short-preview">
  <div
    bind:this={host}
    class="relative flex min-h-0 w-full max-w-[140px] flex-1 flex-col overflow-hidden rounded-xl border-2 border-vigil-500 bg-black shadow-lg"
    style="aspect-ratio: 9/16; max-height: min(42vh, 280px)"
  >
    {#if !src}
      <div class="flex flex-1 items-center justify-center p-2 text-[10px] text-surface-500">
        Sin video
      </div>
    {:else}
      {#each panels as r, i (r.id)}
        {@const ar = physicalAspectOfBox(r)}
        <div
          class="relative flex w-full items-center justify-center overflow-hidden border-b-2 border-vigil-500/70 bg-black last:border-0"
          style="flex: 1 1 0; min-height: 0; height: {100 / panels.length}%"
        >
          <!--
            Fit box: same physical AR as the green selection, max size inside panel.
            Content is reduced proportionally (letterbox if needed), never stretched.
          -->
          <div
            class="relative overflow-hidden bg-black"
            style="max-width:100%;max-height:100%;aspect-ratio:{ar};width:auto;height:auto;max-width:100%;width:min(100%, calc(100% * {ar}));height:min(100%, calc(100% / {ar}))"
          >
            <div class="absolute inset-0" style={regionToPanelStageStyle(r)}>
              <video
                class="panel-vid h-full w-full object-cover"
                playsinline
                muted={i > 0}
                preload="auto"
                data-url=""
              ></video>
            </div>
          </div>
          <span
            class="pointer-events-none absolute left-1 top-1 z-10 rounded bg-vigil-600 px-1 py-0.5 text-[8px] font-bold text-white"
          >{i + 1}</span>
        </div>
      {/each}

      {#if loadError}
        <div
          class="absolute inset-0 z-20 flex items-center justify-center bg-black/80 p-2 text-center text-[10px] text-cut"
        >
          {loadError}
        </div>
      {:else if !ready}
        <div
          class="pointer-events-none absolute inset-0 z-20 flex items-center justify-center bg-black/25 text-[10px] text-surface-300"
        >
          Preview…
        </div>
      {/if}
    {/if}
  </div>

  <div class="flex shrink-0 items-center gap-1.5">
    <button
      type="button"
      class="rounded-full px-2.5 py-0.5 text-[10px] font-semibold text-white
        {playing ? 'bg-vigil-600' : 'bg-white/10 hover:bg-white/20'}"
      onclick={() => void toggle()}
      data-testid="preview-play"
    >{playing ? "Pausa" : "Play"}</button>
    <span class="font-mono text-[9px] text-surface-500">
      {formatDuration(Math.max(0, current - start))}
      {#if endT}<span class="text-surface-600"> / {formatDuration(endT - start)}</span>{/if}
    </span>
  </div>
</div>
