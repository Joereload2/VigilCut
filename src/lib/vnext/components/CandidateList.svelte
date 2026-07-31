<script lang="ts">
  import type { ClipCandidate } from "$lib/types";
  import { formatDuration, scoreToHuman } from "$lib/vnext/presentation/jobLabels";

  interface Props {
    items: ClipCandidate[];
    selectedId: string | null;
    onSelect: (id: string) => void;
  }

  let { items, selectedId, onSelect }: Props = $props();
</script>

<ul class="min-h-0 flex-1 overflow-y-auto p-2" data-testid="candidate-list">
  {#each items as c (c.id)}
    <li class="mb-1">
      <button
        type="button"
        class="w-full rounded-xl border px-2 py-2 text-left text-[11px] transition
          {selectedId === c.id ? 'border-vigil-500 bg-vigil-950/40' : 'border-transparent bg-surface-900/40 hover:border-surface-700'}"
        onclick={() => onSelect(c.id)}
        data-testid="candidate-item"
      >
        <div class="flex items-center justify-between gap-1">
          <span class="font-semibold text-surface-100">{c.title || "Fragmento"}</span>
          <span class="text-surface-500">{formatDuration(c.duration)}</span>
        </div>
        <div class="mt-0.5 text-surface-400">{scoreToHuman(c.score)}</div>
        <p class="mt-1 line-clamp-2 text-surface-500">{c.transcript || "Sin texto"}</p>
        {#if c.reasons?.[0]?.label}
          <p class="mt-1 text-[10px] text-vigil-300/90">{c.reasons[0].label}</p>
        {/if}
        <p class="mt-1 text-[10px] text-surface-600">
          {formatDuration(c.start)} – {formatDuration(c.end)}
          {#if c.status === "rejected"}
            · Descartado
          {:else if c.status === "approved" || c.status === "modified"}
            · Seleccionado
          {/if}
        </p>
      </button>
    </li>
  {/each}
</ul>
