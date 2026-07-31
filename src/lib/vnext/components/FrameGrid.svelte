<script lang="ts">
  import type { ClipCandidate } from "$lib/types";
  import { formatDuration, scoreToHuman } from "$lib/vnext/presentation/jobLabels";
  import { pathToPlayableUrl } from "$lib/vnext/media/fileUrl";

  interface Props {
    items: ClipCandidate[];
    selectedId: string | null;
    mediaPath: string | null;
    onSelect: (id: string) => void;
  }

  let { items, selectedId, mediaPath, onSelect }: Props = $props();

  /** Compact grid of suggested shorts (2 cols, scrollable). */
  const shown = $derived(items.slice(0, 16));
</script>

<div class="flex min-h-0 flex-1 flex-col" data-testid="frame-grid">
  <ul
    class="grid min-h-0 flex-1 grid-cols-2 content-start gap-1.5 overflow-y-auto p-2"
    data-testid="candidate-list"
  >
    {#each shown as c, i (c.id)}
      {@const thumbSrc = pathToPlayableUrl(mediaPath || c.sourceMediaPath)}
      <li>
        <button
          type="button"
          class="group flex w-full flex-col overflow-hidden rounded-xl border bg-surface-950/80 text-left transition
            {selectedId === c.id
            ? 'border-vigil-500 ring-2 ring-vigil-500/40'
            : 'border-surface-700 hover:border-surface-500'}"
          onclick={() => onSelect(c.id)}
          data-testid="candidate-item"
        >
          <div class="relative aspect-[9/14] w-full bg-surface-900">
            {#if thumbSrc}
              <video
                class="h-full w-full object-cover"
                style="object-position: {(c.framing.centerX * 100).toFixed(0)}% {(c.framing.centerY * 100).toFixed(0)}%"
                src={thumbSrc}
                muted
                playsinline
                preload="metadata"
                onloadedmetadata={(e) => {
                  const v = e.currentTarget;
                  try {
                    v.currentTime = Math.max(0, c.start + Math.min(0.4, c.duration * 0.15));
                  } catch {
                    /* ignore */
                  }
                }}
              ></video>
            {:else}
              <div class="flex h-full items-center justify-center text-[10px] text-surface-600">
                Short {i + 1}
              </div>
            {/if}
            <span
              class="absolute left-1 top-1 rounded bg-black/70 px-1 py-0.5 font-mono text-[8px] text-white"
            >{formatDuration(c.duration)}</span
            >
          </div>
          <div class="space-y-0.5 p-1">
            <p class="truncate text-[9px] font-semibold text-surface-100">
              {c.title || `Short ${i + 1}`}
            </p>
            <p class="truncate text-[8px] text-surface-500">{scoreToHuman(c.score)}</p>
          </div>
        </button>
      </li>
    {/each}
  </ul>
</div>
