<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";

  interface Props {
    onApply: () => void;
    onListenResult: () => void;
  }
  let { onApply, onListenResult }: Props = $props();

  const seg = $derived(projectStore.selectedSegment);
  const canApply = $derived(
    projectStore.segments.length > 0 &&
      projectStore.keepCount > 0 &&
      !projectStore.busy &&
      !!projectStore.mediaPath &&
      !projectStore.mediaPath.startsWith("demo://"),
  );
  const canListen = $derived(projectStore.keepCount > 0 && !projectStore.busy);

  function markKeep() {
    if (!seg) {
      projectStore.statusMessage = "Elige un tramo en la línea de tiempo";
      return;
    }
    projectStore.markAndAdvance(seg.id, "keep");
    projectStore.statusMessage = "Queda · siguiente";
  }

  function markCut() {
    if (!seg) {
      projectStore.statusMessage = "Elige un tramo en la línea de tiempo";
      return;
    }
    projectStore.markAndAdvance(seg.id, "cut");
    projectStore.statusMessage = "Cortar · siguiente";
  }
</script>

<div class="flex shrink-0 flex-col gap-2">
  <!-- Factory status strip -->
  {#if projectStore.segments.length > 0}
    <div
      class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-surface-800 bg-surface-900/60 px-3 py-1.5 text-[11px] text-surface-400"
    >
      <span class="font-semibold text-cut">
        Auto {projectStore.autoCutCount}
      </span>
      <span class="text-surface-600">·</span>
      <span class="font-semibold text-warning">
        Excepciones {projectStore.pendingExceptionCount}
      </span>
      <span class="text-surface-600">·</span>
      <span
        >Silencios <strong class="text-surface-300">{projectStore.silenceCount}</strong></span
      >
      <span class="text-surface-600">·</span>
      <span
        >Final <strong class="font-mono text-keep">{formatTime(projectStore.keptDuration)}</strong
        ></span
      >
      {#if projectStore.pendingExceptionCount === 0}
        <span class="ml-auto rounded-full bg-keep/15 px-2 py-0.5 text-[10px] font-semibold text-keep"
          >Listo para exportar</span
        >
      {:else}
        <span class="ml-auto text-[10px] text-warning"
          >Resuelve excepciones o exporta (pendientes = se conservan)</span
        >
      {/if}
    </div>
  {/if}

  <div
    class="flex flex-wrap items-center gap-2 rounded-xl border border-surface-800 bg-surface-900 px-3 py-2.5 shadow-panel"
  >
    <div class="min-w-0 flex-1 basis-[120px]">
      {#if seg}
        <div class="text-[10px] uppercase tracking-wide text-surface-500">Tramo actual</div>
        <div class="truncate font-mono text-xs text-surface-200">
          {formatTime(seg.start)} – {formatTime(seg.end)}
          <span class="text-surface-500">· {formatTime(seg.end - seg.start)}</span>
        </div>
      {:else}
        <div class="text-xs text-surface-500">Selecciona un tramo arriba</div>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <button
        type="button"
        class="btn h-10 min-w-[6.5rem] bg-keep px-3 text-sm font-bold text-white hover:bg-green-400 disabled:opacity-35"
        disabled={!seg || projectStore.busy}
        onclick={markKeep}
        title="K — mantener y siguiente"
      >
        Mantener <kbd class="ml-0.5 rounded bg-black/20 px-1 text-[10px] font-normal">K</kbd>
      </button>
      <button
        type="button"
        class="btn h-10 min-w-[6.5rem] bg-cut px-3 text-sm font-bold text-white hover:bg-red-400 disabled:opacity-35"
        disabled={!seg || projectStore.busy}
        onclick={markCut}
        title="X — cortar y siguiente"
      >
        Cortar <kbd class="ml-0.5 rounded bg-black/20 px-1 text-[10px] font-normal">X</kbd>
      </button>
    </div>

    <div class="mx-1 hidden h-8 w-px bg-surface-700 sm:block"></div>

    <div class="flex items-center gap-3 text-xs">
      <div class="text-center">
        <div class="font-mono text-sm font-semibold text-keep">
          {formatTime(projectStore.keptDuration)}
        </div>
        <div class="text-[10px] text-surface-500">queda</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-sm font-semibold text-cut">
          {formatTime(projectStore.cutDuration)}
        </div>
        <div class="text-[10px] text-surface-500">se quita</div>
      </div>
    </div>

    <div class="ml-auto flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="btn h-10 border border-vigil-600/50 bg-vigil-950 px-4 text-sm font-semibold text-vigil-300 hover:bg-vigil-900 disabled:opacity-35"
        disabled={!canListen}
        onclick={onListenResult}
        title="Reproduce el resultado final saltando cortes"
      >
        ▶ Oír resultado
      </button>
      <button
        type="button"
        class="btn h-10 bg-vigil-500 px-5 text-sm font-bold text-white shadow-md shadow-vigil-950/50 hover:bg-vigil-400 disabled:opacity-35"
        disabled={!canApply}
        onclick={onApply}
        title="Ctrl+Enter"
      >
        {#if projectStore.busy}
          Exportando…
        {:else}
          Exportar video
        {/if}
      </button>
    </div>
  </div>

  {#if projectStore.keepCount === 0 && projectStore.segments.length > 0}
    <p class="px-1 text-[11px] text-warning">Marca al menos un tramo como Mantener para exportar.</p>
  {/if}
</div>
