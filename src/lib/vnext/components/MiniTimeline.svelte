<script lang="ts">
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";

  let {
    start = 0,
    end = 10,
    mediaDuration = null as number | null,
    editable = false,
    onChange,
  }: {
    start?: number;
    end?: number;
    mediaDuration?: number | null;
    editable?: boolean;
    onChange?: (start: number, end: number) => void;
  } = $props();

  const span = $derived(Math.max(0.1, end - start));
  const total = $derived(mediaDuration && mediaDuration > end ? mediaDuration : Math.max(end * 1.15, end + 5));
  const leftPct = $derived(Math.min(95, (start / total) * 100));
  const widthPct = $derived(Math.min(100 - leftPct, (span / total) * 100));
</script>

<div class="w-full" data-testid="mini-timeline">
  <div class="mb-1 flex justify-between font-mono text-[10px] text-surface-500">
    <span>{formatDuration(start)}</span>
    <span class="text-surface-300">{formatDuration(span)}</span>
    <span>{formatDuration(end)}</span>
  </div>
  <div class="relative h-3 overflow-hidden rounded-full bg-surface-800">
    <div
      class="absolute inset-y-0 rounded-full bg-gradient-to-r from-amber-600/90 to-vigil-500/90"
      style="left: {leftPct}%; width: {widthPct}%"
    ></div>
  </div>
  {#if editable && onChange}
    <div class="mt-2 grid grid-cols-2 gap-2">
      <label class="text-[10px] text-surface-500">
        Inicio
        <input
          class="mt-0.5 w-full accent-vigil-500"
          type="range"
          min="0"
          max={Math.max(end - 0.5, 0.5)}
          step="0.05"
          value={start}
          oninput={(e) => onChange(Number((e.currentTarget as HTMLInputElement).value), end)}
        />
      </label>
      <label class="text-[10px] text-surface-500">
        Final
        <input
          class="mt-0.5 w-full accent-vigil-500"
          type="range"
          min={start + 0.5}
          max={Math.max(start + 120, end)}
          step="0.05"
          value={end}
          oninput={(e) => onChange(start, Number((e.currentTarget as HTMLInputElement).value))}
        />
      </label>
    </div>
  {/if}
</div>
