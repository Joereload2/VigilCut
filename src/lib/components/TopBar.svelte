<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import type { FfmpegStatus } from "$lib/types";
  import { formatTime } from "$lib/types";
  import { isTauri } from "$lib/utils/tauri";

  interface Props {
    ffmpeg: FfmpegStatus | null;
    version: string;
    onOpen: () => void;
    onExport: () => void;
    onReanalyze: () => void;
  }

  let { ffmpeg, version, onOpen, onExport, onReanalyze }: Props = $props();
</script>

<header
  class="flex h-12 shrink-0 items-center gap-3 border-b border-surface-800 bg-surface-950/90 px-4 backdrop-blur"
>
  <div class="flex items-center gap-2">
    <div
      class="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-to-br from-vigil-500 to-emerald-700 text-sm font-bold text-white shadow"
    >
      V
    </div>
    <div>
      <div class="text-sm font-semibold leading-tight tracking-tight">VigilCut</div>
      <div class="text-[10px] text-surface-500">v{version} · human-in-the-loop</div>
    </div>
  </div>

  <div class="mx-2 h-6 w-px bg-surface-800"></div>

  <button class="btn-primary" onclick={onOpen} disabled={projectStore.busy}>
    <span>📁</span> Abrir / Open
  </button>
  <button
    class="btn-secondary"
    onclick={onReanalyze}
    disabled={projectStore.busy || !projectStore.mediaPath}
  >
    🔎 Re-analizar
  </button>
  <button
    class="btn-primary"
    onclick={onExport}
    disabled={projectStore.busy || projectStore.segments.length === 0}
  >
    ⬆ Exportar
  </button>

  <div class="flex-1"></div>

  {#if projectStore.media}
    <div class="hidden items-center gap-4 text-xs text-surface-400 md:flex">
      <span>
        Fuente
        <strong class="ml-1 font-mono text-surface-200"
          >{formatTime(projectStore.duration)}</strong
        >
      </span>
      <span>
        Keep
        <strong class="ml-1 font-mono text-keep">{formatTime(projectStore.keptDuration)}</strong>
      </span>
      <span>
        Cut
        <strong class="ml-1 font-mono text-cut">{formatTime(projectStore.cutDuration)}</strong>
      </span>
    </div>
  {/if}

  <div class="flex items-center gap-2 text-xs">
    <span
      class="inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5"
      class:border-vigil-700={ffmpeg?.available}
      class:bg-vigil-950={ffmpeg?.available}
      class:text-vigil-300={ffmpeg?.available}
      class:border-amber-700={!ffmpeg?.available}
      class:bg-amber-950={!ffmpeg?.available}
      class:text-amber-300={!ffmpeg?.available}
      title={ffmpeg?.version ?? ffmpeg?.error ?? ""}
    >
      <span
        class="h-1.5 w-1.5 rounded-full"
        class:bg-vigil-400={ffmpeg?.available}
        class:bg-amber-400={!ffmpeg?.available}
      ></span>
      FFmpeg {ffmpeg?.available ? "OK" : "—"}
    </span>
    {#if !isTauri()}
      <span class="rounded-full border border-surface-700 px-2 py-0.5 text-surface-400">
        Demo web
      </span>
    {/if}
  </div>
</header>
