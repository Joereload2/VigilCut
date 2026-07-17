<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime, type Segment } from "$lib/types";

  function badge(seg: Segment) {
    if (seg.decision === "cut") return "bg-cut/20 text-cut border-cut/40";
    if (seg.decision === "pending") return "bg-warning/20 text-warning border-warning/40";
    return "bg-keep/20 text-keep border-keep/40";
  }
</script>

<div class="panel flex min-h-0 flex-1 flex-col overflow-hidden">
  <div class="flex items-center justify-between border-b border-surface-800 px-3 py-2">
    <span class="label">Segmentos / Segments</span>
    <span class="font-mono text-[10px] text-surface-500">{projectStore.segments.length}</span>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if projectStore.segments.length === 0}
      <p class="p-3 text-center text-xs text-surface-500">
        Sin segmentos. Abre un video y ejecuta la detección.
      </p>
    {:else}
      <ul class="space-y-1">
        {#each projectStore.segments as seg (seg.id)}
          <li
            class="flex w-full items-center gap-2 rounded-lg border border-transparent px-2 py-1.5 text-xs transition hover:bg-surface-800
              {projectStore.selectedSegmentId === seg.id ? 'border-surface-600 bg-surface-800' : ''}"
          >
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-2 text-left"
              onclick={() => {
                projectStore.selectedSegmentId = seg.id;
                projectStore.currentTime = seg.start;
              }}
            >
              <span class="w-14 shrink-0 font-mono text-surface-400">{formatTime(seg.start)}</span>
              <span class="min-w-0 flex-1 truncate capitalize text-surface-200">{seg.kind}</span>
              <span class="font-mono text-surface-500">{formatTime(seg.end - seg.start)}</span>
            </button>
            <button
              type="button"
              class="shrink-0 rounded border px-1.5 py-0.5 text-[10px] font-semibold uppercase {badge(seg)}"
              onclick={() => projectStore.toggleSegment(seg.id)}
            >
              {seg.decision}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
