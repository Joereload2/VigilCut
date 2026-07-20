<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";

  interface Props {
    onApply: () => void;
    /** Rare path: pick destination (factory default is 1-click next to source). */
    onApplyAs?: () => void;
    onListenResult: () => void;
  }
  let { onApply, onApplyAs, onListenResult }: Props = $props();

  const seg = $derived(projectStore.selectedSegment);
  const pending = $derived(projectStore.pendingExceptionCount);
  const ready = $derived(pending === 0 && projectStore.segments.length > 0);
  /** Export is valid when we have keep ranges (EDL or segments), not only "keep" badges */
  const canApply = $derived(
    !projectStore.busy &&
      !!projectStore.mediaPath &&
      !projectStore.mediaPath.startsWith("demo://") &&
      (projectStore.keepRanges.length > 0 ||
        projectStore.keptDuration > 0.05 ||
        projectStore.keepCount > 0),
  );
  const canListen = $derived(canApply);

  function markKeep() {
    if (!seg) {
      projectStore.statusMessage = "Elige una excepción o tramo";
      return;
    }
    projectStore.markAndAdvance(seg.id, "keep");
    projectStore.statusMessage = "Queda · siguiente";
  }

  function markCut() {
    if (!seg) {
      projectStore.statusMessage = "Elige una excepción o tramo";
      return;
    }
    projectStore.markAndAdvance(seg.id, "cut");
    projectStore.statusMessage = "Cortar · siguiente";
  }
</script>

<div class="flex shrink-0 flex-col gap-2">
  {#if projectStore.segments.length > 0}
    <div
      class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-surface-800 bg-surface-900/60 px-3 py-1.5 text-[11px] text-surface-400"
    >
      <span class="font-semibold text-cut">Auto {projectStore.autoCutCount}</span>
      <span class="text-surface-600">·</span>
      <span class="font-semibold text-warning">Excepciones {pending}</span>
      <span class="text-surface-600">·</span>
      <span
        >Final
        <strong class="font-mono text-keep"
          >{formatTime(
            projectStore.estimate?.estimatedDuration ?? projectStore.keptDuration,
          )}</strong
        ></span
      >
      {#if ready}
        <span class="ml-auto rounded-full bg-keep/15 px-2 py-0.5 text-[10px] font-semibold text-keep"
          >Listo — oír y exportar</span
        >
      {:else}
        <span class="ml-auto text-[10px] text-warning"
          >Resuelve excepciones arriba (o exporta: pendientes se conservan)</span
        >
      {/if}
    </div>
  {/if}

  <div
    class="flex flex-wrap items-center gap-2 rounded-xl border border-surface-800 bg-surface-900 px-3 py-2.5 shadow-panel"
  >
    <!-- Primary factory actions first (left-to-right reading: do the job) -->
    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="btn h-11 border border-vigil-600/50 bg-vigil-950 px-4 text-sm font-semibold text-vigil-300 hover:bg-vigil-900 disabled:opacity-35"
        disabled={!canListen}
        onclick={onListenResult}
        title="Reproduce el resultado saltando cortes"
      >
        ▶ Oír resultado
      </button>
      <button
        type="button"
        class="btn h-11 bg-vigil-500 px-6 text-sm font-bold text-white shadow-md shadow-vigil-950/50 hover:bg-vigil-400 disabled:opacity-35
          {ready ? 'ring-2 ring-keep/40' : ''}"
        disabled={!canApply}
        onclick={onApply}
        title="Ctrl+Enter — exporta al lado del original (sin diálogo)"
      >
        {#if projectStore.busy}
          Exportando…
        {:else}
          Exportar video
        {/if}
      </button>
      {#if onApplyAs}
        <button
          type="button"
          class="btn-ghost h-11 px-2 text-[11px] text-surface-500 hover:text-surface-300 disabled:opacity-35"
          disabled={!canApply || projectStore.busy}
          onclick={onApplyAs}
          title="Ctrl+Shift+Enter — elegir carpeta/nombre"
        >
          Otro destino…
        </button>
      {/if}
      <label
        class="flex h-11 cursor-pointer items-center gap-2 rounded-lg border px-2.5 text-[11px] transition
          {projectStore.audioEnhance.enabled
          ? 'border-vigil-500/50 bg-vigil-950/50 text-vigil-200'
          : 'border-surface-700 bg-surface-950 text-surface-400 hover:border-surface-500'}"
        title="Denoise + normalización al exportar el MP4"
      >
        <input
          type="checkbox"
          class="accent-vigil-500"
          checked={projectStore.audioEnhance.enabled}
          onchange={(e) =>
            projectStore.setAudioEnhanceEnabled((e.currentTarget as HTMLInputElement).checked)}
        />
        <span class="font-medium leading-tight">
          Audio enhance
          <span class="block text-[9px] opacity-70">denoise · loudnorm</span>
        </span>
      </label>
    </div>

    <div class="mx-1 hidden h-8 w-px bg-surface-700 sm:block"></div>

    <div class="flex items-center gap-3 text-xs">
      <div class="text-center">
        <div class="font-mono text-sm font-semibold text-keep">
          {formatTime(projectStore.estimate?.estimatedDuration ?? projectStore.keptDuration)}
        </div>
        <div class="text-[10px] text-surface-500">queda</div>
      </div>
      <div class="text-center">
        <div class="font-mono text-sm font-semibold text-cut">
          {formatTime(
            projectStore.estimate?.cutDuration ?? projectStore.cutDuration,
          )}
        </div>
        <div class="text-[10px] text-surface-500">se quita</div>
      </div>
    </div>

    <!-- Manual K/X only when something still needs a human (or user selected a tramo) -->
    {#if pending > 0 || (seg && (seg.needsReview || seg.decision === "pending"))}
      <div class="ml-auto flex items-center gap-2">
        <div class="hidden min-w-0 text-right sm:block">
          <div class="text-[10px] uppercase tracking-wide text-surface-500">Excepción</div>
          {#if seg}
            <div class="font-mono text-[11px] text-surface-300">
              {formatTime(seg.start)}–{formatTime(seg.end)}
            </div>
          {/if}
        </div>
        <button
          type="button"
          class="btn h-10 min-w-[5.5rem] bg-keep px-3 text-sm font-bold text-white hover:bg-green-400 disabled:opacity-35"
          disabled={!seg || projectStore.busy}
          onclick={markKeep}
          title="K — conservar y siguiente"
        >
          Queda <kbd class="ml-0.5 rounded bg-black/20 px-1 text-[10px] font-normal">K</kbd>
        </button>
        <button
          type="button"
          class="btn h-10 min-w-[5.5rem] bg-cut px-3 text-sm font-bold text-white hover:bg-red-400 disabled:opacity-35"
          disabled={!seg || projectStore.busy}
          onclick={markCut}
          title="X — cortar y siguiente"
        >
          Cortar <kbd class="ml-0.5 rounded bg-black/20 px-1 text-[10px] font-normal">X</kbd>
        </button>
      </div>
    {/if}
  </div>
</div>
