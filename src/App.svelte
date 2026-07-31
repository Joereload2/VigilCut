<script lang="ts">
  /**
   * Solo producto Short (canvas original + 9:16 + marcos verdes).
   * Sin UI de Silencios / B-roll / Biblioteca / Shorts legacy.
   */
  import { onMount } from "svelte";
  import VNextShell from "$lib/vnext/VNextShell.svelte";
  import type { JobProgress } from "$lib/types";
  import * as api from "$lib/utils/tauri";
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";

  onMount(() => {
    // Progress del análisis (clipping) hacia el store vNext
    if (!api.isTauri()) return;
    let unlisten: (() => void) | undefined;
    void import("@tauri-apps/api/event").then(({ listen }) => {
      void listen<JobProgress>("vigilcut://progress", (ev) => {
        const p = ev.payload;
        if (!p) return;
        if (
          vnextSession.pendingAction === "analyze" ||
          vnextSession.pendingAction === "create"
        ) {
          if (typeof p.percent === "number") {
            vnextSession.setAnalysisProgress(p.percent, p.message);
          }
        }
      }).then((fn) => {
        unlisten = fn;
      });
    });
    return () => unlisten?.();
  });
</script>

<div
  class="relative flex h-full max-h-full min-h-0 min-w-0 w-full max-w-full flex-1 flex-col overflow-hidden bg-surface-950 text-surface-100"
  data-shell="vnext-only"
>
  <VNextShell />
</div>
