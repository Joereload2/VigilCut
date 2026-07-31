<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import RecoverableError from "$lib/vnext/components/RecoverableError.svelte";
  import StationDock from "$lib/vnext/components/StationDock.svelte";
  import { buildStationGuide } from "$lib/vnext/presentation/stations";
  import { humanErrorFromJobs } from "$lib/vnext/presentation/errors";

  const guide = $derived(
    buildStationGuide({
      stage: "interrupted",
      isHub: false,
      hasSource: !!vnextSession.project?.sourceMediaPath,
      candidateCount: vnextSession.candidates.length,
      hasSelection: !!vnextSession.selectedCandidateId,
      jobs: vnextSession.flowJobs,
    }),
  );
  const detail = $derived(
    humanErrorFromJobs(
      vnextSession.jobs,
      "El trabajo se detuvo antes de terminar. Podés reintentar.",
    ),
  );
</script>

<div class="vn-page" data-testid="page-interrupted">
  <div class="vn-canvas flex items-center justify-center p-4">
    <RecoverableError title="Proceso interrumpido" message={detail} />
  </div>
  <StationDock
    {guide}
    label="Reintentar"
    pending={!!vnextSession.pendingAction}
    pendingLabel="Reintentando…"
    secondaryLabel="Proyectos"
    onPrimary={() => void vnextSession.retryFailed()}
    onSecondary={() => vnextSession.goProjects()}
  />
</div>
