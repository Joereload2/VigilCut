<script lang="ts">
  import type { ContentProjectSnapshotV1 } from "$lib/types/vnext/v1";
  import type { ProjectStage } from "$lib/vnext/types/flow";
  import ProjectStageIndicator from "./ProjectStageIndicator.svelte";
  import { nextActionHuman } from "$lib/vnext/presentation/deriveStage";

  let {
    project,
    stage,
    onOpen,
  }: {
    project: ContentProjectSnapshotV1;
    stage: ProjectStage;
    onOpen: () => void;
  } = $props();

  function shortDate(iso: string) {
    try {
      return new Date(iso).toLocaleString(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      });
    } catch {
      return iso;
    }
  }
</script>

<button
  type="button"
  class="flex w-full flex-col gap-2 rounded-2xl border border-surface-800 bg-surface-900/60 p-4 text-left transition hover:border-vigil-500/50 hover:bg-surface-900"
  onclick={onOpen}
  data-testid="project-card"
>
  <div class="flex items-start justify-between gap-2">
    <h3 class="text-base font-semibold text-white">{project.title}</h3>
    <ProjectStageIndicator {stage} />
  </div>
  <p class="truncate text-xs text-surface-500" title={project.sourceMediaPath}>
    {project.sourceMediaPath.split(/[/\\]/).pop()}
  </p>
  <div class="flex items-center justify-between text-[11px] text-surface-400">
    <span>{shortDate(project.updatedAt)}</span>
    <span class="font-medium text-vigil-300">{nextActionHuman(stage)}</span>
  </div>
</button>
