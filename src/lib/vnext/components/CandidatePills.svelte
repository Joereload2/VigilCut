<script lang="ts">
  import type { ClipCandidate } from "$lib/types";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";

  let {
    items,
    selectedId,
    onSelect,
  }: {
    items: ClipCandidate[];
    selectedId: string | null;
    onSelect: (id: string) => void;
  } = $props();

  const shown = $derived(items.slice(0, 8));
</script>

<div class="flex flex-wrap gap-1.5" data-testid="candidate-pills">
  {#each shown as c, i (c.id)}
    <button
      type="button"
      class="min-w-[3.25rem] rounded-lg px-2.5 py-1.5 text-[10px] font-semibold transition
        {selectedId === c.id
        ? 'bg-vigil-600 text-white shadow'
        : 'bg-vigil-900/50 text-vigil-200 ring-1 ring-vigil-700/50 hover:bg-vigil-800/60'}"
      onclick={() => onSelect(c.id)}
      title={c.title || `Clip ${i + 1}`}
    >
      {i + 1}
      <span class="ml-1 font-mono font-normal opacity-80">{formatDuration(c.duration)}</span>
    </button>
  {/each}
</div>
