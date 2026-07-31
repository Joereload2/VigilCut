<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import StationDock from "$lib/vnext/components/StationDock.svelte";
  import { buildStationGuide } from "$lib/vnext/presentation/stations";

  const guide = $derived(
    buildStationGuide({
      stage: "candidates_ready",
      isHub: false,
      hasSource: true,
      candidateCount: 0,
      hasSelection: false,
      jobs: vnextSession.flowJobs,
      emptyCandidates: true,
    }),
  );
</script>

<div class="vn-page" data-testid="page-empty-candidates">
  <div class="vn-canvas flex flex-col items-center justify-center gap-3 px-4 text-center">
    <h1 class="text-xl font-semibold text-white">No encontramos momentos útiles</h1>
    <p class="max-w-md text-sm text-surface-400">
      El análisis terminó sin proponer fragmentos. Reanalizá o elegí otro video.
    </p>
  </div>
  <StationDock
    {guide}
    label="Analizar de nuevo"
    pending={!!vnextSession.pendingAction}
    pendingLabel="Reiniciando…"
    disabled={!vnextSession.project || !!vnextSession.pendingAction}
    secondaryLabel="Proyectos"
    onPrimary={() => void vnextSession.reanalyzeCurrent()}
    onSecondary={() => vnextSession.goProjects()}
  />
</div>
