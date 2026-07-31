<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import JobProgressView from "$lib/vnext/components/JobProgressView.svelte";
  import StationDock from "$lib/vnext/components/StationDock.svelte";
  import { buildStationGuide } from "$lib/vnext/presentation/stations";

  const job = $derived(
    vnextSession.flowJobs.find((j) => j.kind === "vertical_render") ?? null,
  );
  const guide = $derived(
    buildStationGuide({
      stage: "rendering",
      isHub: false,
      hasSource: true,
      candidateCount: 1,
      hasSelection: true,
      jobs: vnextSession.flowJobs,
    }),
  );
</script>

<div class="vn-page" data-testid="page-rendering">
  <div class="vn-canvas-scroll flex flex-col items-center justify-center gap-5 px-4 py-8">
    <div class="w-full max-w-lg">
      <JobProgressView title="Creando tu Short" {job} />
    </div>
    <div class="vn-panel w-full max-w-lg px-5 py-4 text-center">
      <p class="text-sm font-semibold text-white">No hace falta otra acción</p>
      <p class="mt-1 text-xs text-surface-400">
        Generamos el vertical 1080×1920. Al terminar vamos a Resultado.
      </p>
    </div>
    {#if job && (job.status === "queued" || job.status === "running")}
      <button
        type="button"
        class="text-xs text-surface-500 underline disabled:opacity-40"
        disabled={!!vnextSession.pendingAction}
        onclick={() => void vnextSession.cancelActive()}
      >Cancelar</button>
    {/if}
  </div>
  <StationDock {guide} showPrimary={false} hint="Siguiente automático: Resultado" />
</div>
