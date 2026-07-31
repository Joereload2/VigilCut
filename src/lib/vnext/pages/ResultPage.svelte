<script lang="ts">
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import ShortPreview from "$lib/vnext/components/ShortPreview.svelte";
  import RecoverableError from "$lib/vnext/components/RecoverableError.svelte";
  import { formatBytes } from "$lib/vnext/presentation/jobLabels";
  import { isTauri } from "$lib/utils/tauri";

  const art = $derived(vnextSession.finalArtifact);
  const completedNoArt = $derived(
    !art &&
      vnextSession.jobs.some(
        (j) => j.kind === "vertical_render" && j.status === "completed",
      ),
  );
  const fileName = $derived(art?.path.split(/[/\\]/).pop() ?? "");
  const folderPath = $derived(
    art?.path ? art.path.replace(/[/\\][^/\\]+$/, "") || art.path : "",
  );

  let copied = $state(false);

  async function openFile() {
    if (!art?.path || !isTauri()) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("open_path_in_os", { path: art.path });
    } catch (e) {
      vnextSession.error = String(e);
    }
  }

  async function openFolder() {
    if (!art?.path || !isTauri()) return;
    try {
      const parent = folderPath || art.path;
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("open_path_in_os", { path: parent });
    } catch (e) {
      vnextSession.error = String(e);
    }
  }

  async function copyPath() {
    if (!art?.path) return;
    try {
      await navigator.clipboard.writeText(art.path);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      vnextSession.error = "No se pudo copiar la ruta";
    }
  }
</script>

<div class="flex h-full min-h-0 flex-col" data-testid="page-result">
  {#if completedNoArt}
    <div class="flex flex-1 items-center justify-center p-4">
      <RecoverableError
        title="El proceso terminó sin archivo"
        message="No hay un video validado. Eso es un error de integridad."
        onBack={() => vnextSession.goProjects()}
        onRetry={() => void vnextSession.retryFailed()}
      />
    </div>
  {:else if !art}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6">
      <p class="text-sm font-medium text-white">Buscando el Short…</p>
      <p class="max-w-sm text-center text-xs text-surface-500">
        Si tarda mucho, el render puede seguir en segundo plano. Revisá la barra de error arriba.
      </p>
    </div>
  {:else}
    <div
      class="grid min-h-0 flex-1 grid-cols-1 gap-2 overflow-hidden p-2 lg:grid-cols-[minmax(0,1fr)_minmax(16rem,20rem)] lg:items-stretch"
    >
      <!-- Muestra 9:16 del archivo final -->
      <div class="flex min-h-0 flex-col items-center justify-center overflow-hidden">
        <p class="mb-1 shrink-0 text-[9px] font-bold uppercase tracking-wide text-emerald-400">
          Short listo · 9:16
        </p>
        <div class="flex min-h-0 w-full flex-1 items-center justify-center overflow-hidden">
          <ShortPreview
            sourcePath={art.path}
            start={0}
            end={null}
            aspect="9/16"
            loopSegment={false}
            autoplay={false}
            size="full"
            framed={true}
            fit="contain"
            label=""
          />
        </div>
      </div>

      <div class="flex min-h-0 flex-col gap-2 overflow-y-auto">
        <div>
          <h2 class="text-base font-semibold text-white">Video vertical generado</h2>
          <p class="mt-0.5 break-all font-mono text-[11px] text-surface-300" title={art.path}>
            {fileName}
          </p>
        </div>

        <div class="rounded-xl border border-surface-700 bg-surface-900/80 px-4 py-3 text-left">
          <p class="text-[10px] font-bold uppercase text-surface-500">Guardado en</p>
          <p class="mt-1 break-all font-mono text-[11px] leading-relaxed text-surface-200">
            {art.path}
          </p>
          <p class="mt-1 break-all text-[10px] text-surface-500">Carpeta: {folderPath}</p>
          <div class="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              class="rounded-lg bg-vigil-600 px-3 py-2 text-xs font-semibold text-white hover:bg-vigil-500"
              onclick={() => void openFolder()}
            >Abrir carpeta</button>
            <button
              type="button"
              class="rounded-lg border border-surface-600 px-3 py-2 text-xs font-semibold text-surface-200 hover:bg-surface-800"
              onclick={() => void openFile()}
            >Abrir archivo</button>
            <button
              type="button"
              class="rounded-lg border border-surface-600 px-3 py-2 text-xs text-surface-300 hover:bg-surface-800"
              onclick={() => void copyPath()}
            >{copied ? "Ruta copiada ✓" : "Copiar ruta"}</button>
          </div>
        </div>

        <dl class="grid grid-cols-3 gap-2 text-center text-[11px] text-surface-400">
          <div class="rounded-lg border border-surface-800 bg-surface-900/50 p-2">
            <dt>Tamaño</dt>
            <dd class="font-mono text-surface-200">{formatBytes(art.byteSize)}</dd>
          </div>
          <div class="rounded-lg border border-surface-800 bg-surface-900/50 p-2">
            <dt>Validación</dt>
            <dd class="text-emerald-400">{art.validationStatus === "passed" ? "OK" : "Revisar"}</dd>
          </div>
          <div class="rounded-lg border border-surface-800 bg-surface-900/50 p-2">
            <dt>Formato</dt>
            <dd class="text-surface-200">9:16 MP4</dd>
          </div>
        </dl>

        <!-- Volver al flujo del video (momentos + encuadre), sin borrar el short -->
        <button
          type="button"
          class="w-full rounded-xl border border-vigil-500/60 bg-vigil-950/40 py-3 text-sm font-semibold text-vigil-200 hover:bg-vigil-950/70"
          data-testid="cta-full-video"
          onclick={() => vnextSession.openFullVideoFlow()}
        >
          Ir al video completo
        </button>
        <p class="text-center text-[10px] text-surface-500">
          Revisá o elegí otros momentos del mismo video (el short ya guardado no se borra).
        </p>

        <button
          type="button"
          class="text-xs text-surface-400 underline hover:text-white"
          onclick={() => vnextSession.goCreate()}
          data-testid="cta-otro"
        >Abrir otro archivo de video</button>
      </div>
    </div>
  {/if}
</div>
