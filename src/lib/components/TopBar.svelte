<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import type { FfmpegStatus } from "$lib/types";
  import { formatTime } from "$lib/types";

  interface Props {
    ffmpeg: FfmpegStatus | null;
    version: string;
    onOpen: () => void;
    onReanalyze: () => void;
  }

  let { ffmpeg, version, onOpen, onReanalyze }: Props = $props();

  const hasProject = $derived(!!projectStore.mediaPath);
</script>

<header
  class="flex h-12 shrink-0 items-center gap-2 border-b border-surface-800 bg-surface-950 px-3 sm:gap-3 sm:px-4"
>
  <div class="flex shrink-0 items-center gap-2">
    <div
      class="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-vigil-500 to-emerald-700 text-sm font-bold text-white"
    >
      V
    </div>
    <div class="hidden sm:block">
      <div class="text-sm font-semibold leading-none">VigilCut</div>
      <div
        class="mt-0.5 max-w-[180px] truncate text-[10px] text-surface-500"
        title={projectStore.fileName ?? ""}
      >
        {projectStore.fileName ?? `v${version}`}
      </div>
    </div>
  </div>

  <div class="mx-1 hidden h-6 w-px bg-surface-800 sm:block"></div>

  <button class="btn-primary" onclick={onOpen} disabled={projectStore.busy}>
    {hasProject ? "Otro video" : "Abrir video"}
  </button>

  {#if hasProject}
    <button
      class="btn-secondary hidden sm:inline-flex"
      onclick={onReanalyze}
      disabled={projectStore.busy}
      title="Vuelve a detectar silencios"
    >
      Re-detectar
    </button>
  {/if}

  <div class="flex-1"></div>

  {#if projectStore.media}
    <div class="hidden items-center gap-3 text-xs text-surface-400 lg:flex">
      <span
        >{formatTime(projectStore.duration)}
        <span class="text-surface-600">→</span>
        <strong class="font-mono text-keep"
          >{formatTime(
            projectStore.estimate?.estimatedDuration ?? projectStore.keptDuration,
          )}</strong
        ></span
      >
      {#if projectStore.pendingExceptionCount > 0}
        <span
          class="rounded-full border border-warning/40 bg-warning/10 px-2 py-0.5 font-mono text-[10px] text-warning"
        >
          {projectStore.pendingExceptionCount} excepciones
        </span>
      {:else if projectStore.analysisRun}
        <span
          class="rounded-full border border-keep/30 bg-keep/10 px-2 py-0.5 text-[10px] text-keep"
        >
          listo
        </span>
      {/if}
    </div>
  {/if}

  <span
    class="hidden items-center gap-1.5 rounded-full border px-2 py-0.5 text-[10px] sm:inline-flex
      {ffmpeg?.available
      ? 'border-vigil-800 bg-vigil-950/40 text-vigil-300'
      : 'border-amber-800 bg-amber-950/40 text-amber-300'}"
    title={ffmpeg?.version ?? ffmpeg?.error ?? ""}
  >
    <span
      class="h-1.5 w-1.5 rounded-full {ffmpeg?.available ? 'bg-vigil-400' : 'bg-amber-400'}"
    ></span>
    FFmpeg
  </span>
</header>
