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
  let panelOpen = $state(false);

  onMount(() => {
    if (!api.isTauri()) return;
    void (async () => {
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

      const started = await api.queueBatchJob(paths, out, true);
      job = started as BatchJob;
      panelOpen = true;
      projectStore.statusMessage = `Lote ${paths.length} archivos en curso…`;
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
      const started = await api.queueInboxBatch(inbox, null, true);
      job = started as BatchJob;
      panelOpen = true;
      projectStore.statusMessage = `Inbox → outbox · lote en curso`;
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
      <div class="text-[10px] text-surface-500">Inbox → análisis → export (auto-excepciones)</div>
    </div>
    <span class="text-surface-500">{panelOpen ? "▾" : "▸"}</span>
  </button>

  {#if panelOpen}
    <div class="space-y-2 border-t border-surface-800 p-3">
      <div class="flex flex-wrap gap-2">
        <button type="button" class="btn-primary text-xs" onclick={pickFilesAndRun}>
          Procesar archivos…
        </button>
        <button type="button" class="btn-secondary text-xs" onclick={runInbox}>
          Carpeta inbox…
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
