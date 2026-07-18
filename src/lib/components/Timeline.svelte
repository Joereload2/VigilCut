<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime, type Segment } from "$lib/types";
  import { drawWaveform } from "$lib/utils/waveform";

  let trackEl = $state<HTMLDivElement | null>(null);
  let scrollEl = $state<HTMLDivElement | null>(null);
  let zoom = $state(1);

  const duration = $derived(Math.max(projectStore.duration, 0.001));
  const pxPerSec = $derived(Math.max(4, 14 * zoom));
  const trackWidth = $derived(Math.ceil(duration * pxPerSec));

  function colorFor(seg: Segment): string {
    if (seg.decision === "cut") return "bg-cut/85 hover:bg-cut";
    if (seg.decision === "keep") return "bg-keep/80 hover:bg-keep";
    return "bg-warning/75 hover:bg-warning";
  }

  function onTrackClick(e: MouseEvent) {
    const el = trackEl;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const x = e.clientX - rect.left + (scrollEl?.scrollLeft ?? 0);
    const t = Math.max(0, Math.min(duration, x / pxPerSec));
    projectStore.currentTime = t;
    const hit = projectStore.segments.find((s) => t >= s.start && t < s.end);
    if (hit) projectStore.selectedSegmentId = hit.id;
  }

  function onSegmentClick(seg: Segment, e: MouseEvent) {
    e.stopPropagation();
    projectStore.selectSegment(seg.id);
    // Double-click: same as ActionBar — decide (toggle) and advance
    if (e.detail === 2) {
      projectStore.toggleAndAdvance(seg.id);
    }
  }

  function onWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      zoom = Math.min(6, Math.max(0.3, zoom * (e.deltaY > 0 ? 0.9 : 1.1)));
    }
  }

  // Keep selected segment visible
  $effect(() => {
    const seg = projectStore.selectedSegment;
    const sc = scrollEl;
    if (!seg || !sc) return;
    const left = seg.start * pxPerSec;
    const right = seg.end * pxPerSec;
    const viewL = sc.scrollLeft;
    const viewR = viewL + sc.clientWidth;
    if (left < viewL + 40) sc.scrollLeft = Math.max(0, left - 80);
    else if (right > viewR - 40) sc.scrollLeft = right - sc.clientWidth + 80;
  });

  const rulerMarks = $derived(
    Array.from({ length: Math.ceil(duration / 5) + 1 }, (_, i) => i * 5),
  );
</script>

<div class="panel flex h-44 shrink-0 flex-col overflow-hidden sm:h-48">
  <div class="flex items-center gap-2 border-b border-surface-800 px-3 py-1.5">
    <span class="text-xs font-semibold text-surface-300">Línea de tiempo</span>
    <span class="hidden text-[10px] text-surface-500 sm:inline">
      Clic = elegir · Doble clic = cambiar y seguir
    </span>
    <div class="flex-1"></div>
    <span class="flex items-center gap-2 text-[10px] text-surface-400">
      <span class="inline-flex items-center gap-1"><span class="h-2 w-2 rounded-sm bg-keep"></span> Queda</span>
      <span class="inline-flex items-center gap-1"><span class="h-2 w-2 rounded-sm bg-cut"></span> Cortar</span>
    </span>
    <button class="btn-ghost px-1.5 text-xs" onclick={() => (zoom = Math.max(0.3, zoom * 0.85))}>−</button>
    <button class="btn-ghost px-1.5 text-xs" onclick={() => (zoom = Math.min(6, zoom * 1.15))}>+</button>
  </div>

  <div
    bind:this={scrollEl}
    class="relative min-h-0 flex-1 overflow-x-auto overflow-y-hidden bg-surface-950"
    onwheel={onWheel}
  >
    <div
      class="sticky top-0 z-10 h-5 border-b border-surface-800 bg-surface-900/95"
      style:width="{trackWidth}px"
    >
      {#each rulerMarks as t}
        <div
          class="absolute top-0 h-full border-l border-surface-700 pl-1 text-[9px] leading-5 text-surface-500"
          style:left="{t * pxPerSec}px"
        >
          {formatTime(t)}
        </div>
      {/each}
    </div>

    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      bind:this={trackEl}
      class="relative h-[4.5rem] cursor-pointer"
      style:width="{trackWidth}px"
      onclick={onTrackClick}
    >
      {#each projectStore.segments as seg (seg.id)}
        <button
          type="button"
          class="absolute top-1.5 h-14 overflow-hidden rounded-md border border-black/20 text-left shadow-sm transition {colorFor(
            seg,
          )} {projectStore.selectedSegmentId === seg.id
            ? 'z-10 ring-2 ring-white brightness-110'
            : 'opacity-90'}"
          style:left="{seg.start * pxPerSec}px"
          style:width="{Math.max(4, (seg.end - seg.start) * pxPerSec)}px"
          title="{seg.decision === 'cut' ? 'Cortar' : 'Mantener'} · {formatTime(seg.start)}–{formatTime(
            seg.end,
          )}"
          onclick={(e) => onSegmentClick(seg, e)}
        >
          <div class="truncate px-1.5 pt-1.5 text-[10px] font-bold uppercase text-white">
            {seg.decision === "cut" ? "Cortar" : "Queda"}
          </div>
          <div class="truncate px-1.5 text-[9px] text-white/80">
            {formatTime(seg.end - seg.start)}
          </div>
        </button>
      {/each}

      <div
        class="pointer-events-none absolute top-0 z-20 h-full w-0.5 bg-white shadow-[0_0_8px_rgba(255,255,255,0.9)]"
        style:left="{projectStore.currentTime * pxPerSec}px"
      >
        <div
          class="absolute left-1/2 top-0 h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[8px] border-x-transparent border-t-white"
        ></div>
      </div>
    </div>

    <div class="h-10 border-t border-surface-800 bg-surface-900/40" style:width="{trackWidth}px">
      {#if projectStore.waveform?.peaks?.length}
        <canvas
          class="h-full w-full opacity-70"
          width={trackWidth}
          height={40}
          use:drawWaveform={projectStore.waveform.peaks}
        ></canvas>
      {/if}
    </div>
  </div>
</div>
