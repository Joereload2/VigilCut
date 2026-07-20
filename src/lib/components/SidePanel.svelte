<script lang="ts">
  import { onMount } from "svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";

  let settingsOpen = $state(false);

  onMount(() => {
    void projectStore.refreshPresets();
  });

  const stats = $derived(projectStore.analysisRun?.stats);
  const method = $derived(projectStore.analysisRun?.method ?? "—");
  /** Only human-relevant rows — not the full 100+ tramo dump */
  const reviewRows = $derived(
    projectStore.segments.filter((s) => s.needsReview || s.decision === "pending"),
  );
</script>

<div class="panel flex min-h-0 flex-1 flex-col overflow-hidden">
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="text-sm font-semibold text-surface-100">Fábrica</div>
    <div class="text-[10px] text-surface-500">
      Stats del run · solo lo que aún pide humano
    </div>
  </div>

  {#if stats}
    <div class="grid grid-cols-2 gap-1.5 border-b border-surface-800 p-2 text-[10px]">
      <div class="rounded-lg bg-surface-950/80 px-2 py-1.5">
        <div class="text-surface-500">Auto-cortes</div>
        <div class="font-mono text-sm font-semibold text-cut">{stats.autoCutCount}</div>
      </div>
      <div class="rounded-lg bg-surface-950/80 px-2 py-1.5">
        <div class="text-surface-500">Excepciones</div>
        <div class="font-mono text-sm font-semibold text-warning">
          {stats.pendingExceptionCount}
        </div>
      </div>
      <div class="rounded-lg bg-surface-950/80 px-2 py-1.5">
        <div class="text-surface-500">Final</div>
        <div class="font-mono text-sm text-keep">{formatTime(stats.outputDuration)}</div>
      </div>
      <div class="rounded-lg bg-surface-950/80 px-2 py-1.5">
        <div class="text-surface-500">Recortado</div>
        <div class="font-mono text-sm text-cut">−{formatTime(stats.autoRemovedDuration)}</div>
      </div>
      <div class="col-span-2 truncate rounded-lg bg-surface-950/80 px-2 py-1 text-surface-500">
        motor: <span class="font-mono text-surface-300">{method}</span>
      </div>
    </div>
  {/if}

  <div class="min-h-0 flex-1 overflow-y-auto p-1.5">
    {#if !projectStore.mediaPath}
      <p class="p-3 text-center text-xs text-surface-500">Abre un video para arrancar la fábrica.</p>
    {:else if reviewRows.length === 0}
      <p class="p-3 text-center text-xs text-surface-500">
        {projectStore.analysisRun
          ? "Nada pendiente aquí. Usa Supervisión → Oír → Exportar."
          : "Analizando…"}
      </p>
    {:else}
      <div class="mb-1 px-1 text-[10px] font-medium uppercase tracking-wide text-warning/80">
        Pendiente de humano ({reviewRows.length})
      </div>
      <ul class="space-y-0.5">
        {#each reviewRows as seg (seg.id)}
          <li class="flex items-stretch gap-1 rounded-lg border border-warning/20 bg-warning/5 px-1 py-1">
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-2 px-1.5 py-1 text-left text-xs"
              onclick={() => projectStore.selectSegment(seg.id)}
            >
              <span class="h-2 w-2 shrink-0 rounded-full bg-warning"></span>
              <span class="w-11 shrink-0 font-mono text-surface-400">{formatTime(seg.start)}</span>
              <span class="min-w-0 flex-1 truncate text-surface-200">Revisar</span>
              <span class="shrink-0 font-mono text-surface-500"
                >{formatTime(seg.end - seg.start)}</span
              >
            </button>
            <button
              type="button"
              class="shrink-0 self-center rounded-md bg-cut/20 px-2 py-1 text-[10px] font-bold text-cut"
              onclick={() => projectStore.markAndAdvance(seg.id, "cut")}
            >
              Cortar
            </button>
            <button
              type="button"
              class="shrink-0 self-center rounded-md bg-keep/20 px-2 py-1 text-[10px] font-bold text-keep"
              onclick={() => projectStore.markAndAdvance(seg.id, "keep")}
            >
              Queda
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="border-t border-surface-800">
    <button
      type="button"
      class="flex w-full items-center justify-between px-3 py-2 text-left text-xs text-surface-400 hover:bg-surface-800/50 hover:text-surface-200"
      onclick={() => (settingsOpen = !settingsOpen)}
    >
      <span class="font-medium">Policy / detección (avanzado)</span>
      <span class="text-surface-600">{settingsOpen ? "▾" : "▸"}</span>
    </button>

    {#if settingsOpen}
      <div class="space-y-3 border-t border-surface-800/80 px-3 pb-3 pt-2">
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

        {#if projectStore.project}
          <div class="mt-2 border-t border-surface-800 pt-2">
            <div class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-surface-500">
              Al exportar
            </div>
            <label class="flex items-center gap-2 text-[11px] text-surface-300">
              <input
                type="checkbox"
                class="accent-vigil-500"
                bind:checked={projectStore.project.preset.audio.enabled}
              />
              Audio enhance (denoise + loudnorm)
            </label>
            <p class="mt-1 text-[9px] leading-snug text-surface-600">
              Se aplica de verdad en el MP4 exportado (no solo preview).
            </p>
          </div>
        {/if}

        <button
          type="button"
          class="btn-secondary w-full text-xs"
          disabled={projectStore.busy || !projectStore.mediaPath}
          onclick={() => projectStore.reanalyze()}
        >
          Re-analizar con policy actual
        </button>
      </div>
    {/if}
  </div>
</div>
