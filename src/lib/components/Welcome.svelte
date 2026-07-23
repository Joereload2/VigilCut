<script lang="ts">
  import { onMount } from "svelte";
  import BatchPanel from "$lib/components/BatchPanel.svelte";
  import * as api from "$lib/utils/tauri";
  import type { ProjectSummary } from "$lib/types";

  interface Props {
    onOpen: () => void;
    onGoSilence?: () => void;
    onGoClips?: () => void;
    onGoVisual?: () => void;
    onGoLibrary?: () => void;
    onOpenPath?: (path: string) => void;
  }
  let { onOpen, onGoSilence, onGoClips, onGoVisual, onGoLibrary, onOpenPath }: Props = $props();

  type HomeTab = "start" | "recent" | "factory";
  let tab = $state<HomeTab>("start");
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
        .slice(0, 12);
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

  const tabs: { id: HomeTab; label: string }[] = [
    { id: "start", label: "Empezar" },
    { id: "recent", label: "Recientes" },
    { id: "factory", label: "Fábrica" },
  ];
</script>

<div class="flex h-full min-h-0 w-full flex-col overflow-hidden">
  <!-- Compact header -->
  <div class="flex shrink-0 items-center gap-3 border-b border-surface-800/80 px-4 py-3 sm:px-6">
    <div
      class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-vigil-500 to-emerald-800 text-base font-bold text-white shadow-panel"
    >
      V
    </div>
    <div class="min-w-0 flex-1 text-left">
      <h1 class="text-base font-semibold tracking-tight text-white sm:text-lg">
        VigilCut Factory
      </h1>
      <p class="truncate text-[11px] text-surface-500">
        La IA prepara el material. Tú supervisas y exportas.
      </p>
    </div>
    <button type="button" class="btn-primary shrink-0 text-xs" onclick={onOpen}>
      Abrir video
    </button>
  </div>

  <!-- Tabs -->
  <div
    class="flex shrink-0 gap-1 border-b border-surface-800 bg-surface-950/80 px-3 pt-2 sm:px-6"
    role="tablist"
    aria-label="Inicio"
  >
    {#each tabs as t (t.id)}
      <button
        type="button"
        role="tab"
        aria-selected={tab === t.id}
        class="rounded-t-lg px-3 py-2 text-xs font-semibold transition
          {tab === t.id
          ? 'bg-surface-900 text-white ring-1 ring-inset ring-surface-700'
          : 'text-surface-500 hover:bg-surface-900/50 hover:text-surface-300'}"
        onclick={() => (tab = t.id)}
      >
        {t.label}
        {#if t.id === "recent" && recent.length > 0}
          <span class="ml-1 font-mono text-[10px] text-surface-500">{recent.length}</span>
        {/if}
      </button>
    {/each}
  </div>

  <!-- Tab panels: single viewport, no page scroll cascade -->
  <div class="min-h-0 flex-1 overflow-hidden bg-surface-950">
    {#if tab === "start"}
      <div
        class="flex h-full min-h-0 flex-col gap-3 overflow-y-auto p-4 sm:p-6"
        role="tabpanel"
      >
        <p class="text-left text-[11px] text-surface-500">
          Elige un modo de trabajo. Después de abrir el video podrás cambiar de modo en la barra
          izquierda.
        </p>
        <div class="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
          <button
            type="button"
            class="rounded-xl border border-vigil-600/40 bg-vigil-950/40 p-3.5 text-left transition hover:border-vigil-500 hover:bg-vigil-950/70"
            onclick={() => (onGoSilence ? onGoSilence() : onOpen())}
          >
            <div class="text-xl">✂</div>
            <div class="mt-1.5 text-sm font-bold text-white">Cortar silencios</div>
            <div class="mt-1 text-[10px] leading-snug text-surface-400">
              Limpia pausas, oye el resultado y exporta el vídeo largo.
            </div>
          </button>
          <button
            type="button"
            class="rounded-xl border border-amber-600/40 bg-amber-950/30 p-3.5 text-left transition hover:border-amber-500 hover:bg-amber-950/50"
            onclick={() => (onGoClips ? onGoClips() : onOpen())}
          >
            <div class="text-xl">📱</div>
            <div class="mt-1.5 text-sm font-bold text-white">Shorts / clips 9:16</div>
            <div class="mt-1 text-[10px] leading-snug text-surface-400">
              Encuentra momentos fuertes, revisa y exporta vertical.
            </div>
          </button>
          <button
            type="button"
            class="rounded-xl border border-sky-600/40 bg-sky-950/30 p-3.5 text-left transition hover:border-sky-500 hover:bg-sky-950/50"
            onclick={() => (onGoVisual ? onGoVisual() : onOpen())}
          >
            <div class="text-xl">🖼</div>
            <div class="mt-1.5 text-sm font-bold text-white">Visual / B-roll</div>
            <div class="mt-1 text-[10px] leading-snug text-surface-400">
              Encuentra y coloca imágenes sobre la línea de tiempo de un video.
            </div>
          </button>
          <button
            type="button"
            class="rounded-xl border border-violet-600/40 bg-violet-950/30 p-3.5 text-left transition hover:border-violet-500 hover:bg-violet-950/50"
            onclick={() => (onGoLibrary ? onGoLibrary() : undefined)}
          >
            <div class="text-xl">▦</div>
            <div class="mt-1.5 text-sm font-bold text-white">Biblioteca Visual</div>
            <div class="mt-1 text-[10px] leading-snug text-surface-400">
              Gestiona assets, conceptos, generación y revisión sin abrir un video.
            </div>
          </button>
        </div>

        <div
          class="mt-auto flex flex-wrap items-center justify-between gap-2 rounded-xl border border-surface-800/80 bg-surface-900/40 px-3 py-2.5"
        >
          <span class="text-[10px] text-surface-500">
            También puedes abrir un archivo sin elegir modo (queda en Silencios).
          </span>
          <button type="button" class="btn-secondary text-xs" onclick={onOpen}
            >Solo abrir un video…</button
          >
        </div>
      </div>
    {:else if tab === "recent"}
      <div class="flex h-full min-h-0 flex-col overflow-hidden p-4 sm:p-6" role="tabpanel">
        {#if recent.length === 0}
          <div
            class="flex flex-1 flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-surface-800 text-center"
          >
            <p class="text-sm text-surface-400">Sin proyectos recientes</p>
            <button type="button" class="btn-primary text-xs" onclick={onOpen}>Abrir video</button>
          </div>
        {:else}
          <ul class="min-h-0 flex-1 space-y-1 overflow-y-auto pr-1">
            {#each recent as p (p.id)}
              <li>
                <button
                  type="button"
                  class="flex w-full items-center justify-between gap-2 rounded-lg border border-transparent px-3 py-2.5 text-left transition hover:border-surface-800 hover:bg-surface-900/80"
                  onclick={() => openRecent(p)}
                  title={p.mediaPath}
                >
                  <span class="min-w-0">
                    <span class="block truncate text-sm font-medium text-surface-100">{p.name}</span>
                    <span class="block truncate font-mono text-[10px] text-surface-500"
                      >{shortPath(p.mediaPath)}</span
                    >
                  </span>
                  <span class="shrink-0 text-[11px] font-medium text-vigil-300">Abrir</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else}
      <div class="flex h-full min-h-0 flex-col gap-3 overflow-y-auto p-4 sm:p-6" role="tabpanel">
        {#if paths}
          <div
            class="rounded-xl border border-surface-800 bg-surface-900/50 p-3 text-left text-[11px]"
          >
            <div class="mb-2 font-semibold text-surface-300">Carpetas de fábrica</div>
            <div class="space-y-1.5 font-mono text-surface-500">
              <div class="flex items-center justify-between gap-2">
                <span class="min-w-0 truncate" title={paths.inbox}>inbox: {paths.inbox}</span>
                <button
                  type="button"
                  class="btn-ghost shrink-0 text-[10px]"
                  onclick={() => openDir("inbox")}>Abrir</button
                >
              </div>
              <div class="flex items-center justify-between gap-2">
                <span class="min-w-0 truncate" title={paths.outbox}>outbox: {paths.outbox}</span>
                <button
                  type="button"
                  class="btn-ghost shrink-0 text-[10px]"
                  onclick={() => openDir("outbox")}>Abrir</button
                >
              </div>
            </div>
          </div>
        {/if}
        <div class="min-h-0 flex-1">
          <BatchPanel />
        </div>
      </div>
    {/if}
  </div>
</div>
