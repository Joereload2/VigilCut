<script lang="ts">
  import { onMount } from "svelte";
  import { projectStore } from "$lib/stores/project.svelte";

  onMount(() => {
    void projectStore.refreshPresets();
  });
</script>

<div class="panel overflow-hidden">
  <div class="border-b border-surface-800 px-3 py-2">
    <span class="label">Presets</span>
  </div>
  <div class="max-h-40 space-y-1 overflow-y-auto p-2">
    {#if projectStore.presets.length === 0}
      <p class="px-1 text-[11px] text-surface-500">
        Default, Podcast, YouTube, Gentle, Clip Select (al iniciar backend)
      </p>
      <button
        class="btn-secondary w-full justify-start text-xs"
        onclick={() =>
          projectStore.applyPreset({
            id: "default",
            name: "Default",
            silence: projectStore.silenceOptions,
            audio: {
              enabled: false,
              denoise: true,
              denoiseStrength: 0.35,
              normalize: true,
              targetLufs: -14,
              compress: false,
            },
            color: {
              enabled: false,
              brightness: 0,
              contrast: 1,
              saturation: 1,
              gamma: 1,
              autoLevels: false,
            },
            export: {
              container: "mp4",
              videoCodec: "libx264",
              audioCodec: "aac",
              crf: 18,
              preset: "medium",
              audioBitrateK: 192,
              reencode: true,
              applyCuts: true,
            },
            isBuiltin: true,
          })}
      >
        Default / Predeterminado
      </button>
    {:else}
      {#each projectStore.presets as p (p.id)}
        <button
          type="button"
          class="w-full rounded-lg border px-2 py-1.5 text-left text-xs transition
            {projectStore.activePresetId === p.id
            ? 'border-vigil-600 bg-vigil-950/50 text-vigil-100'
            : 'border-transparent hover:bg-surface-800 text-surface-200'}"
          onclick={() => projectStore.applyPreset(p)}
          title={p.description ?? ""}
        >
          <div class="font-medium">{p.name}</div>
          {#if p.description}
            <div class="truncate text-[10px] text-surface-500">{p.description}</div>
          {/if}
        </button>
      {/each}
    {/if}
  </div>
</div>
