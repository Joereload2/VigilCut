<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
</script>

<footer
  class="flex h-7 shrink-0 items-center gap-2 border-t border-surface-800 bg-surface-950 px-3 text-[11px]"
>
  {#if projectStore.busy}
    <span class="inline-flex items-center gap-1.5 text-vigil-400">
      <span class="h-1.5 w-1.5 animate-pulse rounded-full bg-vigil-400"></span>
      Procesando…
    </span>
  {:else if projectStore.error}
    <span class="truncate text-cut" title={projectStore.error}>{projectStore.error}</span>
  {:else}
    <span class="truncate text-surface-500">{projectStore.statusMessage}</span>
  {/if}

  <div class="flex-1"></div>

  <span class="hidden text-surface-600 sm:inline"
    >K mantener · X cortar · Espacio play · Ctrl+Enter exportar</span
  >
</footer>
