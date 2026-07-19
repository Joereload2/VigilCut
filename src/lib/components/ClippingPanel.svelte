<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import {
    DEFAULT_CLIPPING_OPTIONS,
    formatTime,
    type ClipCandidate,
    type ClippingOptions,
    type ClippingRun,
  } from "$lib/types";
  import * as api from "$lib/utils/tauri";

  let options = $state<ClippingOptions>({ ...DEFAULT_CLIPPING_OPTIONS });
  let run = $state<ClippingRun | null>(null);
  let filter = $state<"review" | "all" | "approved" | "discarded">("review");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let selectedId = $state<string | null>(null);

  const visible = $derived.by(() => {
    if (!run) return [] as ClipCandidate[];
    const list = run.candidates.filter((c) => c.isPrimaryVariant);
    switch (filter) {
      case "review":
        return list.filter((c) =>
          ["preselected", "suggested", "modified"].includes(c.status),
        );
      case "approved":
        return list.filter((c) => c.status === "approved" || c.status === "exported");
      case "discarded":
        return list.filter((c) => c.status === "discarded" || c.status === "rejected");
      default:
        return list;
    }
  });

  const selected = $derived(visible.find((c) => c.id === selectedId) ?? visible[0] ?? null);

  async function analyze() {
    if (!projectStore.mediaPath || projectStore.mediaPath.startsWith("demo://")) {
      error = "Abre un video real primero";
      return;
    }
    if (!api.isTauri()) {
      error = "Clipping requiere la app de escritorio";
      return;
    }
    busy = true;
    error = null;
    projectStore.statusMessage = "Buscando clips…";
    try {
      options = { ...options, transcriptPath: options.transcriptPath ?? null };
      run = await api.runClipping(projectStore.mediaPath, options);
      // If default and file might exist, re-run with srt only if user set path — leave as is
      filter = "review";
      selectedId = run.candidates.find((c) => c.status === "preselected")?.id ?? null;
      projectStore.statusMessage = `Clips: ${run.summary.preselected} preseleccionados / ${run.summary.candidatesFound} detectados`;
    } catch (e) {
      error = String(e);
      projectStore.statusMessage = "Error en clipping";
    } finally {
      busy = false;
    }
  }

  async function pickSrt() {
    if (!api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Subtítulos", extensions: ["srt", "vtt"] }],
      });
      if (typeof p === "string") {
        options = { ...options, transcriptPath: p };
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function setStatus(id: string, status: string) {
    if (!run) return;
    try {
      const updated = await api.updateClipStatus(run.id, id, status);
      run = {
        ...run,
        candidates: run.candidates.map((c) => (c.id === id ? updated : c)),
      };
    } catch (e) {
      error = String(e);
    }
  }

  async function bulk(status: string, highOnly: boolean) {
    if (!run) return;
    try {
      run = await api.bulkClipStatus(run.id, status, highOnly);
    } catch (e) {
      error = String(e);
    }
  }

  function seekPlay(c: ClipCandidate) {
    projectStore.currentTime = c.start;
    projectStore.previewMode = "original";
    window.dispatchEvent(new CustomEvent("vigilcut:play-from", { detail: { t: c.start } }));
  }

  async function exportApproved() {
    if (!run || !projectStore.mediaPath) return;
    busy = true;
    error = null;
    try {
      const parts = projectStore.mediaPath.split(/[/\\]/);
      parts.pop();
      const dir = parts.join(projectStore.mediaPath.includes("\\") ? "\\" : "/") || ".";
      const stem =
        projectStore.mediaPath.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") ?? "video";
      const outDir = `${dir}${projectStore.mediaPath.includes("\\") ? "\\" : "/"}${stem}-clips`;
      const res = await api.exportClips({
        runId: run.id,
        outputDir: outDir,
        candidateIds: run.candidates
          .filter((c) => c.status === "approved" || c.status === "preselected")
          .map((c) => c.id),
      });
      run = res.run;
      projectStore.statusMessage = `Clips exportados → ${res.outputDir}`;
      try {
        const { open } = await import("@tauri-apps/plugin-shell");
        await open(res.outputDir);
      } catch {
        /* ignore */
      }
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function scoreColor(s: number) {
    if (s >= 72) return "text-keep";
    if (s >= 55) return "text-warning";
    return "text-surface-400";
  }

  // Keyboard when panel focused via window when run active
  function onKey(e: KeyboardEvent) {
    if (!run || !selected) return;
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    if (e.key === "a" || e.key === "A") {
      e.preventDefault();
      void setStatus(selected.id, "approved");
    } else if (e.key === "r" || e.key === "R") {
      e.preventDefault();
      void setStatus(selected.id, "rejected");
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const idx = visible.findIndex((c) => c.id === selected.id);
      if (idx >= 0 && idx < visible.length - 1) selectedId = visible[idx + 1].id;
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const idx = visible.findIndex((c) => c.id === selected.id);
      if (idx > 0) selectedId = visible[idx - 1].id;
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="panel flex min-h-0 flex-col overflow-hidden border-vigil-800/40">
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="flex items-center justify-between gap-2">
      <div>
        <div class="text-sm font-semibold text-surface-100">Clipping inteligente</div>
        <div class="text-[10px] text-surface-500">
          Candidatos · puntuación · 9:16 · supervisión rápida
        </div>
      </div>
    </div>

    <div class="mt-2 grid grid-cols-2 gap-2">
      <label class="text-[10px] text-surface-400">
        Duración
        <select
          class="mt-0.5 w-full rounded-lg border border-surface-700 bg-surface-950 px-2 py-1 text-xs"
          bind:value={options.durationProfile}
        >
          <option value="micro">Micro 10–20s</option>
          <option value="short">Corto 20–40s</option>
          <option value="standard">Estándar 40–60s</option>
          <option value="extended">Extendido 60–90s</option>
        </select>
      </label>
      <label class="text-[10px] text-surface-400">
        Selección
        <select
          class="mt-0.5 w-full rounded-lg border border-surface-700 bg-surface-950 px-2 py-1 text-xs"
          bind:value={options.selectionProfile}
        >
          <option value="conservative">Conservador</option>
          <option value="balanced">Equilibrado</option>
          <option value="broad">Amplio</option>
          <option value="exploratory">Exploratorio</option>
        </select>
      </label>
    </div>

    <div class="mt-2 flex flex-wrap gap-1.5">
      <button
        type="button"
        class="btn-primary text-xs"
        disabled={busy || !projectStore.mediaPath}
        onclick={analyze}
      >
        {busy ? "Analizando…" : "Encontrar clips"}
      </button>
      <button type="button" class="btn-ghost text-[10px]" onclick={pickSrt}>+ SRT/VTT</button>
      {#if options.transcriptPath}
        <span class="truncate text-[9px] text-surface-500" title={options.transcriptPath}
          >SRT OK</span
        >
      {/if}
    </div>
  </div>

  {#if error}
    <div class="border-b border-cut/30 bg-cut/10 px-3 py-1.5 text-[11px] text-cut">{error}</div>
  {/if}

  {#if run}
    <div class="border-b border-surface-800 px-3 py-2 text-[10px] text-surface-400">
      <div class="flex flex-wrap gap-2">
        <span
          >Candidatos <strong class="text-surface-200">{run.summary.candidatesFound}</strong></span
        >
        <span
          >Preseleccionados <strong class="text-keep">{run.summary.preselected}</strong></span
        >
        <span
          >Alta conf. <strong class="text-vigil-300">{run.summary.highConfidence}</strong></span
        >
        <span
          >Mejor <strong class={scoreColor(run.summary.bestScore)}
            >{Math.round(run.summary.bestScore)}</strong
          ></span
        >
        <span class="text-surface-600">{run.summary.analysisSeconds.toFixed(1)}s</span>
      </div>
      {#if run.summary.warnings.length}
        <p class="mt-1 text-[10px] text-warning/90">{run.summary.warnings[0]}</p>
      {/if}
      <div class="mt-2 flex flex-wrap gap-1">
        <button type="button" class="btn-ghost text-[10px]" onclick={() => (filter = "review")}
          >Revisar</button
        >
        <button type="button" class="btn-ghost text-[10px]" onclick={() => (filter = "all")}
          >Todos</button
        >
        <button type="button" class="btn-ghost text-[10px]" onclick={() => (filter = "approved")}
          >Aprobados</button
        >
        <button
          type="button"
          class="btn-ghost text-[10px] text-keep"
          onclick={() => bulk("approved", true)}>Aprobar alta conf.</button
        >
        <button
          type="button"
          class="btn-ghost text-[10px] text-cut"
          onclick={() => bulk("rejected", false)}>Rechazar visibles*</button
        >
        <button
          type="button"
          class="btn-secondary text-[10px]"
          disabled={busy}
          onclick={exportApproved}>Exportar 9:16</button
        >
      </div>
      <p class="mt-1 text-[9px] text-surface-600">
        Atajos: A aprobar · R rechazar · ↑↓ navegar · clic play
      </p>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto p-2">
      {#if visible.length === 0}
        <p class="p-3 text-center text-xs text-surface-500">No hay candidatos en este filtro.</p>
      {:else}
        <ul class="space-y-2">
          {#each visible as c (c.id)}
            <li
              class="rounded-xl border p-2.5 transition
                {selected?.id === c.id
                ? 'border-vigil-500/50 bg-vigil-950/30'
                : 'border-surface-800 bg-surface-950/50'}"
            >
              <button type="button" class="w-full text-left" onclick={() => (selectedId = c.id)}>
                <div class="flex items-start justify-between gap-2">
                  <div class="min-w-0">
                    <div class="truncate text-xs font-semibold text-surface-100">{c.title}</div>
                    <div class="mt-0.5 font-mono text-[10px] text-surface-500">
                      {formatTime(c.start)}–{formatTime(c.end)} · {formatTime(c.duration)}
                    </div>
                  </div>
                  <div class="text-right">
                    <div class="font-mono text-sm font-bold {scoreColor(c.score)}">
                      {Math.round(c.score)}
                    </div>
                    <div class="text-[9px] uppercase text-surface-500">{c.status}</div>
                  </div>
                </div>
                <p class="mt-1 line-clamp-2 text-[11px] text-surface-400">{c.summary}</p>
                {#if c.strengths[0]}
                  <p class="mt-1 text-[10px] text-keep/80">+ {c.strengths[0]}</p>
                {/if}
                {#if c.risks[0]}
                  <p class="text-[10px] text-warning/80">! {c.risks[0]}</p>
                {/if}
              </button>
              <div class="mt-2 flex flex-wrap gap-1">
                <button
                  type="button"
                  class="btn-ghost text-[10px]"
                  onclick={() => seekPlay(c)}>▶ Oír</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-keep"
                  onclick={() => setStatus(c.id, "approved")}>Aprobar</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-cut"
                  onclick={() => setStatus(c.id, "rejected")}>Rechazar</button
                >
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 p-4 text-center">
      <p class="text-xs text-surface-400">
        La IA propone los mejores momentos. Tú solo apruebas o rechazas.
      </p>
      <p class="text-[10px] text-surface-600">
        Mejor con archivo .srt junto al video o importado.
      </p>
    </div>
  {/if}
</div>
