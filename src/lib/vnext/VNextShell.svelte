<script lang="ts">
  /**
   * Flujo Short en 2 partes de edición:
   * 1) candidates → elegir momento + duración (video + texto)
   * 2) edit → solo marcos verdes / qué se ve en 9:16
   */
  import { onMount, onDestroy } from "svelte";
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import ProjectsPage from "$lib/vnext/pages/ProjectsPage.svelte";
  import CreateProjectPage from "$lib/vnext/pages/CreateProjectPage.svelte";
  import ProcessingPage from "$lib/vnext/pages/ProcessingPage.svelte";
  import ShortPickPage from "$lib/vnext/pages/ShortPickPage.svelte";
  import ShortCanvasPage from "$lib/vnext/pages/ShortCanvasPage.svelte";
  import RenderingPage from "$lib/vnext/pages/RenderingPage.svelte";
  import ResultPage from "$lib/vnext/pages/ResultPage.svelte";
  import EmptyCandidatesPage from "$lib/vnext/pages/EmptyCandidatesPage.svelte";
  import InterruptedPage from "$lib/vnext/pages/InterruptedPage.svelte";
  import FailedPage from "$lib/vnext/pages/FailedPage.svelte";

  const route = $derived(vnextSession.route);
  const isHub = $derived(vnextSession.screen === "list");

  onMount(() => {
    void vnextSession.refreshProjectList();
    let unlisten: (() => void) | undefined;
    void import("$lib/utils/tauri").then(({ isTauri }) => {
      if (!isTauri()) return;
      void import("@tauri-apps/api/event").then(({ listen }) => {
        void listen<{ percent?: number; message?: string; job?: string }>(
          "vigilcut://progress",
          (ev) => {
            const p = ev.payload;
            if (!p) return;
            if (
              vnextSession.pendingAction === "analyze" ||
              vnextSession.pendingAction === "create"
            ) {
              if (typeof p.percent === "number") {
                vnextSession.setAnalysisProgress(p.percent, p.message);
              }
            }
            if (vnextSession.project && p.job !== "clipping") {
              void vnextSession.refetchProjectData().then(() => vnextSession.ensureSelection());
            }
          },
        ).then((fn) => {
          unlisten = fn;
        });
      });
    });
    return () => unlisten?.();
  });

  onDestroy(() => {
    vnextSession.stopPolling();
  });
</script>

<div
  class="flex h-full max-h-full min-h-0 w-full flex-col overflow-hidden bg-surface-950 text-surface-100"
  data-testid="vnext-shell"
>
  <!-- Header compacto: más espacio para el canvas -->
  <header class="flex h-8 shrink-0 items-center gap-2 border-b border-surface-800 px-2">
    <div class="flex items-center gap-1.5">
      <div
        class="flex h-5 w-5 items-center justify-center rounded bg-gradient-to-br from-vigil-500 to-emerald-700 text-[10px] font-bold text-white"
      >
        V
      </div>
      <span class="text-xs font-semibold">VigilCut</span>
    </div>
    {#if !isHub}
      <button
        type="button"
        class="text-[11px] text-surface-400 hover:text-white"
        onclick={() => vnextSession.goProjects()}
      >← Inicio</button>
    {/if}
    {#if vnextSession.project && !isHub}
      <span class="max-w-[12rem] truncate text-[10px] text-surface-500">
        {vnextSession.project.title}
      </span>
    {/if}
    <div class="flex-1"></div>
  </header>

  {#if vnextSession.error}
    <div class="shrink-0 border-b border-cut/30 bg-cut/10 px-3 py-1 text-[11px] text-cut">
      {vnextSession.error}
      <button type="button" class="ml-2 underline" onclick={() => vnextSession.clearError()}
        >Cerrar</button
      >
    </div>
  {/if}

  <main class="min-h-0 min-w-0 flex-1 overflow-hidden">
    {#if route === "projects"}
      <ProjectsPage />
    {:else if route === "create"}
      <CreateProjectPage />
    {:else if route === "processing"}
      <ProcessingPage />
    {:else if route === "candidates"}
      <ShortPickPage />
    {:else if route === "edit"}
      <ShortCanvasPage />
    {:else if route === "empty_candidates"}
      <EmptyCandidatesPage />
    {:else if route === "rendering"}
      <RenderingPage />
    {:else if route === "result"}
      <ResultPage />
    {:else if route === "interrupted"}
      <InterruptedPage />
    {:else if route === "failed"}
      <FailedPage />
    {:else}
      <ProjectsPage />
    {/if}
  </main>
</div>
