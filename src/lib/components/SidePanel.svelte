<script lang="ts">
  import { onMount } from "svelte";
  import { projectStore } from "$lib/stores/project.svelte";

  interface Props {
    forceSettingsOpen?: boolean;
  }
  let { forceSettingsOpen: _forceSettingsOpen = false }: Props = $props();

  onMount(() => {
    void projectStore.refreshPresets();
  });
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="text-sm font-semibold text-surface-100">Ajustes de corte</div>
    <div class="text-[10px] text-surface-500">Policy · VAD · audio al exportar</div>
  </div>

  <div class="min-h-0 flex-1 space-y-3 overflow-y-auto px-3 py-3">
    {#if projectStore.presets.length > 0}
      <label class="block text-[11px] text-surface-400">
        Preset
        <select
          class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-900 px-2 py-1.5 text-xs text-surface-100"
          value={projectStore.activePresetId}
          onchange={(e) => {
            const id = (e.currentTarget as HTMLSelectElement).value;
            const p = projectStore.presets.find((x) => x.id === id);
            if (p) projectStore.applyPreset(p);
          }}
        >
          {#each projectStore.presets as p (p.id)}
            <option value={p.id}>{p.name}</option>
          {/each}
        </select>
      </label>
    {/if}

    <label class="block text-[11px] text-surface-400">
      Umbral auto-corte (confianza)
      <input
        type="range"
        min="0.55"
        max="0.95"
        step="0.05"
        class="mt-1 w-full accent-vigil-500"
        bind:value={projectStore.silenceOptions.autoApproveMinScore}
      />
      <span class="font-mono text-surface-500"
        >{Math.round(projectStore.silenceOptions.autoApproveMinScore * 100)}%</span
      >
    </label>

    <label class="block text-[11px] text-surface-400">
      Silencio mínimo (s)
      <input
        type="number"
        step="0.05"
        min="0.1"
        max="5"
        class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-900 px-2 py-1 font-mono text-xs text-surface-100"
        bind:value={projectStore.silenceOptions.minSilenceDuration}
      />
    </label>

    <label class="block text-[11px] text-surface-400">
      Margen de habla (s)
      <input
        type="number"
        step="0.01"
        min="0"
        max="1"
        class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-900 px-2 py-1 font-mono text-xs text-surface-100"
        bind:value={projectStore.silenceOptions.padding}
      />
    </label>

    <label class="flex items-center gap-2 text-[11px] text-surface-300">
      <input
        type="checkbox"
        class="accent-vigil-500"
        bind:checked={projectStore.silenceOptions.preferSilero}
      />
      Preferir Silero VAD
    </label>

    <label class="flex items-center gap-2 text-[11px] text-surface-300">
      <input
        type="checkbox"
        class="accent-vigil-500"
        bind:checked={projectStore.silenceOptions.preferWhisper}
      />
      Whisper al re-analizar (lento)
    </label>

    <div class="border-t border-surface-800 pt-2">
      <div class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-surface-500">
        Al exportar
      </div>
      <label class="flex items-center gap-2 text-[11px] text-surface-300">
        <input
          type="checkbox"
          class="accent-vigil-500"
          checked={projectStore.audioEnhance.enabled}
          onchange={(e) =>
            projectStore.setAudioEnhanceEnabled((e.currentTarget as HTMLInputElement).checked)}
        />
        Audio enhance (denoise + loudnorm)
      </label>
    </div>

    <button
      type="button"
      class="btn-secondary w-full text-xs"
      disabled={projectStore.busy || !projectStore.mediaPath}
      onclick={() => projectStore.reanalyze()}
    >
      Re-analizar con policy actual
    </button>
  </div>
</div>
