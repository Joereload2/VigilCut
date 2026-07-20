<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/utils/tauri";
  import type { ProjectSummary } from "$lib/types";

  interface Props {
    onOpen: () => void;
    onGoSilence?: () => void;
    onGoClips?: () => void;
    onOpenPath?: (path: string) => void;
  }
  let { onOpen, onGoSilence, onGoClips, onOpenPath }: Props = $props();

  let paths = $state<{ inbox: string; outbox: string } | null>(null);
  let recent = $state<ProjectSummary[]>([]);

  onMount(() => {
    if (!api.isTauri()) return;
    void api.getFactoryPaths().then((p) => {
      paths = { inbox: p.inbox, outbox: p.outbox };
    });
    void api.listRecentProjects().then((list) => {
      recent = list
        .filter((p) => p.mediaPath && !p.mediaPath.startsWith("demo://"))
        .slice(0, 6);
    });
  });

  async function openDir(which: string) {
    if (!api.isTauri()) return;
    try {
      const dir = await api.openFactoryFolder(which);
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(dir);
    } catch (e) {
      console.error(e);
    }
  }

  function openRecent(p: ProjectSummary) {
    if (onOpenPath) onOpenPath(p.mediaPath);
  }

  function shortPath(p: string) {
    const parts = p.split(/[/\\]/);
    return parts.slice(-2).join("/") || p;
  }
</script>

<div class="flex flex-col items-center justify-center gap-6 px-4 py-8 text-center">
  <div
    class="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-vigil-500 to-emerald-800 text-2xl font-bold text-white shadow-panel"
  >
    V
  </div>

  <div class="max-w-lg">
    <h1 class="text-2xl font-semibold tracking-tight text-white">VigilCut Factory</h1>
    <p class="mt-2 text-sm leading-relaxed text-surface-400">
      La IA prepara el material. Tú supervisas y exportas.
    </p>
  </div>

  <div class="grid w-full max-w-xl grid-cols-1 gap-3 sm:grid-cols-2">
    <button
      type="button"
      class="rounded-2xl border border-vigil-600/40 bg-vigil-950/40 p-4 text-left transition hover:border-vigil-500 hover:bg-vigil-950/70"
      onclick={() => (onGoSilence ? onGoSilence() : onOpen())}
    >
      <div class="text-2xl">✂</div>
      <div class="mt-2 text-sm font-bold text-white">Cortar silencios</div>
      <div class="mt-1 text-[11px] text-surface-400">
        Limpia pausas, oye el resultado y exporta el vídeo largo.
      </div>
    </button>
    <button
      type="button"
      class="rounded-2xl border border-amber-600/40 bg-amber-950/30 p-4 text-left transition hover:border-amber-500 hover:bg-amber-950/50"
      onclick={() => (onGoClips ? onGoClips() : onOpen())}
    >
      <div class="text-2xl">📱</div>
      <div class="mt-2 text-sm font-bold text-white">Shorts / clips 9:16</div>
      <div class="mt-1 text-[11px] text-surface-400">
        Encuentra momentos fuertes, revisa y exporta vertical.
      </div>
    </button>
  </div>

  <button type="button" class="btn-secondary text-xs" onclick={onOpen}>Solo abrir un video…</button>

  {#if recent.length > 0}
    <div class="w-full max-w-xl rounded-xl border border-surface-800 bg-surface-900/50 p-3 text-left">
      <div class="mb-2 text-[11px] font-semibold text-surface-300">Recientes</div>
      <ul class="space-y-1">
        {#each recent as p (p.id)}
          <li>
            <button
              type="button"
              class="flex w-full items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-left transition hover:bg-surface-800/80"
              onclick={() => openRecent(p)}
              title={p.mediaPath}
            >
              <span class="min-w-0">
                <span class="block truncate text-xs font-medium text-surface-100">{p.name}</span>
                <span class="block truncate font-mono text-[10px] text-surface-500"
                  >{shortPath(p.mediaPath)}</span
                >
              </span>
              <span class="shrink-0 text-[10px] text-vigil-300">Abrir</span>
            </button>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if paths}
    <div class="w-full max-w-xl rounded-xl border border-surface-800 bg-surface-900/50 p-3 text-left text-[11px]">
      <div class="mb-2 font-semibold text-surface-300">Lote (carpeta inbox)</div>
      <div class="space-y-1 font-mono text-surface-500">
        <div class="flex items-center justify-between gap-2">
          <span class="truncate" title={paths.inbox}>inbox: {paths.inbox}</span>
          <button type="button" class="btn-ghost shrink-0 text-[10px]" onclick={() => openDir("inbox")}
            >Abrir</button
          >
        </div>
        <div class="flex items-center justify-between gap-2">
          <span class="truncate" title={paths.outbox}>outbox: {paths.outbox}</span>
          <button type="button" class="btn-ghost shrink-0 text-[10px]" onclick={() => openDir("outbox")}
            >Abrir</button
          >
        </div>
      </div>
    </div>
  {/if}
</div>
