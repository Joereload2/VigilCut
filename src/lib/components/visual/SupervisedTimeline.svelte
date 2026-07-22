<script lang="ts">
  /**
   * Proportional output timeline: track always 100% width.
   * Slot left/width = time / duration (e.g. 15s of 60s → 25%).
   * No page-level horizontal scroll — only the full duration fits the container.
   */
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import { drawWaveform } from "$lib/utils/waveform";
  import type { CompositionIssue, ProtectedRange, Seg, VisualPlacement } from "./types";
  import { isFullscreenMode, reviewLabel } from "./types";

  interface Props {
    duration: number;
    placements: VisualPlacement[];
    protectedRanges: ProtectedRange[];
    transcript: Seg[];
    issues?: CompositionIssue[];
    selectedId?: string | null;
    imagePathFor: (p: VisualPlacement) => string | null;
    onSelect: (id: string) => void;
    onMove: (id: string, outputStart: number, outputEnd: number) => void;
    onSnapMove?: (id: string, outputStart: number, outputEnd: number) => void;
  }

  let {
    duration,
    placements,
    protectedRanges,
    transcript,
    issues = [],
    selectedId = null,
    imagePathFor,
    onSelect,
    onMove,
    onSnapMove,
  }: Props = $props();

  let trackRoot = $state<HTMLDivElement | null>(null);
  let trackWidthPx = $state(800);

  const dur = $derived(Math.max(duration, 0.1));
  const playhead = $derived(projectStore.outputClock());
  /** Side margins so blocks don't stick to the edges (percent of content track). */
  const MARGIN = 2.5;
  const SPAN = 100 - MARGIN * 2;

  const playheadPct = $derived(MARGIN + Math.min(SPAN, Math.max(0, (playhead / dur) * SPAN)));

  /** Time → percent inside the padded track (margin left + proportional span). */
  function pct(t: number): number {
    return MARGIN + Math.min(SPAN, Math.max(0, (t / dur) * SPAN));
  }
  function widthPct(start: number, end: number): number {
    const w = ((end - start) / dur) * SPAN;
    return Math.max(0.35, Math.min(SPAN, w));
  }

  const outputKeepBlocks = $derived.by(() => {
    const ranges = projectStore.localKeepRanges();
    if (!ranges.length) return [{ start: 0, end: dur, keep: true }];
    const blocks: { start: number; end: number; keep: boolean }[] = [];
    let o = 0;
    for (const [s, e] of ranges) {
      const len = e - s;
      if (len > 0) {
        blocks.push({ start: o, end: o + len, keep: true });
        o += len;
      }
    }
    return blocks.length ? blocks : [{ start: 0, end: dur, keep: true }];
  });

  const outPhrases = $derived.by(() => {
    return transcript.map((seg) => {
      const srcS = seg.span.start;
      const srcE = seg.span.end;
      const outS =
        projectStore.localKeepRanges().length > 0
          ? projectStore.sourceToEdited(srcS)
          : srcS;
      const outE =
        projectStore.localKeepRanges().length > 0
          ? projectStore.sourceToEdited(srcE)
          : srcE;
      return {
        id: seg.id ?? `${srcS}`,
        text: seg.text,
        start: Math.max(0, outS),
        end: Math.max(outS + 0.05, outE),
      };
    });
  });

  const conflictIds = $derived(
    new Set(
      issues
        .filter((i) => i.severity === "error" || i.severity === "warn")
        .map((i) => i.placementId),
    ),
  );

  /** Adaptive ruler ticks for full duration in 100% width */
  const rulerMarks = $derived.by(() => {
    const d = dur;
    let step = 2;
    if (d > 30) step = 5;
    if (d > 90) step = 10;
    if (d > 180) step = 30;
    if (d > 600) step = 60;
    const marks: number[] = [];
    for (let t = 0; t <= d + 0.001; t += step) marks.push(t);
    if (marks[marks.length - 1] < d - 0.05) marks.push(d);
    return marks;
  });

  function thumbUrl(path?: string | null) {
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      return null;
    }
  }

  function seekOutput(t: number) {
    const clamped = Math.max(0, Math.min(dur, t));
    if (projectStore.localKeepRanges().length > 0) {
      projectStore.currentTime = projectStore.editedToSource(clamped);
      projectStore.previewMode = "edited";
    } else {
      projectStore.currentTime = clamped;
    }
  }

  function timeFromClientX(clientX: number): number {
    const el = trackRoot;
    if (!el) return 0;
    const rect = el.getBoundingClientRect();
    // Label column ~3rem; side margins inside content span
    const labelW = 48;
    const usable = Math.max(1, rect.width - labelW);
    const x = clientX - rect.left - labelW;
    const xInSpan = x - (usable * MARGIN) / 100;
    const spanPx = (usable * SPAN) / 100;
    return Math.max(0, Math.min(dur, (xInSpan / Math.max(1, spanPx)) * dur));
  }

  function pxPerSec(): number {
    const el = trackRoot;
    if (!el) return 10;
    const usable = Math.max(1, el.getBoundingClientRect().width - 48);
    const spanPx = (usable * SPAN) / 100;
    return spanPx / dur;
  }

  function onTrackClick(e: MouseEvent) {
    seekOutput(timeFromClientX(e.clientX));
  }

  let drag = $state<null | {
    id: string;
    kind: "move" | "start" | "end";
    originX: number;
    start0: number;
    end0: number;
  }>(null);

  function startDrag(
    e: PointerEvent,
    id: string,
    kind: "move" | "start" | "end",
    start0: number,
    end0: number,
  ) {
    e.preventDefault();
    e.stopPropagation();
    onSelect(id);
    seekOutput(start0);
    drag = { id, kind, originX: e.clientX, start0, end0 };
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  }

  function applyDrag(e: PointerEvent, commit: boolean) {
    if (!drag) return;
    const dx = (e.clientX - drag.originX) / pxPerSec();
    let s = drag.start0;
    let en = drag.end0;
    if (drag.kind === "move") {
      const len = en - s;
      s = Math.max(0, Math.min(dur - len, drag.start0 + dx));
      en = s + len;
    } else if (drag.kind === "start") {
      s = Math.max(0, Math.min(drag.end0 - 0.25, drag.start0 + dx));
    } else {
      en = Math.min(dur, Math.max(drag.start0 + 0.25, drag.end0 + dx));
    }
    if (commit) {
      const id = drag.id;
      drag = null;
      if (onSnapMove) onSnapMove(id, s, en);
      else onMove(id, s, en);
    } else {
      onMove(drag.id, s, en);
    }
  }

  function selectBlock(pl: VisualPlacement) {
    onSelect(pl.id);
    seekOutput(pl.outputStart);
  }

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

<!-- svelte-ignore a11y_no_static_element_interactions -->
<section
  class="flex h-full min-h-0 min-w-0 w-full max-w-full flex-col overflow-hidden rounded-xl border border-sky-800/50 bg-surface-950"
  style="box-sizing:border-box; width:100%; height:100%"
  aria-label="Línea de tiempo B-roll"
  onpointermove={(e) => applyDrag(e, false)}
  onpointerup={(e) => applyDrag(e, true)}
  onpointercancel={(e) => applyDrag(e, true)}
>
  <div
    class="flex min-w-0 shrink-0 items-center gap-2 overflow-x-hidden border-b border-surface-800 px-2 py-0.5"
  >
    <span class="shrink-0 text-[11px] font-semibold text-surface-200">Timeline</span>
    <div class="min-w-0 flex-1"></div>
    <span class="shrink-0 font-mono text-[10px] text-surface-500">{formatTime(dur)}</span>
  </div>

  <!-- Tracks MUST be flex-col so flex-1 children fill remaining height (no empty void). -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    bind:this={trackRoot}
    class="relative flex min-h-0 min-w-0 w-full max-w-full flex-1 flex-col overflow-hidden"
    style="box-sizing:border-box"
    onclick={onTrackClick}
  >
    <!-- Ruler -->
    <div
      class="relative h-4 w-full border-b border-surface-800 bg-surface-900/95 pl-12"
      style="box-sizing:border-box"
    >
      {#each rulerMarks as t}
        <div
          class="absolute top-0 h-full border-l border-surface-700 pl-0.5 text-[8px] leading-4 text-surface-500"
          style="left: calc(3rem + (100% - 3rem) * {pct(t) / 100})"
        >
          {formatTime(t)}
        </div>
      {/each}
    </div>

    <!-- Compact fixed track heights (timeline band is intentionally small) -->
    <div class="relative h-5 w-full border-b border-surface-800/80 pl-12">
      <div
        class="absolute left-0 top-0 z-10 flex h-full w-12 items-center bg-surface-950/95 px-1 text-[8px] text-surface-500"
      >
        Video
      </div>
      {#each outputKeepBlocks as b, i (i)}
        <div
          class="absolute top-0.5 h-4 rounded bg-keep/55 ring-1 ring-keep/40"
          style="left: calc(3rem + (100% - 3rem) * {pct(b.start) / 100}); width: calc((100% - 3rem) * {widthPct(b.start, b.end) / 100})"
          title="Conservado {formatTime(b.start)}–{formatTime(b.end)}"
        ></div>
      {/each}
      {#each protectedRanges as pr (pr.id)}
        <div
          class="absolute top-0.5 h-4 rounded bg-cut/35 ring-1 ring-cut/50"
          style="left: calc(3rem + (100% - 3rem) * {pct(pr.outputStart) / 100}); width: calc((100% - 3rem) * {widthPct(pr.outputStart, pr.outputEnd) / 100})"
          title="Protegido: {pr.reason}"
        ></div>
      {/each}
    </div>

    <div class="relative h-5 w-full border-b border-surface-800/80 pl-12">
      <div
        class="absolute left-0 top-0 z-10 flex h-full w-12 items-center bg-surface-950/95 px-1 text-[8px] text-surface-500"
      >
        Texto
      </div>
      {#each outPhrases as ph (ph.id)}
        {@const inSel = placements.find(
          (p) =>
            p.id === selectedId && ph.start < p.outputEnd && ph.end > p.outputStart,
        )}
        <button
          type="button"
          class="absolute top-0.5 h-4 overflow-hidden rounded border px-0.5 text-left text-[8px] leading-4
            {inSel
            ? 'border-amber-400 bg-amber-500/30 text-amber-50'
            : 'border-surface-700 bg-surface-800/80 text-surface-300'}"
          style="left: calc(3rem + (100% - 3rem) * {pct(ph.start) / 100}); width: calc((100% - 3rem) * {widthPct(ph.start, ph.end) / 100}); min-width: 3px"
          title={ph.text}
          onclick={(e) => {
            e.stopPropagation();
            seekOutput(ph.start);
          }}
        >
          <span class="block truncate">{ph.text}</span>
        </button>
      {/each}
    </div>

    <div class="relative h-6 w-full border-b border-surface-800/80 pl-12">
      <div
        class="absolute left-0 top-0 z-10 flex h-full w-12 items-center bg-surface-950/95 px-1 text-[8px] text-surface-500"
      >
        B-roll
      </div>
      {#each placements.filter((p) => p.status === "active") as pl (pl.id)}
        {@const u = thumbUrl(imagePathFor(pl))}
        {@const conflict = conflictIds.has(pl.id) || pl.reviewStatus === "conflict"}
        {@const sel = selectedId === pl.id}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="absolute top-0.5 flex h-5 cursor-grab items-stretch overflow-hidden rounded border text-[8px] active:cursor-grabbing
            {sel
            ? 'z-10 border-sky-300 ring-1 ring-sky-400/80'
            : conflict
              ? 'border-amber-500/80 bg-amber-950/70'
              : 'border-sky-700/70 bg-sky-900/80'}"
          style="left: calc(3rem + (100% - 3rem) * {pct(pl.outputStart) / 100}); width: calc((100% - 3rem) * {widthPct(pl.outputStart, pl.outputEnd) / 100}); min-width: 10px"
          title="{pl.label || pl.assetId} · {reviewLabel(pl.reviewStatus)} · {pl.outputStart.toFixed(1)}–{pl.outputEnd.toFixed(1)}s"
          onpointerdown={(e) => startDrag(e, pl.id, "move", pl.outputStart, pl.outputEnd)}
          onclick={(e) => {
            e.stopPropagation();
            selectBlock(pl);
          }}
        >
          <div
            class="w-1 shrink-0 cursor-ew-resize bg-white/25 hover:bg-white/50"
            onpointerdown={(e) => startDrag(e, pl.id, "start", pl.outputStart, pl.outputEnd)}
          ></div>
          {#if u}
            <img src={u} alt="" class="h-full w-4 shrink-0 object-cover" draggable="false" />
          {/if}
          <div class="flex min-w-0 flex-1 items-center truncate px-0.5 text-sky-50">
            {pl.label || "img"}{#if conflict} ⚠{/if}
          </div>
          <div
            class="w-1 shrink-0 cursor-ew-resize bg-white/25 hover:bg-white/50"
            onpointerdown={(e) => startDrag(e, pl.id, "end", pl.outputStart, pl.outputEnd)}
          ></div>
        </div>
      {/each}
    </div>

    <div class="relative h-5 w-full pl-12">
      <div
        class="absolute left-0 top-0 z-10 flex h-full w-12 items-center bg-surface-950/95 px-1 text-[8px] text-surface-500"
      >
        Audio
      </div>
      {#if projectStore.waveform?.peaks?.length}
        <canvas
          class="h-full w-full max-w-full opacity-70"
          width={Math.max(120, Math.floor(trackWidthPx - 48))}
          height={20}
          use:drawWaveform={projectStore.waveform.peaks}
        ></canvas>
      {:else}
        <div class="flex h-full items-center text-[8px] text-surface-600">—</div>
      {/if}
    </div>

    <!-- Playhead -->
    <div
      class="pointer-events-none absolute bottom-0 top-0 z-30 w-0.5 bg-white shadow-[0_0_6px_rgba(255,255,255,0.85)]"
      style="left: calc(3rem + (100% - 3rem) * {playheadPct / 100})"
    >
      <div
        class="absolute left-1/2 top-0 h-0 w-0 -translate-x-1/2 border-x-[4px] border-t-[6px] border-x-transparent border-t-white"
      ></div>
    </div>
  </div>
</section>
