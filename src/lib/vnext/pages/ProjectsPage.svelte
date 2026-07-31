<script lang="ts">
  import { onMount } from "svelte";
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import { nextActionHuman } from "$lib/vnext/presentation/deriveStage";
  import type { ProjectStage } from "$lib/vnext/types/flow";

  onMount(() => {
    void vnextSession.refreshProjectList();
  });

  function coarseStage(p: { nextAction?: string | null; sourceMediaPath: string }): ProjectStage {
    const na = p.nextAction ?? "";
    if (na.includes("done") || na === "done") return "completed";
    if (na.includes("render")) return "adjusting";
    if (na.includes("review") || na.includes("generate")) return "candidates_ready";
    if (!p.sourceMediaPath) return "source_required";
    return "source_ready";
  }

  function fileName(path: string) {
    return path.split(/[/\\]/).pop() || path;
  }

  function confirmDelete(id: string, title: string, e: Event) {
    e.stopPropagation();
    e.preventDefault();
    const ok = window.confirm(
      `¿Quitar “${title}” del historial?\n\nNo se borra el video original ni los Shorts ya exportados en disco.`,
    );
    if (ok) void vnextSession.deleteProjectFromHistory(id);
  }
</script>

<div class="flex h-full max-h-full min-h-0 flex-col overflow-hidden" data-testid="page-projects">
  <div
    class="mx-auto flex h-full min-h-0 w-full max-w-md flex-col gap-3 overflow-hidden px-3 py-4"
  >
    <div class="shrink-0">
      <h1 class="text-lg font-bold text-white">VigilCut</h1>
      <p class="mt-0.5 text-[11px] text-surface-400">
        Video → marcos verdes → Short 9:16
      </p>
    </div>

    <button
      type="button"
      class="w-full shrink-0 rounded-xl bg-vigil-600 py-2.5 text-sm font-semibold text-white hover:bg-vigil-500"
      data-testid="cta-create-short"
      onclick={() => vnextSession.goCreate()}
    >
      Abrir un video
    </button>

    {#if vnextSession.projects.length > 0}
      <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div class="mb-1 flex shrink-0 items-center justify-between">
          <p class="text-[9px] font-semibold uppercase tracking-wide text-surface-500">
            Continuar ({vnextSession.projects.length})
          </p>
        </div>
        <ul class="min-h-0 flex-1 space-y-1 overflow-y-auto pr-0.5">
          {#each vnextSession.projects as p (p.id)}
            {@const stage = coarseStage(p)}
            <li class="flex items-stretch gap-1">
              <button
                type="button"
                class="min-w-0 flex-1 rounded-lg border border-surface-800 bg-surface-900/40 px-2 py-1.5 text-left hover:border-vigil-500/40"
                onclick={() => void vnextSession.loadProject(p.id)}
              >
                <span class="flex items-center justify-between gap-2">
                  <span class="min-w-0 truncate text-[11px] font-medium text-white"
                    >{p.title || fileName(p.sourceMediaPath)}</span
                  >
                  <span class="shrink-0 text-[9px] text-vigil-300"
                    >{nextActionHuman(stage)} →</span
                  >
                </span>
              </button>
              <button
                type="button"
                class="shrink-0 rounded-lg border border-surface-800 px-2 text-[10px] text-surface-500 hover:border-cut/50 hover:bg-cut/10 hover:text-cut"
                title="Quitar del historial"
                aria-label="Quitar {p.title} del historial"
                disabled={vnextSession.pendingAction === "delete_project"}
                onclick={(e) => confirmDelete(p.id, p.title || fileName(p.sourceMediaPath), e)}
              >
                ✕
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</div>
