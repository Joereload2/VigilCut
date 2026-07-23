<script lang="ts">
  import VisualWorkspace from "$lib/components/visual/VisualWorkspace.svelte";
  import * as api from "$lib/utils/tauri";
  import type { VisualsViewId } from "$lib/components/visual/library/libraryTypes";

  let view = $state<VisualsViewId>("library");
  let syncMode = $state("local_only");
  let syncPending = $state(0);

  $effect(() => {
    void api.librarySyncStatus().then((value) => {
      const status = value as { mode?: string; pending?: number };
      syncMode = status.mode ?? "local_only";
      syncPending = status.pending ?? 0;
    });
  });
</script>

<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden p-2">
  <div class="mb-2 flex items-center justify-between rounded-lg border border-surface-800 bg-surface-950/60 px-3 py-2 text-[11px] text-surface-400">
    <span>Sincronización: {syncMode === "local_only" ? "solo local" : syncMode}</span>
    <span>{syncPending} pendiente(s) · SQLite sigue operativa offline</span>
  </div>
  <VisualWorkspace
    bind:view
    libraryOnly
    onViewChange={(next) => (view = next)}
  />
</div>
