<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
</script>

<footer
  class="flex h-8 shrink-0 items-center gap-3 border-t border-surface-800 bg-surface-950 px-3 text-[11px] text-surface-400"
>
  {#if projectStore.busy}
    <span class="inline-flex items-center gap-1.5 text-vigil-400">
      <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-vigil-400"></span>
      Trabajando…
    </span>
  {:else}
    <span class="text-surface-500">●</span>
  {/if}

  <span class="truncate">{projectStore.statusMessage}</span>

  {#if projectStore.error}
    <span class="truncate text-cut" title={projectStore.error}>Error: {projectStore.error}</span>
  {/if}

  <div class="flex-1"></div>

  {#if projectStore.estimate}
    <span class="font-mono text-surface-500">
      export ≈ {formatTime(projectStore.estimate.estimatedDuration)}
      (−{formatTime(projectStore.estimate.cutDuration)})
    </span>
  {/if}

  {#if projectStore.mediaPath}
    <span class="max-w-xs truncate font-mono text-surface-600" title={projectStore.mediaPath}>
      {projectStore.mediaPath}
    </span>
  {/if}
</footer>
