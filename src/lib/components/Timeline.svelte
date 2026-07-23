<script lang="ts">
  /**
   * Silence-mode timeline: always 100% width; segments sized by % of duration.
   */
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime, type Segment } from "$lib/types";
  import { drawWaveform } from "$lib/utils/waveform";

  let trackRoot = $state<HTMLDivElement | null>(null);
  let trackWidthPx = $state(800);

  const duration = $derived(Math.max(projectStore.duration, 0.001));
  /** Side margins so segments don't stick to the edges. */
  const MARGIN = 2.5;
  const SPAN = 100 - MARGIN * 2;

  const playheadPct = $derived(
    MARGIN + Math.min(SPAN, Math.max(0, (projectStore.currentTime / duration) * SPAN)),
  );

  function pct(t: number): number {
    return MARGIN + Math.min(SPAN, Math.max(0, (t / duration) * SPAN));
  }
  function widthPct(start: number, end: number): number {
    return Math.max(0.25, Math.min(SPAN, ((end - start) / duration) * SPAN));
  }

  function colorFor(seg: Segment): string {
    if (seg.decision === "cut") return "bg-cut/85 hover:bg-cut";
    if (seg.decision === "keep") return "bg-keep/80 hover:bg-keep";
    return "bg-warning/75 hover:bg-warning";
  }

  function timeFromClientX(clientX: number): number {
    const el = trackRoot;
    if (!el) return 0;
    const rect = el.getBoundingClientRect();
    const usable = Math.max(1, rect.width);
    const x = clientX - rect.left;
    const xInSpan = x - (usable * MARGIN) / 100;
    const spanPx = (usable * SPAN) / 100;
    return Math.max(0, Math.min(duration, (xInSpan / Math.max(1, spanPx)) * duration));
  }

  function onTrackClick(e: MouseEvent) {
    const t = timeFromClientX(e.clientX);
    projectStore.currentTime = t;
    const hit = projectStore.segments.find((s) => t >= s.start && t < s.end);
    if (hit) projectStore.selectedSegmentId = hit.id;
  }

  function onSegmentClick(seg: Segment, e: MouseEvent) {
    e.stopPropagation();
    projectStore.selectSegment(seg.id);
    if (e.detail === 2) {
      projectStore.toggleAndAdvance(seg.id);
    }
  }

  const rulerMarks = $derived.by(() => {
    const d = duration;
    let step = 5;
    if (d > 120) step = 10;
    if (d > 300) step = 30;
    if (d > 900) step = 60;
    const marks: number[] = [];
    for (let t = 0; t <= d + 0.001; t += step) marks.push(t);
    if (marks[marks.length - 1] < d - 0.05) marks.push(d);
    return marks;
  });

  $effect(() => {
    const el = trackRoot;
    if (!el || typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width;
      if (w && w > 0) trackWidthPx = w;
    });
    ro.observe(el);
    trackWidthPx = el.getBoundingClientRect().width;
    return () => ro.disconnect();
  });
</script>

<div
  class="panel flex h-full min-h-0 min-w-0 w-full max-w-full flex-col overflow-hidden"
  style="box-sizing:border-box"
>
  <div
    class="flex min-w-0 shrink-0 items-center gap-2 overflow-x-hidden border-b border-surface-800 px-2 py-1 sm:px-3"
  >
    <span class="shrink-0 text-xs font-semibold text-surface-300">Línea de tiempo</span>
    <span class="hidden text-[10px] text-surface-500 sm:inline">
      100% ancho · tramos proporcionales · clic = elegir · doble = cambiar
    </span>
    <div class="min-w-0 flex-1"></div>
    <span class="flex shrink-0 items-center gap-2 text-[10px] text-surface-400">
      <span class="inline-flex items-center gap-1"
        ><span class="h-2 w-2 rounded-sm bg-keep"></span> Queda</span
      >
      <span class="inline-flex items-center gap-1"
        ><span class="h-2 w-2 rounded-sm bg-cut"></span> Cortar</span
      >
    </span>
    <span class="shrink-0 font-mono text-[10px] text-surface-500">{formatTime(duration)}</span>
  </div>

  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={trackRoot}
    class="relative flex min-h-0 min-w-0 w-full flex-1 flex-col overflow-hidden bg-surface-950"
    onclick={onTrackClick}
  >
    <div class="relative h-5 w-full border-b border-surface-800 bg-surface-900/95">
      {#each rulerMarks as t}
        <div
          class="absolute top-0 h-full border-l border-surface-700 pl-0.5 text-[9px] leading-5 text-surface-500"
          style="left: {pct(t)}%"
        >
          {formatTime(t)}
        </div>
      {/each}
    </div>

    <div class="relative min-h-0 w-full flex-1 cursor-pointer">
      {#each projectStore.segments as seg (seg.id)}
        <button
          type="button"
          class="absolute top-1.5 h-14 overflow-hidden rounded-md border border-black/20 text-left shadow-sm transition {colorFor(
            seg,
          )} {projectStore.selectedSegmentId === seg.id
            ? 'z-10 ring-2 ring-white brightness-110'
            : 'opacity-90'}"
          style="left: {pct(seg.start)}%; width: {widthPct(seg.start, seg.end)}%; min-width: 3px"
          title="{seg.decision === 'cut' ? 'Cortar' : 'Mantener'} · {formatTime(seg.start)}–{formatTime(
            seg.end,
          )} ({widthPct(seg.start, seg.end).toFixed(1)}%)"
          onclick={(e) => onSegmentClick(seg, e)}
        >
          <div class="truncate px-1 pt-1.5 text-[10px] font-bold uppercase text-white">
            {seg.decision === "cut" ? "Cortar" : "Queda"}
          </div>
          <div class="truncate px-1 text-[9px] text-white/80">
            {formatTime(seg.end - seg.start)}
          </div>
        </button>
      {/each}

      <div
        class="pointer-events-none absolute top-0 z-20 h-full w-0.5 bg-white shadow-[0_0_8px_rgba(255,255,255,0.9)]"
        style="left: {playheadPct}%"
      >
        <div
          class="absolute left-1/2 top-0 h-0 w-0 -translate-x-1/2 border-x-[5px] border-t-[8px] border-x-transparent border-t-white"
        ></div>
      </div>
    </div>

    <div class="h-8 w-full shrink-0 border-t border-surface-800 bg-surface-900/40">
      {#if projectStore.waveform?.peaks?.length}
        <canvas
          class="h-full w-full max-w-full opacity-70"
          width={Math.max(120, Math.floor(trackWidthPx))}
          height={32}
          use:drawWaveform={projectStore.waveform.peaks}
        ></canvas>
      {/if}
    </div>
  </div>
</div>
