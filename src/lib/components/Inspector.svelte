<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";

  const seg = $derived(projectStore.selectedSegment);
</script>

<div class="panel flex flex-col overflow-hidden">
  <div class="border-b border-surface-800 px-3 py-2">
    <span class="label">Inspector</span>
  </div>

  <div class="space-y-4 p-3 text-sm">
    {#if seg}
      <div>
        <div class="label mb-1">Segmento seleccionado</div>
        <div class="font-mono text-xs text-surface-300">{seg.id.slice(0, 8)}…</div>
      </div>
      <div class="grid grid-cols-2 gap-2 text-xs">
        <div>
          <div class="text-surface-500">Inicio</div>
          <div class="font-mono">{formatTime(seg.start, true)}</div>
        </div>
        <div>
          <div class="text-surface-500">Fin</div>
          <div class="font-mono">{formatTime(seg.end, true)}</div>
        </div>
        <div>
          <div class="text-surface-500">Tipo</div>
          <div class="capitalize">{seg.kind}</div>
        </div>
        <div>
          <div class="text-surface-500">Confianza</div>
          <div class="font-mono">{(seg.confidence * 100).toFixed(0)}%</div>
        </div>
      </div>
      <div class="flex gap-2">
        <button class="btn-primary flex-1 text-xs" onclick={() => projectStore.setDecision(seg.id, "keep")}>
          Keep
        </button>
        <button class="btn-danger flex-1 text-xs" onclick={() => projectStore.setDecision(seg.id, "cut")}>
          Cut
        </button>
      </div>
    {:else}
      <p class="text-xs text-surface-500">Selecciona un segmento en el timeline o la lista.</p>
    {/if}

    <hr class="border-surface-800" />

    <div>
      <div class="label mb-2">Detección de silencios</div>
      <label class="mb-2 block text-xs text-surface-400">
        Mín. silencio (s)
        <input
          type="number"
          step="0.05"
          min="0.1"
          max="5"
          class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-900 px-2 py-1 font-mono text-surface-100"
          bind:value={projectStore.silenceOptions.minSilenceDuration}
        />
      </label>
      <label class="mb-2 block text-xs text-surface-400">
        Padding (s)
        <input
          type="number"
          step="0.01"
          min="0"
          max="1"
          class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-900 px-2 py-1 font-mono text-surface-100"
          bind:value={projectStore.silenceOptions.padding}
        />
      </label>
      <label class="mb-2 block text-xs text-surface-400">
        Umbral {projectStore.silenceOptions.threshold.toFixed(2)}
        <input
          type="range"
          min="0.1"
          max="0.9"
          step="0.05"
          class="mt-1 w-full accent-vigil-500"
          bind:value={projectStore.silenceOptions.threshold}
        />
      </label>
      <label class="flex items-center gap-2 text-xs text-surface-300">
        <input type="checkbox" class="accent-vigil-500" bind:checked={projectStore.silenceOptions.autoCutSilence} />
        Auto-cut silencios
      </label>
      <label class="mt-2 flex items-center gap-2 text-xs text-surface-300">
        <input type="checkbox" class="accent-vigil-500" bind:checked={projectStore.silenceOptions.preferSilero} />
        Preferir Silero VAD
      </label>
    </div>
  </div>
</div>
