<script lang="ts">
  /**
   * Source video + green frames that define what each 9:16 split section shows.
   * Max 3 sections. Each frame is draggable and moldable (resize).
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
  import {
    type CropRegion,
    MAX_SECTIONS,
    clampSection,
    defaultSection,
    sectionBox,
  } from "$lib/vnext/types/crop";

  let {
    sourcePath = null as string | null,
    start = 0,
    end = null as number | null,
    regions = [] as CropRegion[],
    activeId = null as string | null,
    onRegionsChange,
    onActiveChange,
  }: {
    sourcePath?: string | null;
    start?: number;
    end?: number | null;
    regions?: CropRegion[];
    activeId?: string | null;
    onRegionsChange?: (r: CropRegion[]) => void;
    onActiveChange?: (id: string) => void;
  } = $props();

  let videoEl = $state<HTMLVideoElement | null>(null);
  let wrapEl = $state<HTMLDivElement | null>(null);
  let ready = $state(false);
  let playing = $state(false);
  let error = $state<string | null>(null);
  let current = $state(0);
  // NO $state — evita bucle effect al asignar
  let boundUrl: string | null = null;
  let handle: LocalVideoHandle | null = null;
  const spanLive = { start: 0, end: null as number | null };
  let drag: null | {
    id: string;
    mode: "move" | "resize";
    originX: number;
    originY: number;
    base: CropRegion;
  } = $state(null);

  const src = $derived(pathToPlayableUrl(sourcePath));
  const endT = $derived(end != null && end > start ? end : null);
  const list = $derived(
    (regions.length > 0 ? regions : [defaultSection(0)]).map((r) => clampSection(r)),
  );
  const active = $derived(list.find((r) => r.id === activeId) ?? list[0]);

  $effect(() => {
    spanLive.start = start;
    spanLive.end = end != null && end > start ? end : null;
  });

  // Carga del archivo: SOLO depende de la URL (no de start ni de marcos)
  $effect(() => {
    const v = videoEl;
    const url = src;
    if (!v) return;
    if (!url) {
      ready = false;
      error = "No se puede leer la ruta del video.";
      handle?.destroy();
      handle = null;
      boundUrl = null;
      return;
    }
    if (boundUrl === url && handle) return;

    handle?.destroy();
    handle = null;
    ready = false;
    playing = false;
    error = null;
    boundUrl = url;

    handle = bindLocalVideo(v, url, {
      onReady: () => {
        ready = true;
        error = null;
        seekVideo(v, spanLive.start);
      },
      onError: (msg) => {
        ready = false;
        error = msg;
      },
      onTime: () => {
        current = v.currentTime;
        const e = spanLive.end;
        if (e != null && v.currentTime >= e - 0.08) {
          seekVideo(v, spanLive.start);
          if (!v.paused) void playVideo(v).catch(() => {});
        }
      },
      onPlay: () => {
        playing = true;
        ready = true;
      },
      onPause: () => {
        playing = false;
      },
    });

    return () => {
      handle?.destroy();
      handle = null;
    };
  });

  // Seek al cambiar el tramo temporal (sin recargar)
  $effect(() => {
    const v = videoEl;
    const s = start;
    if (!v || !ready) return;
    if (v.paused && Math.abs(v.currentTime - s) > 0.3) {
      seekVideo(v, s);
      current = s;
    }
  });

  function setList(next: CropRegion[]) {
    onRegionsChange?.(next.map(clampSection).slice(0, MAX_SECTIONS));
  }

  async function toggle() {
    const v = videoEl;
    if (!v) return;
    if (v.paused) {
      seekVideo(v, start);
      try {
        await playVideo(v);
        ready = true;
        error = null;
        playing = true;
      } catch (e) {
        error = String(e);
      }
    } else {
      v.pause();
      playing = false;
    }
  }

  function pointerDown(e: PointerEvent, id: string, mode: "move" | "resize") {
    e.preventDefault();
    e.stopPropagation();
    const base = list.find((r) => r.id === id);
    if (!base) return;
    onActiveChange?.(id);
    drag = { id, mode, originX: e.clientX, originY: e.clientY, base: { ...base } };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function pointerMove(e: PointerEvent) {
    if (!drag || !wrapEl) return;
    const rect = wrapEl.getBoundingClientRect();
    if (rect.width < 8 || rect.height < 8) return;
    const dx = (e.clientX - drag.originX) / rect.width;
    const dy = (e.clientY - drag.originY) / rect.height;
    const b = drag.base;

    if (drag.mode === "move") {
      setList(
        list.map((r) =>
          r.id === drag!.id
            ? clampSection({ ...r, centerX: b.centerX + dx, centerY: b.centerY + dy })
            : r,
        ),
      );
    } else {
      // Free resize — any size; preview/export fit proportionally into 9:16
      setList(
        list.map((r) =>
          r.id === drag!.id
            ? clampSection({
                ...r,
                widthFrac: b.widthFrac + dx,
                heightFrac: b.heightFrac + dy,
              })
            : r,
        ),
      );
    }
  }

  function pointerUp(e: PointerEvent) {
    if (!drag) return;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      /* */
    }
    drag = null;
  }

  function addSection() {
    if (list.length >= MAX_SECTIONS) return;
    const r = defaultSection(list.length);
    setList([...list, r]);
    onActiveChange?.(r.id);
  }

  function removeActive() {
    if (list.length <= 1) return;
    const id = active?.id ?? list[list.length - 1].id;
    const next = list.filter((r) => r.id !== id);
    setList(next);
    onActiveChange?.(next[0]?.id);
  }

  onDestroy(() => {
    handle?.destroy();
    videoEl?.pause();
    videoEl?.removeAttribute("src");
  });
</script>

<div class="flex h-full min-h-0 w-full flex-col gap-1" data-testid="multi-crop-source">
  <div
    bind:this={wrapEl}
    class="relative min-h-0 w-full flex-1 select-none overflow-hidden rounded-lg border border-surface-700 bg-black"
    style="max-height: 100%"
    role="presentation"
    onpointermove={pointerMove}
    onpointerup={pointerUp}
    onpointercancel={pointerUp}
  >
    {#if !src}
      <div class="flex h-full items-center justify-center text-xs text-surface-500">
        Sin video fuente
      </div>
    {:else}
      <video
        bind:this={videoEl}
        class="pointer-events-none h-full w-full bg-black object-cover"
        playsinline
        preload="auto"
      ></video>

      {#if error}
        <div
          class="absolute inset-0 z-30 flex items-center justify-center bg-black/80 p-3 text-center text-xs text-red-300"
        >
          {error}
        </div>
      {:else if !ready}
        <div
          class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-black/25 text-xs text-surface-300"
        >
          Cargando… (Play abajo)
        </div>
      {/if}

      <!-- Marcos siempre visibles (aunque cargue) para no bloquear el trabajo -->
      {#if active}
        {@const g = sectionBox(active)}
        <div class="pointer-events-none absolute inset-y-0 left-0 z-[5] bg-black/55" style="width:{g.left}%"></div>
        <div
          class="pointer-events-none absolute inset-y-0 z-[5] bg-black/55"
          style="left:{g.left + g.w}%;width:{Math.max(0, 100 - g.left - g.w)}%"
        ></div>
        <div class="pointer-events-none absolute left-0 right-0 top-0 z-[5] bg-black/40" style="height:{g.top}%"></div>
        <div
          class="pointer-events-none absolute left-0 right-0 z-[5] bg-black/40"
          style="top:{g.top + g.h}%;height:{Math.max(0, 100 - g.top - g.h)}%"
        ></div>
      {/if}

      {#each list as r, i (r.id)}
        {@const g = sectionBox(r)}
        {@const isActive = active?.id === r.id}
        <div
          class="absolute z-20 box-border cursor-move rounded-lg border-[3px] border-vigil-500 shadow-[0_0_0_1px_rgba(0,0,0,0.55)]
            {isActive ? 'ring-2 ring-vigil-400/50' : 'opacity-85'}"
          style="left:{g.left}%;top:{g.top}%;width:{g.w}%;height:{g.h}%"
          role="button"
          tabindex="0"
          aria-label="Sección {i + 1} del Short 9:16"
          onpointerdown={(e) => pointerDown(e, r.id, "move")}
          data-testid="crop-box"
        >
          <span
            class="absolute left-1 top-1 rounded bg-vigil-600 px-1.5 py-0.5 text-[9px] font-bold text-white"
          >
            Sección {i + 1}
          </span>
          <span
            class="absolute bottom-1 left-1 max-w-[90%] truncate rounded bg-black/65 px-1 py-0.5 text-[8px] text-surface-200"
          >
            → panel {i + 1} del 9:16
          </span>
          <button
            type="button"
            class="absolute bottom-0 right-0 z-30 h-4 w-4 translate-x-1/3 translate-y-1/3 cursor-se-resize rounded-sm border border-white bg-vigil-500 shadow"
            title="Redimensionar sección"
            aria-label="Redimensionar sección {i + 1}"
            onpointerdown={(e) => pointerDown(e, r.id, "resize")}
          ></button>
        </div>
      {/each}
    {/if}
  </div>

  <div class="flex shrink-0 flex-wrap items-center gap-1.5">
    <button
      type="button"
      class="rounded bg-white/10 px-2.5 py-0.5 text-[11px] font-semibold text-white hover:bg-white/20 disabled:opacity-40"
      disabled={!src}
      onclick={() => void toggle()}
      data-testid="source-play"
    >{playing ? "Pausa" : "Play"}</button>
    <span class="font-mono text-[9px] text-surface-500">
      {formatDuration(Math.max(0, current - start))}
      {#if endT}<span class="text-surface-600"> / {formatDuration(endT - start)}</span>{/if}
    </span>
    <button
      type="button"
      class="rounded border border-vigil-600/60 bg-vigil-950/50 px-2 py-0.5 text-[9px] font-semibold text-vigil-200 disabled:opacity-40"
      disabled={list.length >= MAX_SECTIONS}
      onclick={addSection}
      data-testid="add-crop"
    >+ Sección ({list.length}/{MAX_SECTIONS})</button>
    <button
      type="button"
      class="rounded border border-surface-700 px-1.5 py-0.5 text-[9px] text-surface-400 disabled:opacity-40"
      disabled={list.length <= 1}
      onclick={removeActive}
    >Quitar</button>
    <span class="text-[9px] text-surface-600">marcos libres · 9:16 reduce al caber</span>
  </div>
</div>
