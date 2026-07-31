<script lang="ts">
  import { onMount } from "svelte";
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import { nextActionHuman } from "$lib/vnext/presentation/deriveStage";
  import { projectStageToHumanLabel } from "$lib/vnext/presentation/jobLabels";
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
</script>

<div class="flex h-full min-h-0 flex-col" data-testid="page-projects">
  <div class="mx-auto flex w-full max-w-xl flex-1 flex-col justify-center gap-6 px-4 py-10">
    <div>
      <h1 class="text-2xl font-bold text-white">VigilCut</h1>
      <p class="mt-2 text-sm text-surface-400">
        Elegí un video. Recortá con marcos verdes. Creá el Short 9:16.
      </p>
    </div>

    <button
      type="button"
      class="w-full rounded-2xl bg-vigil-600 py-4 text-base font-semibold text-white hover:bg-vigil-500"
      data-testid="cta-create-short"
      onclick={() => vnextSession.goCreate()}
    >
      Abrir un video
    </button>

    {#if vnextSession.projects.length > 0}
      <div>
        <p class="mb-2 text-xs font-semibold uppercase text-surface-500">Continuar</p>
        <ul class="flex flex-col gap-2">
          {#each vnextSession.projects as p (p.id)}
            {@const stage = coarseStage(p)}
            <li>
              <button
                type="button"
                class="flex w-full items-center justify-between rounded-xl border border-surface-800 bg-surface-900/50 px-3 py-3 text-left hover:border-vigil-500/40"
                onclick={() => void vnextSession.loadProject(p.id)}
              >
                <span class="min-w-0">
                  <span class="block truncate text-sm font-medium text-white">{p.title}</span>
                  <span class="block truncate text-[11px] text-surface-500"
                    >{fileName(p.sourceMediaPath)}</span
                  >
                </span>
                <span class="shrink-0 text-xs text-vigil-300">{nextActionHuman(stage)} →</span>
              </button>
            </li>
          {/each}
        </ul>
      </div>
    {/if}
  </div>
</div>
