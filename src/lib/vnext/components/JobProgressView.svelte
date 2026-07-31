<script lang="ts">
  import type { FlowJobView } from "$lib/vnext/types/flow";
  import { jobToUserProgress } from "$lib/vnext/presentation/jobLabels";

  let {
    job = null,
    title = "Progreso",
    steps = null as string[] | null,
    activeStep = 0,
  }: {
    job?: FlowJobView | null;
    title?: string;
    steps?: string[] | null;
    activeStep?: number;
  } = $props();

  const prog = $derived(job ? jobToUserProgress(job) : null);
</script>

<div class="rounded-2xl border border-surface-800 bg-surface-900/50 p-5" data-testid="job-progress">
  <h2 class="text-lg font-semibold text-white">{title}</h2>
  {#if prog}
    <p class="mt-1 text-sm text-surface-300">{prog.label} — {prog.detail}</p>
    {#if prog.percent != null}
      <div class="mt-4 h-2 overflow-hidden rounded-full bg-surface-800">
        <div class="h-full rounded-full bg-vigil-500 transition-all" style="width: {prog.percent}%"></div>
      </div>
      <p class="mt-1 text-right font-mono text-[11px] text-surface-500">{prog.percent.toFixed(0)}%</p>
    {:else if prog.indeterminate}
      <div class="mt-4 h-2 overflow-hidden rounded-full bg-surface-800">
        <div class="h-full w-1/3 animate-pulse rounded-full bg-sky-500/80"></div>
      </div>
      <p class="mt-1 text-[11px] text-surface-500">Trabajando…</p>
    {/if}
  {:else}
    <p class="mt-1 text-sm text-surface-400">Preparando…</p>
    <div class="mt-4 h-2 overflow-hidden rounded-full bg-surface-800">
      <div class="h-full w-1/3 animate-pulse rounded-full bg-sky-500/80"></div>
    </div>
  {/if}

  {#if steps}
    <ol class="mt-5 space-y-2">
      {#each steps as step, i}
        <li class="flex items-center gap-2 text-sm
          {i === activeStep ? 'font-semibold text-white' : i < activeStep ? 'text-keep' : 'text-surface-500'}">
          <span class="flex h-6 w-6 items-center justify-center rounded-full border text-[11px]
            {i === activeStep ? 'border-vigil-400 bg-vigil-950 text-vigil-200' : i < activeStep ? 'border-keep/50 text-keep' : 'border-surface-700'}">
            {i < activeStep ? "✓" : i + 1}
          </span>
          {step}
        </li>
      {/each}
    </ol>
  {/if}
</div>
