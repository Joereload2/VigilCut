<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
</script>

<footer
  class="flex min-h-8 shrink-0 flex-wrap items-center gap-x-2 gap-y-0.5 border-t border-surface-800 bg-surface-950 px-3 py-1.5 text-[11px]"
  style="box-sizing:border-box"
>
  {#if projectStore.busy}
    <span class="inline-flex min-w-0 items-center gap-1.5 text-vigil-400">
      <span class="h-1.5 w-1.5 shrink-0 animate-pulse rounded-full bg-vigil-400"></span>
      <span class="min-w-0 break-words">Procesando… {projectStore.statusMessage || ""}</span>
    </span>
  {:else if projectStore.error}
    <span class="min-w-0 flex-1 break-words text-cut" title={projectStore.error}
      >{projectStore.error}</span
    >
  {:else}
    <span class="min-w-0 flex-1 break-words text-surface-500">{projectStore.statusMessage}</span>
  {/if}

  <span
    class="shrink-0 text-[10px] text-surface-600"
    title="Atajos de teclado"
  >
    K mantener · X cortar · Espacio play · Ctrl+Enter exportar
  </span>
</footer>
