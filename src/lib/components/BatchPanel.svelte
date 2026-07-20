<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import * as api from "$lib/utils/tauri";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  interface BatchFileResult {
    mediaPath: string;
    ok: boolean;
    outputPath?: string | null;
    autoCuts: number;
    exceptionsPending: number;
    exceptionsForced: number;
    sourceDuration: number;
    outputDuration: number;
    error?: string | null;
  }

  interface BatchJob {
    id: string;
    mediaPaths: string[];
    outputDir: string;
    status: string;
    progress: number;
    currentFile?: string | null;
    completed: number;
    failed: number;
    errors: string[];
    results: BatchFileResult[];
    autoAcceptExceptions: boolean;
  }

  let job = $state<BatchJob | null>(null);
  let unsubs: UnlistenFn[] = [];
  let panelOpen = $state(true);
  let watchRunning = $state(false);
  let watchMsg = $state("");
  let packs = $state<{ id: string; name: string }[]>([]);
  let policyId = $state("factory");
  /** safe = default (keep doubts); supervised = no export if pending; aggressive = force-cut */
  let exceptionMode = $state<"safe" | "supervised" | "aggressive">("safe");

  onMount(() => {
    if (!api.isTauri()) return;
    void (async () => {
      try {
        const st = await api.getInboxWatchStatus();
        watchRunning = st.running;
      } catch {
        /* ignore */
      }
      try {
        packs = (await api.listPolicyPacks()).map((p) => ({ id: p.id, name: p.name }));
      } catch {
        packs = [{ id: "factory", name: "Factory default" }];
      }
      unsubs.push(
        await listen<BatchJob>("batch://progress", (e) => {
          job = e.payload;
        }),
      );
      unsubs.push(
        await listen<BatchJob>("batch://done", (e) => {
          job = e.payload;
          projectStore.statusMessage = `Lote terminado · ${e.payload.completed} ok · ${e.payload.failed} fallos`;
        }),
      );
      unsubs.push(
        await listen<{ path: string }>("watch://processing", (e) => {
          watchMsg = `Procesando ${e.payload.path.split(/[/\\]/).pop()}`;
        }),
      );
      unsubs.push(
        await listen("watch://done", () => {
          watchMsg = "Archivo de inbox listo en outbox";
        }),
      );
    })();
  });

  onDestroy(() => {
    unsubs.forEach((u) => u());
  });

  async function pickFilesAndRun() {
    if (!api.isTauri()) {
      projectStore.statusMessage = "Batch solo en app desktop";
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        multiple: true,
        filters: [
          {
            name: "Video",
            extensions: ["mp4", "mov", "mkv", "webm", "m4v", "avi"],
          },
        ],
      });
      if (!selected) return;
      const paths = Array.isArray(selected) ? selected : [selected];
      const out = await open({
        directory: true,
        multiple: false,
        title: "Carpeta de salida (outbox)",
      });
      if (!out || typeof out !== "string") return;

      if (exceptionMode === "aggressive") {
        const { ask } = await import("@tauri-apps/plugin-dialog");
        const ok = await ask(
          "Modo AGRESIVO: las dudas de la IA se cortarán automáticamente. Puede eliminar material dudoso. ¿Continuar?",
          { title: "Confirmar modo agresivo", kind: "warning" },
        );
        if (!ok) return;
      }
      const started = await api.queueBatchJob(paths, out, exceptionMode, policyId);
      job = started as BatchJob;
      panelOpen = true;
      projectStore.statusMessage = `Lote ${paths.length} archivos · modo ${exceptionMode}…`;
    } catch (e) {
      projectStore.error = String(e);
    }
  }

  async function runInbox() {
    if (!api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const inbox = await open({
        directory: true,
        multiple: false,
        title: "Carpeta inbox (vídeos crudos)",
      });
      if (!inbox || typeof inbox !== "string") return;
      if (exceptionMode === "aggressive") {
        const { ask } = await import("@tauri-apps/plugin-dialog");
        const ok = await ask(
          "Modo AGRESIVO en inbox: se cortarán excepciones dudosas. ¿Continuar?",
          { title: "Confirmar modo agresivo", kind: "warning" },
        );
        if (!ok) return;
      }
      const started = await api.queueInboxBatch(inbox, null, exceptionMode);
      job = started as BatchJob;
      panelOpen = true;
      projectStore.statusMessage = `Inbox → outbox · modo ${exceptionMode}`;
    } catch (e) {
      projectStore.error = String(e);
    }
  }

  const pct = $derived(Math.round((job?.progress ?? 0) * 100));
</script>

<div class="panel overflow-hidden">
  <button
    type="button"
    class="flex w-full items-center justify-between px-3 py-2 text-left hover:bg-surface-800/40"
    onclick={() => (panelOpen = !panelOpen)}
  >
    <div>
      <div class="text-sm font-semibold text-surface-100">Fábrica · Lote</div>
      <div class="text-[10px] text-surface-500">Inbox → análisis → export · modo seguro por defecto</div>
    </div>
    <span class="text-surface-500">{panelOpen ? "▾" : "▸"}</span>
  </button>

  {#if panelOpen}
    <div class="space-y-2 border-t border-surface-800 p-3">
      <label class="block text-[11px] text-surface-400">
        Cómo tratar dudas (excepciones)
        <select
          class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-900 px-2 py-1.5 text-xs text-surface-100"
          bind:value={exceptionMode}
        >
          <option value="safe">Seguro — conserva material dudoso (recomendado)</option>
          <option value="supervised">Supervisado — no exporta si hay dudas</option>
          <option value="aggressive">Agresivo — corta dudas (pide confirmación)</option>
        </select>
      </label>
      {#if exceptionMode === "aggressive"}
        <p class="rounded-lg border border-warning/40 bg-warning/10 px-2 py-1.5 text-[10px] text-warning">
          Puede eliminar contenido dudoso. Solo para lotes en los que confías en el umbral.
        </p>
      {:else if exceptionMode === "safe"}
        <p class="text-[10px] text-surface-500">
          Los cortes claros se aplican; las dudas se mantienen en el vídeo exportado.
        </p>
      {/if}
      <div class="flex flex-wrap gap-2">
        <button type="button" class="btn-primary text-xs" onclick={pickFilesAndRun}>
          Procesar archivos…
        </button>
        <button
          type="button"
          class="btn-secondary text-xs"
          onclick={async () => {
            try {
              const started = await api.processFactoryInboxNow();
              job = started as BatchJob;
              projectStore.statusMessage = "Procesando inbox de fábrica…";
            } catch (e) {
              projectStore.error = String(e);
            }
          }}
        >
          Procesar inbox ahora
        </button>
        <button type="button" class="btn-secondary text-xs" onclick={runInbox}>
          Otra carpeta…
        </button>
        <button
          type="button"
          class="btn-ghost text-xs"
          onclick={async () => {
            try {
              if (watchRunning) {
                await api.stopInboxWatch();
                watchRunning = false;
                watchMsg = "Watch detenido";
              } else {
                const st = await api.startInboxWatch();
                watchRunning = st.running;
                watchMsg = "Watch activo: deja vídeos en inbox";
              }
            } catch (e) {
              projectStore.error = String(e);
            }
          }}
        >
          {watchRunning ? "■ Parar watch" : "▶ Watch inbox"}
        </button>
        <button
          type="button"
          class="btn-ghost text-xs"
          onclick={async () => {
            try {
              const dir = await api.openFactoryFolder("outbox");
              const { open } = await import("@tauri-apps/plugin-shell");
              await open(dir);
            } catch (e) {
              console.error(e);
            }
          }}
        >
          Ver outbox
        </button>
      </div>

      {#if packs.length}
        <label class="block text-[11px] text-surface-400">
          Policy pack
          <select
            class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-950 px-2 py-1.5 text-xs text-surface-100"
            bind:value={policyId}
          >
            {#each packs as p}
              <option value={p.id}>{p.name}</option>
            {/each}
          </select>
        </label>
      {/if}

      {#if watchMsg}
        <p class="text-[10px] text-vigil-400">{watchMsg}</p>
      {/if}

      {#if job}
        <div class="rounded-lg bg-surface-950 p-2 text-[11px]">
          <div class="flex items-center justify-between gap-2">
            <span class="font-semibold capitalize text-surface-200">{job.status}</span>
            <span class="font-mono text-surface-400">{pct}%</span>
          </div>
          <div class="mt-1 h-1.5 overflow-hidden rounded-full bg-surface-800">
            <div class="h-full bg-vigil-500 transition-all" style:width="{pct}%"></div>
          </div>
          {#if job.currentFile}
            <p class="mt-1 truncate text-surface-500" title={job.currentFile}>
              {job.currentFile.split(/[/\\]/).pop()}
            </p>
          {/if}
          <p class="mt-1 text-surface-400">
            {job.completed} ok · {job.failed} fallos · out: {job.outputDir}
          </p>
        </div>

        {#if job.results?.length}
          <ul class="max-h-32 space-y-1 overflow-y-auto text-[10px]">
            {#each job.results as r}
              <li
                class="truncate rounded px-1.5 py-1 {r.ok
                  ? 'bg-keep/10 text-keep'
                  : 'bg-cut/10 text-cut'}"
                title={r.error ?? r.outputPath ?? ""}
              >
                {#if r.ok}
                  ✓ {r.mediaPath.split(/[/\\]/).pop()}
                  · −{formatTime(r.sourceDuration - r.outputDuration)}
                  · auto {r.autoCuts}
                  {#if r.exceptionsForced}
                    · forzadas {r.exceptionsForced}
                  {/if}
                {:else}
                  ✕ {r.mediaPath.split(/[/\\]/).pop()} — {r.error}
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {:else}
        <p class="text-[11px] text-surface-500">
          El lote auto-acepta excepciones (modo fábrica). Cada archivo genera MP4 + JSON de manifiesto.
        </p>
      {/if}
    </div>
  {/if}
</div>
