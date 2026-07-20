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

  function togglePlayFromBar() {
    window.dispatchEvent(new CustomEvent("vigilcut:toggle-play"));
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

  <!-- Row 1: main actions -->
  <div
    class="flex flex-wrap items-center gap-2 rounded-xl border border-surface-800 bg-surface-900 px-3 py-2.5 shadow-panel"
  >
    <button
      type="button"
      class="btn h-11 min-w-[5.5rem] border border-surface-600 bg-surface-800 px-4 text-sm font-bold text-white hover:bg-surface-700"
      onclick={togglePlayFromBar}
      title="Espacio — play / pausa"
    >
      {projectStore.isPlaying ? "⏸ Pausa" : "▶ Play"}
    </button>
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
          {formatTime(projectStore.estimate?.cutDuration ?? projectStore.cutDuration)}
        </div>
        <div class="text-[10px] text-surface-500">se quita</div>
      </div>
    </div>

    {#if pending > 0 || (seg && (seg.needsReview || seg.decision === "pending"))}
      <div class="ml-auto flex items-center gap-2">
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

  <!-- Row 2: Audio enhance — always full width, never lost in wrap -->
  <div
    class="flex flex-wrap items-center justify-between gap-2 rounded-xl border px-3 py-2.5 transition
      {projectStore.audioEnhance.enabled
      ? 'border-vigil-500/50 bg-vigil-950/40'
      : 'border-surface-800 bg-surface-900/80'}"
  >
    <label class="flex min-w-0 cursor-pointer items-center gap-3">
      <input
        type="checkbox"
        class="h-4 w-4 shrink-0 accent-vigil-500"
        checked={projectStore.audioEnhance.enabled}
        onchange={(e) =>
          projectStore.setAudioEnhanceEnabled((e.currentTarget as HTMLInputElement).checked)}
      />
      <span class="min-w-0">
        <span class="block text-sm font-semibold text-surface-100">Audio enhance al exportar</span>
        <span class="block text-[11px] text-surface-400"
          >Denoise + normalización loudnorm en el MP4 final</span
        >
      </span>
    </label>
    <span
      class="shrink-0 rounded-full px-2.5 py-1 text-[10px] font-bold uppercase tracking-wide
        {projectStore.audioEnhance.enabled
        ? 'bg-keep/20 text-keep'
        : 'bg-surface-800 text-surface-500'}"
    >
      {projectStore.audioEnhance.enabled ? "ON" : "OFF"}
    </span>
  </div>
</div>
