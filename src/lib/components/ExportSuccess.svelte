<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import { isTauri } from "$lib/utils/tauri";

  interface Props {
    onNewVideo: () => void;
  }
  let { onNewVideo }: Props = $props();

  const exp = $derived(projectStore.lastExport);

  async function openFolder() {
    if (!exp?.path || !isTauri()) {
      projectStore.statusMessage = exp?.path ?? "";
      return;
    }
    const path = exp.path;
    const parts = path.split(/[/\\]/);
    parts.pop();
    const dir = parts.join(path.includes("\\") ? "\\" : "/") || path;
    try {
      // Prefer reveal without blocking the UI if the shell plugin stalls
      const { open } = await import("@tauri-apps/plugin-shell");
      void open(dir).catch(() => {
        projectStore.statusMessage = `Carpeta: ${dir}`;
      });
      projectStore.statusMessage = `Carpeta abierta · ${dir}`;
    } catch (e) {
      projectStore.statusMessage = `Archivo: ${path}`;
      console.error(e);
    }
  }

  function close() {
    projectStore.dismissExportSuccess();
  }

  function another() {
    projectStore.dismissExportSuccess();
    projectStore.resetProject();
    onNewVideo();
  }
</script>

{#if projectStore.showExportSuccess && exp}
  <!-- Cover only the work area; leave StatusBar fully visible at the bottom -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="absolute inset-x-0 top-0 bottom-8 z-[60] flex items-center justify-center bg-surface-950/70 p-4 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    aria-labelledby="export-success-title"
    onclick={close}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="w-full max-w-md rounded-2xl border border-surface-700 bg-surface-900 p-6 shadow-2xl"
      onclick={(e) => e.stopPropagation()}
    >
      <div
        class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-vigil-600/20 text-2xl text-vigil-400"
      >
        ✓
      </div>
      <h2 id="export-success-title" class="mt-3 text-center text-lg font-semibold text-white">
        Video exportado
      </h2>
      <p class="mt-1 text-center text-sm text-keep/90">
        Tu video original no fue modificado.
      </p>
      <p class="mt-1 text-center text-sm text-surface-400">
        Junto al original: <span class="text-surface-200">.mp4</span>
        + <span class="text-surface-200">.chapters.txt</span>
        · meta en <span class="text-surface-200">*-meta/</span>
      </p>

      <div class="mt-4 grid grid-cols-2 gap-2 rounded-xl bg-surface-950/80 p-3 text-center text-xs">
        <div>
          <div class="font-mono text-base font-semibold text-keep">{formatTime(exp.keptDuration)}</div>
          <div class="text-surface-500">duración final</div>
        </div>
        <div>
          <div class="font-mono text-base font-semibold text-cut">−{formatTime(exp.cutDuration)}</div>
          <div class="text-surface-500">recortado</div>
        </div>
      </div>

      <p
        class="mt-3 break-all rounded-lg bg-surface-950 px-2 py-1.5 font-mono text-[10px] text-surface-500"
        title={exp.path}
      >
        {exp.path}
      </p>

      <div class="mt-5 flex flex-col gap-2">
        <button type="button" class="btn-primary w-full py-2.5 font-semibold" onclick={openFolder}>
          Abrir carpeta
        </button>
        <button type="button" class="btn-secondary w-full py-2" onclick={another}>
          Nuevo video
        </button>
        <button type="button" class="btn-primary w-full py-2.5 font-semibold" onclick={close}>
          Seguir editando
        </button>
      </div>
    </div>
  </div>
{/if}
