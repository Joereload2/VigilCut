<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import * as api from "$lib/utils/tauri";

  async function pickVideo() {
    if (!api.isTauri()) {
      vnextSession.error = "Abrí la aplicación de escritorio para elegir un video.";
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm", "m4v"] }],
      });
      if (typeof p === "string") {
        vnextSession.draftPath = p;
        vnextSession.draftTitle =
          p.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") ?? "Short";
      }
    } catch (e) {
      vnextSession.error = String(e);
    }
  }

  function start() {
    if (!vnextSession.draftPath || vnextSession.pendingAction) return;
    void vnextSession.createAndAnalyze(vnextSession.draftPath, vnextSession.draftTitle);
  }
</script>

<div class="flex h-full flex-col items-center justify-center gap-6 px-4" data-testid="page-create">
  <div class="w-full max-w-md rounded-2xl border border-surface-800 bg-surface-900/50 p-6">
    <h1 class="text-xl font-bold text-white">Video del cliente</h1>
    <p class="mt-2 text-sm text-surface-400">
      VigilCut analizará el video y propondrá fragmentos para crear un Short vertical.
    </p>

    {#if vnextSession.draftPath}
      <p class="mt-4 break-all font-mono text-sm text-surface-200">
        {vnextSession.draftPath.split(/[/\\]/).pop()}
      </p>
      <button type="button" class="mt-2 text-xs text-surface-500 underline" onclick={() => void pickVideo()}
        >Cambiar</button
      >
    {:else}
      <button
        type="button"
        class="mt-5 w-full rounded-xl border-2 border-dashed border-vigil-600/50 py-10 text-sm font-semibold text-white hover:border-vigil-500"
        onclick={() => void pickVideo()}
        data-testid="dropzone-pick"
      >Elegir video…</button>
    {/if}

    <button
      type="button"
      class="mt-5 w-full rounded-xl bg-vigil-600 py-3 text-sm font-semibold text-white disabled:opacity-40"
      data-testid="cta-analyze"
      disabled={!vnextSession.draftPath || !!vnextSession.pendingAction}
      onclick={start}
    >
      {vnextSession.pendingAction ? "Analizando…" : "Analizar video"}
    </button>
  </div>
  <button type="button" class="text-xs text-surface-500 underline" onclick={() => vnextSession.goProjects()}
    >← Inicio</button
  >
</div>
