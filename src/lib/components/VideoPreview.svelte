<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isTauri } from "$lib/utils/tauri";

  let videoEl = $state<HTMLVideoElement | null>(null);

  const src = $derived.by(() => {
    const p = projectStore.mediaPath;
    if (!p) return null;
    if (isTauri()) {
      try {
        return convertFileSrc(p);
      } catch {
        return null;
      }
    }
    return null;
  });

  $effect(() => {
    const v = videoEl;
    if (!v) return;
    const onTime = () => {
      let t = v.currentTime;
      if (projectStore.skipCutsPreview && projectStore.isPlaying) {
        const snapped = projectStore.snapPlayheadOverCuts(t);
        if (Math.abs(snapped - t) > 0.04) {
          v.currentTime = snapped;
          t = snapped;
        }
      }
      projectStore.currentTime = t;
    };
    const onPlay = () => (projectStore.isPlaying = true);
    const onPause = () => (projectStore.isPlaying = false);
    v.addEventListener("timeupdate", onTime);
    v.addEventListener("play", onPlay);
    v.addEventListener("pause", onPause);
    return () => {
      v.removeEventListener("timeupdate", onTime);
      v.removeEventListener("play", onPlay);
      v.removeEventListener("pause", onPause);
    };
  });

  $effect(() => {
    const v = videoEl;
    if (!v) return;
    // External seek (timeline click)
    if (Math.abs(v.currentTime - projectStore.currentTime) > 0.15 && !projectStore.isPlaying) {
      v.currentTime = projectStore.currentTime;
    }
  });

  function togglePlay() {
    const v = videoEl;
    if (!v) return;
    if (v.paused) void v.play();
    else v.pause();
  }

  function seek(delta: number) {
    projectStore.currentTime = Math.max(
      0,
      Math.min(projectStore.duration, projectStore.currentTime + delta),
    );
    if (videoEl) videoEl.currentTime = projectStore.currentTime;
  }
</script>

<div class="panel flex min-h-0 flex-1 flex-col overflow-hidden">
  <div class="flex items-center justify-between border-b border-surface-800 px-3 py-2">
    <span class="label">Preview</span>
    <label class="flex items-center gap-2 text-xs text-surface-300">
      <input type="checkbox" bind:checked={projectStore.skipCutsPreview} class="accent-vigil-500" />
      Saltar cortes / Skip cuts
    </label>
  </div>

  <div class="relative flex flex-1 items-center justify-center bg-black">
    {#if src}
      <!-- svelte-ignore a11y_media_has_caption -->
      <video
        bind:this={videoEl}
        class="max-h-full max-w-full"
        src={src}
        playsinline
      ></video>
    {:else}
      <div class="flex flex-col items-center gap-2 p-8 text-center text-surface-500">
        <div class="text-4xl opacity-40">▶</div>
        <p class="text-sm">
          Abre un video para previsualizar.<br />
          <span class="text-surface-600">Open a video to preview.</span>
        </p>
        {#if projectStore.mediaPath && !isTauri()}
          <p class="text-xs text-surface-600">
            Ruta: {projectStore.mediaPath}
          </p>
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex items-center gap-2 border-t border-surface-800 px-3 py-2">
    <button class="btn-ghost px-2" onclick={() => seek(-5)} title="-5s">«</button>
    <button class="btn-secondary min-w-[2.5rem]" onclick={togglePlay}>
      {projectStore.isPlaying ? "❚❚" : "▶"}
    </button>
    <button class="btn-ghost px-2" onclick={() => seek(5)} title="+5s">»</button>
    <span class="font-mono text-xs text-surface-300">
      {formatTime(projectStore.currentTime, true)}
      <span class="text-surface-600">/</span>
      {formatTime(projectStore.duration)}
    </span>
    {#if projectStore.skipCutsPreview}
      <span class="ml-auto text-[10px] text-surface-500">
        editado ≈ {formatTime(projectStore.sourceToEdited(projectStore.currentTime))}
      </span>
    {/if}
  </div>
</div>
