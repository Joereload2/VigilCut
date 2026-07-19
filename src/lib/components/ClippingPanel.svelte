<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import {
    DEFAULT_CLIPPING_OPTIONS,
    formatTime,
    type ClipCandidate,
    type ClipFraming,
    type ClippingOptions,
    type ClippingRun,
    type FramingMode,
  } from "$lib/types";
  import * as api from "$lib/utils/tauri";
  import VerticalClipPreview from "$lib/components/VerticalClipPreview.svelte";

  let options = $state<ClippingOptions>({ ...DEFAULT_CLIPPING_OPTIONS });
  let run = $state<ClippingRun | null>(null);
  let filter = $state<"review" | "all" | "approved" | "discarded">("review");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let selectedId = $state<string | null>(null);
  let showEditor = $state(false);

  const mediaDuration = $derived(projectStore.duration || run?.sourceDuration || 1);

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
      filter = "review";
      selectedId =
        run.candidates.find((c) => c.status === "preselected")?.id ??
        run.candidates.find((c) => c.isPrimaryVariant)?.id ??
        null;
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
    projectStore.previewMode = "original";
    window.dispatchEvent(
      new CustomEvent("vigilcut:play-from", {
        detail: { t: c.start, end: c.end, play: true },
      }),
    );
  }

  async function exportOne(c: ClipCandidate) {
    if (!run || !projectStore.mediaPath || !api.isTauri()) return;
    busy = true;
    error = null;
    try {
      const parts = projectStore.mediaPath.split(/[/\\]/);
      parts.pop();
      const dir = parts.join(projectStore.mediaPath.includes("\\") ? "\\" : "/") || ".";
      const stem =
        projectStore.mediaPath.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") ?? "video";
      const sep = projectStore.mediaPath.includes("\\") ? "\\" : "/";
      const outDir = `${dir}${sep}${stem}-clips`;
      const slug = c.title
        .toLowerCase()
        .replace(/[^a-z0-9]+/gi, "-")
        .replace(/^-|-$/g, "")
        .slice(0, 40) || "clip";
      const outPath = `${outDir}${sep}clips${sep}solo_${slug}.mp4`;
      // Ensure approved for single export path via export_clips with id
      if (c.status !== "approved" && c.status !== "preselected") {
        await setStatus(c.id, "approved");
      }
      const res = await api.exportClips({
        runId: run.id,
        outputDir: outDir,
        candidateIds: [c.id],
      });
      run = res.run;
      projectStore.statusMessage = `Clip exportado → ${outDir}`;
      void outPath;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function applySpan(start: number, end: number) {
    if (!run || !selected) return;
    try {
      const updated = await api.updateClipSpan(run.id, selected.id, start, end);
      run = {
        ...run,
        candidates: run.candidates.map((c) => (c.id === selected.id ? updated : c)),
      };
    } catch (e) {
      error = String(e);
    }
  }

  async function restoreSpan() {
    if (!selected) return;
    await applySpan(selected.originalStart, selected.originalEnd);
  }

  async function setFramingMode(mode: FramingMode) {
    if (!run || !selected) return;
    const framing: ClipFraming = { ...selected.framing, mode };
    try {
      const updated = await api.updateClipFraming(run.id, selected.id, framing);
      run = {
        ...run,
        candidates: run.candidates.map((c) => (c.id === selected.id ? updated : c)),
      };
    } catch (e) {
      error = String(e);
    }
  }

  async function nudgeFraming(dx: number, dy: number) {
    if (!run || !selected) return;
    const framing: ClipFraming = {
      ...selected.framing,
      mode: "manual",
      centerX: Math.min(0.95, Math.max(0.05, selected.framing.centerX + dx)),
      centerY: Math.min(0.95, Math.max(0.05, selected.framing.centerY + dy)),
    };
    try {
      const updated = await api.updateClipFraming(run.id, selected.id, framing);
      run = {
        ...run,
        candidates: run.candidates.map((c) => (c.id === selected.id ? updated : c)),
      };
    } catch (e) {
      error = String(e);
    }
  }

  function markIn() {
    if (!selected) return;
    const t = projectStore.currentTime;
    const end = Math.max(t + 0.5, selected.end);
    void applySpan(t, end);
  }

  function markOut() {
    if (!selected) return;
    const t = projectStore.currentTime;
    const start = Math.min(t - 0.5, selected.start);
    void applySpan(Math.max(0, start), Math.max(start + 0.5, t));
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
    } else if (e.key === "i" || e.key === "I") {
      e.preventDefault();
      markIn();
    } else if (e.key === "o" || e.key === "O") {
      e.preventDefault();
      markOut();
    } else if (e.key === "e" || e.key === "E") {
      e.preventDefault();
      showEditor = !showEditor;
    } else if (e.key === "Enter") {
      e.preventDefault();
      showEditor = true;
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
    <div class="text-sm font-semibold text-surface-100">Clipping inteligente</div>
    <div class="text-[10px] text-surface-500">Candidatos · puntuación · 9:16 · supervisión</div>

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

    <label class="mt-2 flex items-center gap-2 text-[10px] text-surface-400">
      <input type="checkbox" class="accent-vigil-500" bind:checked={options.preferWhisper} />
      Intentar Whisper si no hay SRT
    </label>

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
        <span class="truncate text-[9px] text-surface-500" title={options.transcriptPath}>SRT</span>
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
        <span>Pre <strong class="text-keep">{run.summary.preselected}</strong></span>
        <span
          >Mejor <strong class={scoreColor(run.summary.bestScore)}
            >{Math.round(run.summary.bestScore)}</strong
          ></span
        >
        <span class="text-surface-600">{run.summary.transcriptSource}</span>
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
          class="btn-secondary text-[10px]"
          disabled={busy}
          onclick={exportApproved}>Exportar 9:16</button
        >
      </div>
      <p class="mt-1 text-[9px] text-surface-600">
        A/R · I/O límites · E editor · ↑↓ · play
      </p>
    </div>

    {#if selected && showEditor}
      <div class="border-b border-surface-800 bg-surface-950/80 px-3 py-2">
        <div class="mb-1 flex items-center justify-between">
          <span class="text-[11px] font-semibold text-surface-200">Editar · {selected.title}</span>
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (showEditor = false)}
            >Cerrar</button
          >
        </div>

        <div class="mb-2 flex justify-center">
          <VerticalClipPreview framing={selected.framing} time={selected.start} />
        </div>
        <p class="mb-2 text-center text-[9px] text-surface-500">
          {selected.framing.outputWidth}×{selected.framing.outputHeight} · {selected.framing.mode}
          · frame en {formatTime(selected.start)}
        </p>

        <label class="block text-[10px] text-surface-400">
          Inicio {formatTime(selected.start)}
          <input
            type="range"
            class="mt-0.5 w-full accent-vigil-500"
            min="0"
            max={mediaDuration}
            step="0.05"
            value={selected.start}
            oninput={(e) => {
              const v = Number((e.currentTarget as HTMLInputElement).value);
              void applySpan(v, Math.max(v + 0.5, selected.end));
            }}
          />
        </label>
        <label class="mt-1 block text-[10px] text-surface-400">
          Final {formatTime(selected.end)} · {formatTime(selected.duration)}
          <input
            type="range"
            class="mt-0.5 w-full accent-vigil-500"
            min="0"
            max={mediaDuration}
            step="0.05"
            value={selected.end}
            oninput={(e) => {
              const v = Number((e.currentTarget as HTMLInputElement).value);
              void applySpan(Math.min(v - 0.5, selected.start), v);
            }}
          />
        </label>

        <div class="mt-2 flex flex-wrap gap-1">
          <button type="button" class="btn-ghost text-[10px]" onclick={markIn}>I · inicio aquí</button>
          <button type="button" class="btn-ghost text-[10px]" onclick={markOut}>O · final aquí</button>
          <button type="button" class="btn-ghost text-[10px]" onclick={restoreSpan}
            >Restaurar</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => seekPlay(selected)}
            >▶ Clip</button
          >
        </div>

        <div class="mt-2 flex flex-wrap gap-1">
          <button
            type="button"
            class="btn-ghost text-[10px]"
            onclick={() => setFramingMode("auto_center")}>Centro</button
          >
          <button
            type="button"
            class="btn-ghost text-[10px]"
            onclick={() => setFramingMode("blurred_background")}>Fondo blur</button
          >
          <button
            type="button"
            class="btn-ghost text-[10px]"
            onclick={() => setFramingMode("fit_with_bars")}>Fit</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => nudgeFraming(-0.05, 0)}
            >←</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => nudgeFraming(0.05, 0)}
            >→</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => nudgeFraming(0, -0.05)}
            >↑</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => nudgeFraming(0, 0.05)}
            >↓</button
          >
        </div>

        {#if selected.strengths.length || selected.risks.length}
          <div class="mt-2 space-y-0.5 text-[10px]">
            {#each selected.strengths.slice(0, 3) as s}
              <div class="text-keep/80">+ {s}</div>
            {/each}
            {#each selected.risks.slice(0, 2) as r}
              <div class="text-warning/80">! {r}</div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

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
              <button
                type="button"
                class="w-full text-left"
                onclick={() => {
                  selectedId = c.id;
                }}
              >
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
              </button>
              <div class="mt-2 flex flex-wrap gap-1">
                <button type="button" class="btn-ghost text-[10px]" onclick={() => seekPlay(c)}
                  >▶</button
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
                <button
                  type="button"
                  class="btn-ghost text-[10px]"
                  onclick={() => {
                    selectedId = c.id;
                    showEditor = true;
                  }}>Editar</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-vigil-300"
                  disabled={busy}
                  onclick={() => exportOne(c)}>Export 9:16</button
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
        La IA propone momentos. Tú apruebas, ajustas límites y exportas en 9:16.
      </p>
      <p class="text-[10px] text-surface-600">
        Mejor con .srt junto al video, importado, o Whisper en PATH.
      </p>
    </div>
  {/if}
</div>
