<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import RecoverableError from "$lib/vnext/components/RecoverableError.svelte";
  import StationDock from "$lib/vnext/components/StationDock.svelte";
  import { buildStationGuide } from "$lib/vnext/presentation/stations";
  import { humanErrorFromJobs } from "$lib/vnext/presentation/errors";

  const guide = $derived(
    buildStationGuide({
      stage: "failed",
      isHub: false,
      hasSource: !!vnextSession.project?.sourceMediaPath,
      candidateCount: vnextSession.candidates.length,
      hasSelection: !!vnextSession.selectedCandidateId,
      jobs: vnextSession.flowJobs,
    }),
  );
  const msg = $derived(
    vnextSession.error ||
      humanErrorFromJobs(
        vnextSession.jobs,
        "No se pudo completar. Revisá el video y reintentá.",
      ),
  );
</script>

<div class="vn-page" data-testid="page-failed">
  <div class="vn-canvas flex items-center justify-center p-4">
    <RecoverableError title="No se pudo crear el Short" message={msg} />
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
