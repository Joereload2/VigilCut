<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isTauri } from "$lib/utils/tauri";
  import { formatTime } from "$lib/types";
  import { projectStore } from "$lib/stores/project.svelte";
  import { clippingUi } from "$lib/stores/clipping.svelte";

  let videoEl = $state<HTMLVideoElement | null>(null);
  let stageEl = $state<HTMLDivElement | null>(null);
  let ready = $state(false);
  let loadError = $state<string | null>(null);
  let playing = $state(false);

  /** Local drag session (not only store — survives any re-render) */
  let dragKind = $state<"none" | "frame" | "pan">("none");
  let dragMoved = $state(false);
  let dragStart = $state<{
    px: number;
    py: number;
    fx: number;
    fy: number;
  } | null>(null);

  const clip = $derived(clippingUi.selected);
  const token = $derived(clippingUi.playToken);
  // Live focus — always reactive, drives frame + video
  const focusX = $derived(clippingUi.focusX);
  const focusY = $derived(clippingUi.focusY);
  const posX = $derived((focusX * 100).toFixed(1));
  const posY = $derived((focusY * 100).toFixed(1));

  const src = $derived.by(() => {
    const p = projectStore.mediaPath;
    if (!p || p.startsWith("demo://")) return null;
    if (!isTauri()) {
      if (p.startsWith("blob:") || p.startsWith("http")) return p;
      return null;
    }
    try {
      return convertFileSrc(p);
    } catch {
      try {
        return convertFileSrc(p.replace(/\\/g, "/"));
      } catch {
        return null;
      }
    }
  });

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

  $effect(() => {
    const v = videoEl;
    const c = clip;
    const _t = token;
    if (!v || !c || !ready) return;
    const start = Math.max(0, c.start);
    const onSeeked = () => {
      v.removeEventListener("seeked", onSeeked);
      void v.play().then(() => (playing = true)).catch(() => (playing = false));
    };
    if (Math.abs(v.currentTime - start) > 0.08) {
      v.addEventListener("seeked", onSeeked);
      try {
        v.currentTime = start;
      } catch {
        void v.play().catch(() => {});
      }
    } else {
      void v.play().then(() => (playing = true)).catch(() => (playing = false));
    }
  });

  $effect(() => {
    const v = videoEl;
    if (!v) return;
    const onTime = () => {
      const c = clippingUi.selected;
      if (!c) return;
      projectStore.currentTime = v.currentTime;
      if (v.currentTime >= c.end - 0.05) {
        v.pause();
        playing = false;
        try {
          v.currentTime = c.end;
        } catch {
          /* ignore */
        }
      }
    };
    const onLoaded = () => {
      ready = true;
      loadError = null;
    };
    const onErr = () => {
      loadError = "No se pudo cargar el video en el player 9:16";
      ready = false;
    };
    v.addEventListener("timeupdate", onTime);
    v.addEventListener("loadeddata", onLoaded);
    v.addEventListener("canplay", onLoaded);
    v.addEventListener("error", onErr);
    v.addEventListener("play", () => (playing = true));
    v.addEventListener("pause", () => (playing = false));
    return () => {
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("loadeddata", onLoaded);
      v.removeEventListener("canplay", onLoaded);
      v.removeEventListener("error", onErr);
    };
  });

  // Apply object-position imperatively so it always sticks even if style binding lags
  $effect(() => {
    const v = videoEl;
    if (!v) return;
    const x = `${(clippingUi.focusX * 100).toFixed(2)}%`;
    const y = `${(clippingUi.focusY * 100).toFixed(2)}%`;
    v.style.objectFit = "cover";
    v.style.objectPosition = `${x} ${y}`;
  });

  async function toggle() {
    const v = videoEl;
    const c = clip;
    if (!v || !c) return;
    if (v.paused) {
      if (v.currentTime < c.start - 0.05 || v.currentTime >= c.end - 0.05) {
        try {
          v.currentTime = c.start;
        } catch {
          /* ignore */
        }
      }
      try {
        await v.play();
      } catch (e) {
        loadError = String(e);
      }
    } else {
      v.pause();
    }
  }

  function restart() {
    const v = videoEl;
    const c = clip;
    if (!v || !c) return;
    try {
      v.currentTime = c.start;
    } catch {
      /* ignore */
    }
    void v.play().catch(() => {});
  }

  function clientToFocus(clientX: number, clientY: number): { x: number; y: number } | null {
    const stage = stageEl;
    if (!stage) return null;
    const r = stage.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return null;
    return {
      x: (clientX - r.left) / r.width,
      y: (clientY - r.top) / r.height,
    };
  }

  function startFrameDrag(e: PointerEvent) {
    if (!clip || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    dragKind = "frame";
    dragMoved = false;
    dragStart = {
      px: e.clientX,
      py: e.clientY,
      fx: clippingUi.focusX,
      fy: clippingUi.focusY,
    };
    clippingUi.beginDrag();
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  }

  function startPanDrag(e: PointerEvent) {
    if (!clip || e.button !== 0) return;
    // Ignore if started on controls
    const t = e.target as HTMLElement;
    if (t.closest("[data-no-pan]")) return;
    e.preventDefault();
    dragKind = "pan";
    dragMoved = false;
    dragStart = {
      px: e.clientX,
      py: e.clientY,
      fx: clippingUi.focusX,
      fy: clippingUi.focusY,
    };
    clippingUi.beginDrag();
    stageEl?.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (dragKind === "none" || !dragStart || !stageEl) return;
    const r = stageEl.getBoundingClientRect();
    const dx = e.clientX - dragStart.px;
    const dy = e.clientY - dragStart.py;
    if (Math.abs(dx) + Math.abs(dy) > 3) dragMoved = true;

    if (dragKind === "frame") {
      // Frame follows pointer: focus = position under finger
      const p = clientToFocus(e.clientX, e.clientY);
      if (p) clippingUi.setFocus(p.x, p.y);
    } else {
      // Pan video under fixed zone: invert delta
      const nx = dragStart.fx - dx / r.width;
      const ny = dragStart.fy - dy / r.height;
      clippingUi.setFocus(nx, ny);
    }
  }

  function onPointerUp(e: PointerEvent) {
    if (dragKind === "none") return;
    const kind = dragKind;
    const moved = dragMoved;
    dragKind = "none";
    dragStart = null;
    clippingUi.endDrag();
    try {
      stageEl?.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
    if (!moved && kind === "pan") {
      void toggle();
    } else if (moved) {
      projectStore.statusMessage = `Enfoque ${Math.round(clippingUi.focusX * 100)}% / ${Math.round(clippingUi.focusY * 100)}%`;
    }
  }
</script>

<div class="flex h-full min-h-0 flex-col items-center justify-center gap-3 p-3">
  {#if !clip}
    <div
      class="flex aspect-[9/16] w-full max-w-[min(100%,340px)] max-h-full flex-col items-center justify-center rounded-2xl border border-dashed border-surface-700 bg-surface-900/80 px-4 text-center"
    >
      <div class="mb-2 text-3xl opacity-60">📱</div>
      <p class="text-sm font-medium text-surface-300">Preview 9:16</p>
      <p class="mt-2 text-[11px] leading-relaxed text-surface-500">
        Saca clips a la derecha y selecciona uno para enfocar el 9:16.
      </p>
    </div>
  {:else if !src}
    <div class="max-w-xs text-center text-sm text-cut">
      No hay URL de video. Abre un archivo real en la app de escritorio.
    </div>
  {:else}
    <div class="relative flex min-h-0 w-full max-w-[min(100%,380px)] flex-1 flex-col items-center justify-center">
      <div
        bind:this={stageEl}
        class="relative w-full touch-none select-none overflow-hidden rounded-[1.35rem] border-2 border-amber-500/50 bg-black shadow-2xl shadow-amber-950/40 ring-1 ring-black
          {dragKind !== 'none' ? 'cursor-grabbing' : 'cursor-grab'}"
        style="aspect-ratio: 9 / 16; max-height: min(74vh, 680px);"
        onpointerdown={startPanDrag}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        role="presentation"
      >
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          bind:this={videoEl}
          class="pointer-events-none absolute inset-0 h-full w-full object-cover"
          playsinline
          preload="auto"
        ></video>

        <!-- Movable green focus frame — center = crop focus point -->
        <div
          class="absolute z-20 cursor-move rounded-xl border-[2.5px] border-keep bg-keep/5 shadow-[0_0_0_9999px_rgba(0,0,0,0.28)] active:bg-keep/10"
          style="left:{posX}%;top:{posY}%;width:72%;height:34%;transform:translate(-50%,-50%);"
          data-no-pan
          onpointerdown={startFrameDrag}
          role="slider"
          aria-label="Marco de enfoque 9:16"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(focusX * 100)}
          tabindex="0"
        >
          <div
            class="pointer-events-none absolute -top-5 left-1/2 -translate-x-1/2 whitespace-nowrap rounded bg-keep px-2 py-0.5 text-[9px] font-bold uppercase tracking-wide text-black"
          >
            Arrastra el marco
          </div>
          <!-- Center crosshair -->
          <div class="pointer-events-none absolute inset-0 flex items-center justify-center">
            <span class="relative block h-6 w-6">
              <span class="absolute left-1/2 top-0 h-full w-0.5 -translate-x-1/2 bg-keep"></span>
              <span class="absolute left-0 top-1/2 h-0.5 w-full -translate-y-1/2 bg-keep"></span>
              <span
                class="absolute left-1/2 top-1/2 h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-keep bg-black/40"
              ></span>
            </span>
          </div>
          <!-- Corner grips -->
          <span class="absolute -left-1 -top-1 h-3 w-3 rounded-sm border-l-2 border-t-2 border-keep"></span>
          <span class="absolute -right-1 -top-1 h-3 w-3 rounded-sm border-r-2 border-t-2 border-keep"></span>
          <span class="absolute -bottom-1 -left-1 h-3 w-3 rounded-sm border-b-2 border-l-2 border-keep"></span>
          <span class="absolute -bottom-1 -right-1 h-3 w-3 rounded-sm border-b-2 border-r-2 border-keep"></span>
        </div>

        <div
          class="pointer-events-none absolute left-2 top-2 z-30 rounded-full bg-black/75 px-2.5 py-1 text-[10px] font-bold tracking-wide text-amber-200"
        >
          9:16
        </div>

        <div
          class="pointer-events-none absolute bottom-2 left-2 right-2 z-30 rounded-lg bg-black/70 px-2 py-1.5 text-center"
        >
          <div class="truncate text-[11px] font-semibold text-white">{clip.title}</div>
          <div class="font-mono text-[10px] text-surface-300">
            {formatTime(clip.start)}–{formatTime(clip.end)} · score {Math.round(clip.score)} · foco
            {Math.round(focusX * 100)}/{Math.round(focusY * 100)}
          </div>
        </div>

        {#if !playing && ready && dragKind === "none"}
          <div
            class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center bg-black/15"
          >
            <span
              class="flex h-12 w-12 items-center justify-center rounded-full bg-vigil-500/85 text-xl text-white shadow-xl"
              >▶</span
            >
          </div>
        {/if}

        {#if !ready && !loadError}
          <div class="absolute inset-0 z-40 flex items-center justify-center bg-black/50 text-xs text-surface-300">
            Cargando…
          </div>
        {/if}
        {#if loadError}
          <div class="absolute inset-x-2 bottom-14 z-40 rounded-lg bg-cut/95 p-2 text-[10px] text-white">
            {loadError}
          </div>
        {/if}
      </div>

      <div class="mt-2 flex flex-wrap items-center justify-center gap-1.5" data-no-pan>
        <button type="button" class="btn-primary text-xs" onclick={toggle}>
          {playing ? "Pausa" : "▶ Ver"}
        </button>
        <button type="button" class="btn-ghost text-xs" onclick={restart}>↺</button>
        <button
          type="button"
          class="btn-ghost text-xs"
          onclick={() => clippingUi.nudge(-0.06, 0)}
          title="Mover foco izquierda">←</button
        >
        <button
          type="button"
          class="btn-ghost text-xs"
          onclick={() => clippingUi.nudge(0.06, 0)}
          title="Mover foco derecha">→</button
        >
        <button
          type="button"
          class="btn-ghost text-xs"
          onclick={() => clippingUi.nudge(0, -0.06)}
          title="Mover foco arriba">↑</button
        >
        <button
          type="button"
          class="btn-ghost text-xs"
          onclick={() => clippingUi.nudge(0, 0.06)}
          title="Mover foco abajo">↓</button
        >
        <button type="button" class="btn-ghost text-xs" onclick={() => clippingUi.resetFraming()}
          >Centrar</button
        >
      </div>
      <p class="text-center text-[10px] text-surface-500">
        Arrastra el <span class="text-keep">marco verde</span> o el vídeo · flechas afinan
      </p>
    </div>
  {/if}
</div>
