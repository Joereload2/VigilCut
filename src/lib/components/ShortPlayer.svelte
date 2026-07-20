<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isTauri } from "$lib/utils/tauri";
  import { formatTime, type ClipFraming } from "$lib/types";
  import { projectStore } from "$lib/stores/project.svelte";
  import { clippingUi } from "$lib/stores/clipping.svelte";

  let videoEl = $state<HTMLVideoElement | null>(null);
  let stageEl = $state<HTMLDivElement | null>(null);
  let frameEl = $state<HTMLDivElement | null>(null);
  let ready = $state(false);
  let loadError = $state<string | null>(null);
  let playing = $state(false);

  /**
   * LOCAL focus state — never rely on class-store reactivity for drag UI.
   * Updated imperatively on video + frame DOM nodes every move.
   */
  let focusX = $state(0.5);
  let focusY = $state(0.42);
  let boundClipId = $state<string | null>(null);

  let dragging = $state(false);
  let dragMoved = $state(false);
  let dragMode = $state<"frame" | "pan">("pan");
  let originX = 0;
  let originY = 0;
  let originFx = 0.5;
  let originFy = 0.42;
  let activePointerId: number | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;

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
    } catch {
      try {
        return convertFileSrc(p.replace(/\\/g, "/"));
      } catch {
        return null;
      }
    }
  });

  function clamp(v: number, lo = 0.05, hi = 0.95) {
    return Math.min(hi, Math.max(lo, v));
  }

  /** Paint focus on DOM immediately (works even if Svelte batch is delayed). */
  function paintFocus(x: number, y: number) {
    const px = `${(x * 100).toFixed(2)}%`;
    const py = `${(y * 100).toFixed(2)}%`;
    if (videoEl) {
      videoEl.style.objectFit = "cover";
      videoEl.style.objectPosition = `${px} ${py}`;
    }
    if (frameEl) {
      frameEl.style.left = px;
      frameEl.style.top = py;
    }
  }

  function applyFocus(x: number, y: number, persist: boolean) {
    const nx = clamp(x);
    const ny = clamp(y);
    focusX = nx;
    focusY = ny;
    paintFocus(nx, ny);
    clippingUi.focusX = nx;
    clippingUi.focusY = ny;

    const c = clippingUi.selected;
    if (c) {
      const framing: ClipFraming = {
        ...c.framing,
        mode: "manual",
        centerX: nx,
        centerY: ny,
      };
      clippingUi.selected = { ...c, framing };
    }

    if (!persist) return;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      const cur = clippingUi.selected;
      if (!cur) return;
      const framing: ClipFraming = {
        ...cur.framing,
        mode: "manual",
        centerX: focusX,
        centerY: focusY,
      };
      clippingUi.persistFraming?.(cur.id, framing);
    }, 120);
  }

  // Sync focus when a different clip is selected
  $effect(() => {
    const c = clippingUi.selected;
    const _t = token;
    if (!c) {
      boundClipId = null;
      return;
    }
    if (c.id === boundClipId && dragging) return;
    if (c.id !== boundClipId) {
      boundClipId = c.id;
      const x = clamp(c.framing?.centerX ?? 0.5);
      const y = clamp(c.framing?.centerY ?? 0.42);
      focusX = x;
      focusY = y;
      // paint after DOM binds
      requestAnimationFrame(() => paintFocus(x, y));
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

  // Seek/play only when clip or playToken changes — NOT when focus pans
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
      paintFocus(focusX, focusY);
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

  // Window-level drag listeners — reliable in Tauri WebView2
  function onWinMove(e: PointerEvent) {
    if (!dragging || !stageEl) return;
    if (activePointerId !== null && e.pointerId !== activePointerId) return;
    const r = stageEl.getBoundingClientRect();
    if (r.width < 1 || r.height < 1) return;

    const dx = e.clientX - originX;
    const dy = e.clientY - originY;
    if (Math.abs(dx) + Math.abs(dy) > 2) dragMoved = true;

    // Mouse and green frame move in the SAME direction (no invert).
    // dx>0 (right) → focusX up → frame goes right.
    applyFocus(originFx + dx / r.width, originFy + dy / r.height, false);
  }

  function onWinUp(e: PointerEvent) {
    if (!dragging) return;
    if (activePointerId !== null && e.pointerId !== activePointerId) return;
    const wasMoved = dragMoved;
    const mode = dragMode;
    dragging = false;
    activePointerId = null;
    window.removeEventListener("pointermove", onWinMove);
    window.removeEventListener("pointerup", onWinUp);
    window.removeEventListener("pointercancel", onWinUp);
    clippingUi.dragging = false;

    applyFocus(focusX, focusY, true);

    if (!wasMoved && mode === "pan") {
      void togglePlay();
    } else if (wasMoved) {
      projectStore.statusMessage = `Enfoque ${Math.round(focusX * 100)}% · ${Math.round(focusY * 100)}%`;
    }
  }

  function beginDrag(e: PointerEvent, mode: "frame" | "pan") {
    if (!clip || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();

    dragMode = mode;
    dragging = true;
    dragMoved = false;
    originX = e.clientX;
    originY = e.clientY;
    originFx = focusX;
    originFy = focusY;
    activePointerId = e.pointerId;
    clippingUi.dragging = true;

    window.addEventListener("pointermove", onWinMove);
    window.addEventListener("pointerup", onWinUp);
    window.addEventListener("pointercancel", onWinUp);

    try {
      stageEl?.setPointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
  }

  function onStageDown(e: PointerEvent) {
    const t = e.target as HTMLElement | null;
    if (t?.closest?.("[data-frame]")) {
      beginDrag(e, "frame");
      return;
    }
    if (t?.closest?.("[data-no-pan]")) return;
    beginDrag(e, "pan");
  }

  async function togglePlay() {
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
      } catch (err) {
        loadError = String(err);
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

  function nudge(dx: number, dy: number) {
    applyFocus(focusX + dx, focusY + dy, true);
  }

  function center() {
    applyFocus(0.5, 0.42, true);
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
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        bind:this={stageEl}
        class="relative w-full touch-none select-none overflow-hidden rounded-[1.35rem] border-2 border-amber-500/50 bg-black shadow-2xl
          {dragging ? 'cursor-grabbing' : 'cursor-grab'}"
        style="aspect-ratio: 9 / 16; max-height: min(74vh, 680px);"
        onpointerdown={onStageDown}
      >
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          bind:this={videoEl}
          class="pointer-events-none absolute inset-0 h-full w-full object-cover"
          playsinline
          preload="auto"
        ></video>

        <!-- Dark vignette (no pointer) -->
        <div
          class="pointer-events-none absolute inset-0 z-[5]"
          style="box-shadow: inset 0 0 0 1px rgba(0,0,0,0.2);"
        ></div>

        <!-- GREEN FRAME: movable focus reticle -->
        <div
          bind:this={frameEl}
          data-frame
          class="absolute z-20 box-border cursor-move rounded-xl border-[3px] border-emerald-400 bg-emerald-400/10
            {dragging && dragMode === 'frame' ? 'border-emerald-300 bg-emerald-400/20' : ''}"
          style="left:{(focusX * 100).toFixed(2)}%;top:{(focusY * 100).toFixed(2)}%;width:70%;height:32%;transform:translate(-50%,-50%);touch-action:none;"
          role="slider"
          aria-label="Marco de enfoque"
          aria-valuenow={Math.round(focusX * 100)}
          tabindex="0"
        >
          <div
            class="pointer-events-none absolute -top-6 left-1/2 -translate-x-1/2 whitespace-nowrap rounded-md bg-emerald-400 px-2 py-0.5 text-[10px] font-bold text-black shadow"
          >
            ✥ Arrastra para enfocar
          </div>
          <div class="pointer-events-none absolute inset-0 flex items-center justify-center">
            <div class="relative h-8 w-8">
              <div class="absolute left-1/2 top-0 h-full w-[2px] -translate-x-1/2 bg-emerald-300"></div>
              <div class="absolute left-0 top-1/2 h-[2px] w-full -translate-y-1/2 bg-emerald-300"></div>
              <div
                class="absolute left-1/2 top-1/2 h-3 w-3 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-emerald-300 bg-black/50"
              ></div>
            </div>
          </div>
          <span class="pointer-events-none absolute left-0 top-0 h-4 w-4 border-l-[3px] border-t-[3px] border-emerald-300"></span>
          <span class="pointer-events-none absolute right-0 top-0 h-4 w-4 border-r-[3px] border-t-[3px] border-emerald-300"></span>
          <span class="pointer-events-none absolute bottom-0 left-0 h-4 w-4 border-b-[3px] border-l-[3px] border-emerald-300"></span>
          <span class="pointer-events-none absolute bottom-0 right-0 h-4 w-4 border-b-[3px] border-r-[3px] border-emerald-300"></span>
        </div>

        <div
          class="pointer-events-none absolute left-2 top-2 z-30 rounded-full bg-black/80 px-2.5 py-1 text-[10px] font-bold text-amber-200"
        >
          9:16
        </div>

        <div
          class="pointer-events-none absolute bottom-2 left-2 right-2 z-30 rounded-lg bg-black/75 px-2 py-1.5 text-center"
        >
          <div class="truncate text-[11px] font-semibold text-white">{clip.title}</div>
          <div class="font-mono text-[10px] text-emerald-300/90">
            foco {Math.round(focusX * 100)}% · {Math.round(focusY * 100)}% · {formatTime(clip.start)}–{formatTime(
              clip.end,
            )}
          </div>
        </div>

        {#if !playing && ready && !dragging}
          <div
            class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center"
          >
            <span
              class="flex h-12 w-12 items-center justify-center rounded-full bg-vigil-500/80 text-xl text-white shadow-xl"
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
        <button type="button" class="btn-primary text-xs" onclick={togglePlay}>
          {playing ? "Pausa" : "▶ Ver"}
        </button>
        <button type="button" class="btn-ghost text-xs" onclick={restart}>↺</button>
        <button type="button" class="btn-secondary text-xs px-2" onclick={() => nudge(-0.08, 0)}>←</button>
        <button type="button" class="btn-secondary text-xs px-2" onclick={() => nudge(0.08, 0)}>→</button>
        <button type="button" class="btn-secondary text-xs px-2" onclick={() => nudge(0, -0.08)}>↑</button>
        <button type="button" class="btn-secondary text-xs px-2" onclick={() => nudge(0, 0.08)}>↓</button>
        <button type="button" class="btn-ghost text-xs" onclick={center}>Centrar</button>
      </div>
      <p class="text-center text-[10px] text-surface-400">
        Arrastra el marco verde · o usa las flechas · el vídeo se reencuadra al instante
      </p>
    </div>
  {/if}
</div>
