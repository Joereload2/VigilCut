<script lang="ts">
  import type { ProtectedRange, VisualPlacement } from "./types";

  interface Props {
    duration: number;
    currentTime: number;
    placements: VisualPlacement[];
    protectedRanges: ProtectedRange[];
    selectedId?: string | null;
    onSelect: (id: string) => void;
  }

  let {
    duration,
    currentTime,
    placements,
    protectedRanges,
    selectedId = null,
    onSelect,
  }: Props = $props();

  const dur = $derived(Math.max(duration, 0.1));

  function leftPct(t: number) {
    return `${Math.min(100, Math.max(0, (t / dur) * 100))}%`;
  }
  function widthPct(a: number, b: number) {
    return `${Math.min(100, Math.max(0.5, ((b - a) / dur) * 100))}%`;
  }
</script>

<section class="rounded-xl border border-surface-800 bg-surface-950/80 p-2">
  <div class="mb-1 flex justify-between text-[10px] text-surface-500">
    <span>Pista visual (salida)</span>
    <span class="font-mono">{currentTime.toFixed(1)}s / {dur.toFixed(0)}s</span>
  </div>
  <div class="relative h-10 overflow-hidden rounded-lg bg-surface-900 ring-1 ring-surface-800">
    {#each protectedRanges as pr (pr.id)}
      <div
        class="absolute top-0 h-full bg-cut/25"
        style="left:{leftPct(pr.outputStart)};width:{widthPct(pr.outputStart, pr.outputEnd)}"
        title="Protegido: {pr.reason}"
      ></div>
    {/each}
    {#each placements.filter((p) => p.status === "active") as pl (pl.id)}
      <button
        type="button"
        class="absolute top-1 h-8 rounded border text-[9px] font-medium
          {selectedId === pl.id
          ? 'border-sky-300 bg-sky-500/80 text-white'
          : 'border-sky-700/60 bg-sky-800/70 text-sky-100 hover:bg-sky-700/80'}"
        style="left:{leftPct(pl.outputStart)};width:{widthPct(pl.outputStart, pl.outputEnd)}"
        title="{pl.label || pl.assetId} · {pl.mode}"
        onclick={() => onSelect(pl.id)}
      >
        <span class="block truncate px-1">{pl.label || "img"}</span>
      </button>
    {/each}
    <!-- playhead -->
    <div
      class="pointer-events-none absolute top-0 z-10 h-full w-0.5 bg-white/90"
      style="left:{leftPct(currentTime)}"
    ></div>
  </div>
  <div class="mt-1 flex flex-wrap gap-2 text-[9px] text-surface-600">
    <span class="inline-flex items-center gap-1"
      ><span class="h-2 w-2 rounded bg-sky-700"></span> placement</span
    >
    <span class="inline-flex items-center gap-1"
      ><span class="h-2 w-2 rounded bg-cut/40"></span> protegido</span
    >
  </div>
</section>
