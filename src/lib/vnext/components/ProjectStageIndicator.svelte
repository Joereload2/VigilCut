<script lang="ts">
  import { projectStageToHumanLabel } from "$lib/vnext/presentation/jobLabels";
  import type { ProjectStage } from "$lib/vnext/types/flow";

  let { stage }: { stage: ProjectStage } = $props();

  const tone = $derived.by(() => {
    switch (stage) {
      case "completed":
        return "border-keep/40 bg-keep/10 text-keep";
      case "failed":
        return "border-cut/40 bg-cut/10 text-cut";
      case "interrupted":
        return "border-warning/40 bg-warning/10 text-warning";
      case "processing":
      case "rendering":
        return "border-sky-500/40 bg-sky-950/40 text-sky-200";
      default:
        return "border-surface-700 bg-surface-900 text-surface-300";
    }
  });
</script>

<span
  class="inline-flex items-center rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}"
  data-testid="stage-indicator"
  data-stage={stage}
>
  {projectStageToHumanLabel(stage)}
</span>
