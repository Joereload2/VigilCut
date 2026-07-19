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

  /** Drag-to-focus state */
  let dragging = $state(false);
  let dragMoved = $state(false);
  let dragOrigin = $state<{ x: number; y: number; cx: number; cy: number } | null>(null);

  const clip = $derived(clippingUi.selected);
  const token = $derived(clippingUi.playToken);

  const src = $derived.by(() => {
    const p = projectStore.mediaPath;
    if (!p || p.startsWith("demo://")) return null;
    if (!isTauri()) {
      if (p.startsWith("blob:") || p.startsWith("http")) return p;
      return null;
    }
    try {
      return convertFileSrc(p);
    } catch (e) {
      console.error("ShortPlayer convertFileSrc", e, p);
      try {
        return convertFileSrc(p.replace(/\\/g, "/"));
      } catch (e2) {
        console.error("ShortPlayer convertFileSrc fallback", e2);
        return null;
      }
    }
  });

  const posX = $derived(((clip?.framing.centerX ?? 0.5) * 100).toFixed(1));
  const posY = $derived(((clip?.framing.centerY ?? 0.42) * 100).toFixed(1));
  const mode = $derived(clip?.framing.mode ?? "auto_center");
  const cover = $derived(mode !== "fit_with_bars");

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
      void v
        .play()
        .then(() => {
          playing = true;
        })
        .catch(() => {
          playing = false;
        });
    };
    if (Math.abs(v.currentTime - start) > 0.08) {
      v.addEventListener("seeked", onSeeked);
      try {
        v.currentTime = start;
      } catch {
        void v.play().catch(() => {});
      }
    } else {
      void v
        .play()
        .then(() => {
          playing = true;
        })
        .catch(() => {
          playing = false;
        });
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
        projectStore.statusMessage = "Fin del short";
      }
    };
    const onLoaded = () => {
      ready = true;
      loadError = null;
    };
    const onErr = () => {
      const code = v.error?.code;
      const map: Record<number, string> = {
        1: "Carga cancelada",
        2: "No se pudo leer el archivo",
        3: "Error al decodificar",
        4: "Formato no compatible. Usa MP4 (H.264 + AAC).",
      };
      loadError = map[code ?? 0] ?? "No se pudo cargar el video en el player 9:16";
      ready = false;
    };
    const onPlay = () => {
      playing = true;
    };
    const onPause = () => {
      playing = false;
    };
    v.addEventListener("timeupdate", onTime);
    v.addEventListener("loadeddata", onLoaded);
    v.addEventListener("canplay", onLoaded);
    v.addEventListener("error", onErr);
    v.addEventListener("play", onPlay);
    v.addEventListener("pause", onPause);
    return () => {
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("loadeddata", onLoaded);
      v.removeEventListener("canplay", onLoaded);
      v.removeEventListener("error", onErr);
      v.removeEventListener("play", onPlay);
      v.removeEventListener("pause", onPause);
    };
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

  function onPointerDown(e: PointerEvent) {
    if (!clip || !cover) return;
    // Only primary button
    if (e.button !== 0) return;
    const stage = stageEl;
    if (!stage) return;
    dragging = true;
    dragMoved = false;
    dragOrigin = {
      x: e.clientX,
      y: e.clientY,
      cx: clip.framing.centerX ?? 0.5,
      cy: clip.framing.centerY ?? 0.42,
    };
    stage.setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging || !dragOrigin || !stageEl) return;
    const rect = stageEl.getBoundingClientRect();
    const dx = e.clientX - dragOrigin.x;
    const dy = e.clientY - dragOrigin.y;
    if (Math.abs(dx) + Math.abs(dy) > 4) dragMoved = true;
    // Drag video under the face zone: invert so drag right shows more left of source
    const sens = 0.9;
    const nx = dragOrigin.cx - (dx / Math.max(rect.width, 1)) * sens;
    const ny = dragOrigin.cy - (dy / Math.max(rect.height, 1)) * sens;
    clippingUi.panFraming(nx, ny);
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    const moved = dragMoved;
    dragOrigin = null;
    try {
      stageEl?.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
    // Tap without drag → play/pause
    if (!moved) {
      void toggle();
    } else {
      projectStore.statusMessage = "Enfoque 9:16 ajustado · arrastra para panear";
    }
  }

  function onFocusHandleDown(e: PointerEvent) {
    e.stopPropagation();
    onPointerDown(e);
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
        A la derecha: <strong class="text-surface-300">Sacar clips</strong>, luego elige uno para
        verlo aquí en vertical.
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
        class="relative w-full touch-none overflow-hidden rounded-[1.35rem] border-2 border-amber-500/50 bg-black shadow-2xl shadow-amber-950/40 ring-1 ring-black
          {cover ? (dragging ? 'cursor-grabbing' : 'cursor-grab') : 'cursor-pointer'}"
        style="aspect-ratio: 9 / 16; max-height: min(74vh, 680px);"
        onpointerdown={onPointerDown}
        onpointermove={onPointerMove}
        onpointerup={onPointerUp}
        onpointercancel={onPointerUp}
        role="presentation"
      >
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          bind:this={videoEl}
          class="pointer-events-none absolute inset-0 h-full w-full
            {cover ? 'object-cover' : 'object-contain bg-black'}"
          style={cover ? `object-position: ${posX}% ${posY}%;` : undefined}
          playsinline
          preload="auto"
        ></video>

        {#if cover}
          <!-- Face focus zone: interactive handle -->
          <div
            class="absolute left-[10%] right-[10%] top-[9%] h-[40%] rounded-xl border-2 border-keep/70 shadow-[0_0_0_1px_rgba(0,0,0,0.35)]"
            style="box-shadow: inset 0 0 0 1px rgba(52,211,153,0.25);"
          >
            <div
              class="absolute -top-2.5 left-1/2 -translate-x-1/2 rounded bg-keep/90 px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-wide text-black"
            >
              Rostro aquí · arrastra
            </div>
            <!-- Crosshair at framing focus mapped into zone-ish center -->
            <button
              type="button"
              class="absolute flex h-10 w-10 -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border-2 border-keep bg-black/35 text-keep shadow-lg"
              style="left: {posX}%; top: {posY}%;"
              aria-label="Arrastrar enfoque"
              onpointerdown={onFocusHandleDown}
            >
              <span class="relative block h-5 w-5">
                <span class="absolute left-1/2 top-0 h-full w-px -translate-x-1/2 bg-keep"></span>
                <span class="absolute left-0 top-1/2 h-px w-full -translate-y-1/2 bg-keep"></span>
                <span class="absolute left-1/2 top-1/2 h-2 w-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-keep"></span>
              </span>
            </button>
          </div>
        {/if}

        <div
          class="pointer-events-none absolute left-2 top-2 rounded-full bg-black/75 px-2.5 py-1 text-[10px] font-bold tracking-wide text-amber-200"
        >
          9:16
        </div>

        <div
          class="pointer-events-none absolute bottom-2 left-2 right-2 rounded-lg bg-black/70 px-2 py-1.5 text-center"
        >
          <div class="truncate text-[11px] font-semibold text-white">{clip.title}</div>
          <div class="font-mono text-[10px] text-surface-300">
            {formatTime(clip.start)}–{formatTime(clip.end)} · {formatTime(clip.duration)} · score
            {Math.round(clip.score)}
          </div>
        </div>

        {#if !playing && ready && !dragging}
          <div
            class="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/20"
          >
            <span
              class="flex h-14 w-14 items-center justify-center rounded-full bg-vigil-500/90 text-2xl text-white shadow-2xl ring-4 ring-white/15"
              >▶</span
            >
          </div>
        {/if}

        {#if !ready && !loadError}
          <div class="absolute inset-0 flex items-center justify-center bg-black/50 text-xs text-surface-300">
            Cargando vídeo…
          </div>
        {/if}
        {#if loadError}
          <div class="absolute inset-x-2 bottom-14 rounded-lg bg-cut/95 p-2 text-[10px] text-white">
            {loadError}
          </div>
        {/if}
      </div>

      <div class="mt-3 flex flex-wrap items-center justify-center gap-2">
        <button type="button" class="btn-primary text-xs" onclick={toggle}>
          {playing ? "Pausa" : "▶ Ver short"}
        </button>
        <button type="button" class="btn-ghost text-xs" onclick={restart}>↺ Inicio</button>
        <button type="button" class="btn-ghost text-xs" onclick={() => clippingUi.resetFraming()}
          >Centrar rostro</button
        >
        <span class="text-[10px] text-surface-500">
          {cover ? "Arrastra para enfocar" : "modo fit"} · {posX}% / {posY}%
        </span>
      </div>
    </div>
  {/if}
</div>
