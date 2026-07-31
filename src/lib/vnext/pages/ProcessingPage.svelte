<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import JobProgressView from "$lib/vnext/components/JobProgressView.svelte";
  import {
    jobsToProcessingStep,
    processingStepLabel,
  } from "$lib/vnext/presentation/jobLabels";

  const steps = [0, 1, 2, 3].map((i) => processingStepLabel(i as 0 | 1 | 2 | 3));
  const analyzing = $derived(
    vnextSession.pendingAction === "analyze" || vnextSession.pendingAction === "create",
  );
  const active = $derived(
    analyzing ? stepFromPct(vnextSession.analysisProgressPct) : jobsToProcessingStep(vnextSession.flowJobs),
  );

  function stepFromPct(pct: number): 0 | 1 | 2 | 3 {
    if (pct >= 90) return 3;
    if (pct >= 55) return 2;
    if (pct >= 20) return 1;
    return 0;
  }

  const displayJob = $derived({
    id: "local",
    kind: "clipping_run",
    status: "running" as const,
    progressPct: Math.max(1, Math.min(99, vnextSession.analysisProgressPct || 1)),
  });

  $effect(() => {
    if (!analyzing) return;
    const id = setInterval(() => {
      if (vnextSession.analysisProgressPct < 88) {
        vnextSession.setAnalysisProgress(vnextSession.analysisProgressPct + 0.8);
      }
    }, 1200);
    return () => clearInterval(id);
  });
</script>

<div class="flex h-full flex-col items-center justify-center gap-4 px-4" data-testid="page-processing">
  <div class="w-full max-w-md">
    <JobProgressView title="Analizando video" job={displayJob} {steps} activeStep={active} />
  </div>
  <p class="text-center text-sm text-surface-400">
    Buscando momentos. Después: 1) elegís tramo y duración · 2) marcos verdes para el 9:16.
  </p>
  <p class="text-[11px] text-surface-600">
    {Math.round(vnextSession.analysisProgressPct || 0)}% · no cerrés la app
  </p>
</div>
