<script lang="ts">
  import type { FlowJobView } from "$lib/vnext/types/flow";
  import { jobToUserProgress, jobTypeToHumanLabel, jobStateToHumanLabel } from "$lib/vnext/presentation/jobLabels";

  let { jobs = [] as FlowJobView[] }: { jobs?: FlowJobView[] } = $props();

  const recent = $derived(jobs.slice(0, 5));
</script>

{#if recent.length > 0}
  <aside class="rounded-xl border border-surface-800/80 bg-surface-950/80 p-3" data-testid="activity-panel">
    <h3 class="text-[10px] font-bold uppercase tracking-wide text-surface-500">Actividad</h3>
    <ul class="mt-2 space-y-2">
      {#each recent as j (j.id)}
        {@const p = jobToUserProgress(j)}
        <li class="text-[11px] text-surface-300">
          <span class="font-medium text-surface-100">{jobTypeToHumanLabel(j.kind)}</span>
          <span class="text-surface-500"> · {jobStateToHumanLabel(j.status)}</span>
          {#if p.percent != null}
            <span class="text-surface-600"> · {p.percent.toFixed(0)}%</span>
          {/if}
        </li>
      {/each}
    </ul>
  </aside>
{/if}
