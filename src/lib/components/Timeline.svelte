<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime, type Segment } from "$lib/types";
  import { drawWaveform } from "$lib/utils/waveform";

  let trackEl = $state<HTMLDivElement | null>(null);
  let zoom = $state(1);

  const duration = $derived(Math.max(projectStore.duration, 0.001));
  const pxPerSec = $derived(Math.max(4, 12 * zoom));
  const trackWidth = $derived(Math.ceil(duration * pxPerSec));

  function colorFor(seg: Segment): string {
    if (seg.decision === "cut") return "bg-cut/70 hover:bg-cut/90";
    if (seg.decision === "pending") return "bg-warning/60 hover:bg-warning/80";
    if (seg.kind === "silence") return "bg-silence/50 hover:bg-silence/70";
    if (seg.kind === "speech") return "bg-speech/70 hover:bg-speech/90";
    return "bg-vigil-600/70 hover:bg-vigil-500";
  }

  function onTrackClick(e: MouseEvent) {
    const el = trackEl;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const x = e.clientX - rect.left + el.scrollLeft;
    const t = Math.max(0, Math.min(duration, x / pxPerSec));
    projectStore.currentTime = t;
  }

  function selectAndToggle(seg: Segment, e: MouseEvent) {
    e.stopPropagation();
    projectStore.selectedSegmentId = seg.id;
    if (e.detail === 2) {
      projectStore.toggleSegment(seg.id);
    } else {
      projectStore.currentTime = seg.start;
    }
  }

  function onWheel(e: WheelEvent) {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      zoom = Math.min(8, Math.max(0.25, zoom * (e.deltaY > 0 ? 0.9 : 1.1)));
    }
  }

  const rulerMarks = $derived(
    Array.from({ length: Math.ceil(duration / 5) + 1 }, (_, i) => i * 5),
  );
</script>

<div class="panel flex h-52 shrink-0 flex-col overflow-hidden">
  <div class="flex items-center gap-3 border-b border-surface-800 px-3 py-2">
    <span class="label">Timeline</span>
    <span class="text-[10px] text-surface-500">
      clic = seek · doble clic = toggle keep/cut · Ctrl+rueda = zoom
    </span>
    <div class="flex-1"></div>
    <button class="btn-ghost text-xs" onclick={() => (zoom = Math.max(0.25, zoom * 0.8))}>−</button>
    <span class="w-10 text-center font-mono text-[10px] text-surface-400">{zoom.toFixed(1)}x</span>
    <button class="btn-ghost text-xs" onclick={() => (zoom = Math.min(8, zoom * 1.25))}>+</button>
    <button class="btn-secondary text-xs" onclick={() => projectStore.keepAllSpeech()}>
      Keep speech
    </button>
    <button class="btn-ghost text-xs" onclick={() => projectStore.keepEverything()}>
      Keep all
    </button>
    <button
      class="btn-ghost text-xs"
      onclick={() => projectStore.splitSelectedAtPlayhead()}
      disabled={!projectStore.selectedSegmentId}
    >
      Split
    </button>
  </div>

  <div
    class="relative min-h-0 flex-1 overflow-x-auto overflow-y-hidden bg-surface-950"
    onwheel={onWheel}
  >
    <div
      class="sticky top-0 z-10 flex h-6 border-b border-surface-800 bg-surface-900/90"
      style:width="{trackWidth}px"
    >
      {#each rulerMarks as t}
        <div
          class="absolute top-0 h-full border-l border-surface-700 pl-1 text-[9px] text-surface-500"
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
      class="relative h-20 cursor-crosshair"
      style:width="{trackWidth}px"
      onclick={onTrackClick}
    >
      {#each projectStore.segments as seg (seg.id)}
        <button
          type="button"
          class="absolute top-2 h-14 overflow-hidden rounded border border-black/30 text-left transition {colorFor(
            seg,
          )} {projectStore.selectedSegmentId === seg.id ? 'ring-2 ring-white z-10' : ''}"
          style:left="{seg.start * pxPerSec}px"
          style:width="{Math.max(2, (seg.end - seg.start) * pxPerSec)}px"
          title="{seg.kind} · {seg.decision} · {formatTime(seg.start)}–{formatTime(seg.end)}"
          onclick={(e) => selectAndToggle(seg, e)}
        >
          <div class="truncate px-1 pt-1 text-[9px] font-medium uppercase text-white/90">
            {seg.decision === "cut" ? "CUT" : seg.kind}
          </div>
          <div class="truncate px-1 font-mono text-[9px] text-white/70">
            {formatTime(seg.end - seg.start)}
          </div>
        </button>
      {/each}

      <div
        class="pointer-events-none absolute top-0 z-20 h-full w-0.5 bg-white shadow-[0_0_6px_rgba(255,255,255,0.8)]"
        style:left="{projectStore.currentTime * pxPerSec}px"
      >
        <div
          class="absolute -top-0 left-1/2 h-0 w-0 -translate-x-1/2 border-x-4 border-t-8 border-x-transparent border-t-white"
        ></div>
      </div>
    </div>

    <div
      class="relative h-12 border-t border-surface-800 bg-surface-900/50"
      style:width="{trackWidth}px"
    >
      {#if projectStore.waveform?.peaks?.length}
        <canvas
          class="h-full w-full opacity-80"
          width={trackWidth}
          height={48}
          use:drawWaveform={projectStore.waveform.peaks}
        ></canvas>
      {:else}
        <div class="flex h-full items-center px-3 text-[10px] text-surface-600">
          Waveform (tras análisis / after analysis)
        </div>
      {/if}
    </div>
  </div>
</div>
