<script lang="ts">
  import { onMount } from "svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime, type Segment } from "$lib/types";

  let settingsOpen = $state(false);

  onMount(() => {
    void projectStore.refreshPresets();
  });

  function rowClass(seg: Segment) {
    const sel = projectStore.selectedSegmentId === seg.id;
    if (seg.decision === "cut") {
      return sel
        ? "border-cut/60 bg-cut/15"
        : "border-transparent hover:bg-cut/10";
    }
    if (seg.decision === "keep") {
      return sel
        ? "border-keep/60 bg-keep/15"
        : "border-transparent hover:bg-keep/10";
    }
    return sel
      ? "border-warning/50 bg-warning/10"
      : "border-transparent hover:bg-surface-800";
  }

  function kindLabel(kind: string) {
    if (kind === "silence") return "Silencio";
    if (kind === "speech") return "Habla";
    if (kind === "manual") return "Manual";
    return kind;
  }
</script>

<div class="panel flex min-h-0 flex-1 flex-col overflow-hidden">
  <!-- Header -->
  <div class="flex items-center justify-between border-b border-surface-800 px-3 py-2.5">
    <div>
      <div class="text-sm font-semibold text-surface-100">Tramos</div>
      <div class="text-[10px] text-surface-500">
        Clic = ir · badge = decidir y seguir
      </div>
    </div>
    <div class="text-right text-[10px] text-surface-400">
      <div class="font-mono">
        <span class="text-keep">{projectStore.keepCount}</span>
        <span class="text-surface-600">/</span>
        <span class="text-cut">{projectStore.cutCount}</span>
      </div>
      <div class="text-surface-500">
        {projectStore.reviewPosition.current}/{projectStore.reviewPosition.total}
      </div>
    </div>
  </div>

  <!-- List -->
  <div class="min-h-0 flex-1 overflow-y-auto p-1.5">
    {#if projectStore.segments.length === 0}
      <p class="p-4 text-center text-xs text-surface-500">
        Abre un video: se detectarán silencios automáticamente.
      </p>
    {:else}
      <ul class="space-y-0.5">
        {#each projectStore.segments as seg (seg.id)}
          <li class="flex items-stretch gap-1 rounded-lg border px-1 py-1 {rowClass(seg)}">
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-2 px-1.5 py-1 text-left text-xs"
              onclick={() => projectStore.selectSegment(seg.id)}
            >
              <span
                class="h-2 w-2 shrink-0 rounded-full
                  {seg.decision === 'cut'
                  ? 'bg-cut'
                  : seg.decision === 'keep'
                    ? 'bg-keep'
                    : 'bg-warning'}"
              ></span>
              <span class="w-11 shrink-0 font-mono text-surface-400">{formatTime(seg.start)}</span>
              <span class="min-w-0 flex-1 truncate text-surface-200">{kindLabel(seg.kind)}</span>
              <span class="shrink-0 font-mono text-surface-500"
                >{formatTime(seg.end - seg.start)}</span
              >
            </button>
            <button
              type="button"
              class="shrink-0 self-center rounded-md px-2 py-1 text-[10px] font-bold uppercase tracking-wide
                {seg.decision === 'cut'
                ? 'bg-cut/25 text-cut'
                : 'bg-keep/25 text-keep'}"
              title="Cambiar y pasar al siguiente (igual que K/X)"
              onclick={() => projectStore.toggleAndAdvance(seg.id)}
            >
              {seg.decision === "cut" ? "Cortar" : "Queda"}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Collapsible settings -->
  <div class="border-t border-surface-800">
    <button
      type="button"
      class="flex w-full items-center justify-between px-3 py-2 text-left text-xs text-surface-400 hover:bg-surface-800/50 hover:text-surface-200"
      onclick={() => (settingsOpen = !settingsOpen)}
    >
      <span class="font-medium">Ajustes de detección</span>
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

        <label class="block text-[11px] text-surface-400">
          Sensibilidad
          <input
            type="range"
            min="0.1"
            max="0.9"
            step="0.05"
            class="mt-1 w-full accent-vigil-500"
            bind:value={projectStore.silenceOptions.threshold}
          />
        </label>

        <label class="flex items-center gap-2 text-[11px] text-surface-300">
          <input
            type="checkbox"
            class="accent-vigil-500"
            bind:checked={projectStore.silenceOptions.autoCutSilence}
          />
          Marcar silencios como cortar
        </label>

        <button
          type="button"
          class="btn-secondary w-full text-xs"
          disabled={projectStore.busy || !projectStore.mediaPath}
          onclick={() => projectStore.reanalyze()}
        >
          Volver a detectar
        </button>
      </div>
    {/if}
  </div>
</div>
