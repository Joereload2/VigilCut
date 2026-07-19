<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import {
    DEFAULT_CLIPPING_OPTIONS,
    MIN_CLIP_SCORE,
    formatTime,
    type ClipCandidate,
    type ClipFraming,
    type ClippingOptions,
    type ClippingRun,
    type FramingMode,
  } from "$lib/types";
  import * as api from "$lib/utils/tauri";
  import { clippingUi } from "$lib/stores/clipping.svelte";

  /** Defaults only — extraction is automatic; this panel is for review. */
  let options = $state<ClippingOptions>({ ...DEFAULT_CLIPPING_OPTIONS });
  let run = $state<ClippingRun | null>(null);
  let filter = $state<"review" | "all" | "approved" | "discarded">("review");
  let busy = $state(false);
  let error = $state<string | null>(null);
  let selectedId = $state<string | null>(null);
  let showEditor = $state(false);
  let showAdvanced = $state(false);

  const mediaDuration = $derived(projectStore.duration || run?.sourceDuration || 1);

  const allPrimary = $derived(
    run
      ? run.candidates.filter(
          (c) => c.isPrimaryVariant && c.score >= MIN_CLIP_SCORE,
        )
      : ([] as ClipCandidate[]),
  );

  const counts = $derived.by(() => {
    const list = allPrimary;
    return {
      total: list.length,
      review: list.filter((c) =>
        ["preselected", "suggested", "modified"].includes(c.status),
      ).length,
      approved: list.filter((c) => c.status === "approved" || c.status === "exported")
        .length,
      discarded: list.filter((c) => c.status === "discarded" || c.status === "rejected")
        .length,
    };
  });

  const visible = $derived.by(() => {
    if (!run) return [] as ClipCandidate[];
    const list = allPrimary;
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

  function variantCount(c: ClipCandidate): number {
    if (!run) return 0;
    return run.candidates.filter(
      (x) => x.variantGroupId === c.variantGroupId && !x.isPrimaryVariant,
    ).length;
  }

  function variantsOf(c: ClipCandidate): ClipCandidate[] {
    if (!run) return [];
    return run.candidates.filter(
      (x) => x.variantGroupId === c.variantGroupId && !x.isPrimaryVariant,
    );
  }

  let openVariants = $state<Record<string, boolean>>({});

  async function promoteVariant(id: string) {
    if (!run) return;
    try {
      run = await api.promoteClipVariant(run.id, id);
      selectedId = id;
      const c = run.candidates.find((x) => x.id === id);
      if (c) clippingUi.play(c);
    } catch (e) {
      error = String(e);
    }
  }

  const selected = $derived(visible.find((c) => c.id === selectedId) ?? visible[0] ?? null);

  // Sync list selection → player (do not clobber live pan on same clip)
  $effect(() => {
    const s = selected;
    if (!s) {
      clippingUi.select(null);
      return;
    }
    const cur = clippingUi.selected;
    if (!cur || cur.id !== s.id) {
      clippingUi.select(s);
      return;
    }
    // Same clip: refresh metadata; keep manual framing while dragging/saving
    if (cur.framing.mode === "manual") {
      clippingUi.select({ ...s, framing: cur.framing });
    } else {
      clippingUi.select(s);
    }
  });

  // ShortPlayer pan → persist framing on the run
  $effect(() => {
    clippingUi.setFramingSaver(async (clipId, framing) => {
      if (!run) return;
      try {
        const updated = await api.updateClipFraming(run.id, clipId, framing);
        run = {
          ...run,
          candidates: run.candidates.map((c) => (c.id === clipId ? updated : c)),
        };
      } catch (e) {
        console.warn("framing save", e);
      }
    });
    return () => clippingUi.setFramingSaver(null);
  });

  /** One-shot: factory extracts clips; human only classifies & watches. */
  async function extractClips() {
    if (!projectStore.mediaPath || projectStore.mediaPath.startsWith("demo://")) {
      error = "Abre un video real primero";
      return;
    }
    if (!api.isTauri()) {
      error = "Sacar clips requiere la app de escritorio";
      return;
    }
    busy = true;
    error = null;
    projectStore.statusMessage = "Sacando clips…";
    try {
      options = {
        ...DEFAULT_CLIPPING_OPTIONS,
        ...options,
        transcriptPath: options.transcriptPath ?? null,
      };
      run = await api.runClipping(projectStore.mediaPath, options);
      filter = "review";
      const first =
        run.candidates.find((c) => c.status === "preselected" && c.isPrimaryVariant) ??
        run.candidates.find((c) => c.isPrimaryVariant) ??
        null;
      selectedId = first?.id ?? null;
      if (first) clippingUi.play(first);
      const n = run.summary.candidatesFound;
      projectStore.statusMessage =
        n > 0
          ? `${n} clips listos · clasifica y mira el 9:16`
          : "No se encontraron clips";
    } catch (e) {
      error = String(e);
      projectStore.statusMessage = "Error al sacar clips";
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
      // Auto-advance to next pending after classify
      if (status === "approved" || status === "rejected" || status === "discarded") {
        const pending = run.candidates.filter(
          (c) =>
            c.isPrimaryVariant &&
            ["preselected", "suggested", "modified"].includes(c.status),
        );
        const next = pending.find((c) => c.id !== id) ?? pending[0] ?? null;
        if (next) {
          selectedId = next.id;
          clippingUi.play(next);
        }
      }
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
    selectedId = c.id;
    clippingUi.play(c);
    projectStore.previewMode = "original";
    projectStore.currentTime = c.start;
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
      clippingUi.play(updated);
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

  function statusLabel(s: string): string {
    switch (s) {
      case "preselected":
      case "suggested":
        return "por revisar";
      case "modified":
        return "ajustado";
      case "approved":
        return "aprobado";
      case "exported":
        return "exportado";
      case "rejected":
      case "discarded":
        return "descartado";
      default:
        return s;
    }
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
      seekPlay(selected);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const idx = visible.findIndex((c) => c.id === selected.id);
      if (idx >= 0 && idx < visible.length - 1) seekPlay(visible[idx + 1]);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      const idx = visible.findIndex((c) => c.id === selected.id);
      if (idx > 0) seekPlay(visible[idx - 1]);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="panel flex min-h-0 flex-col overflow-hidden border-amber-800/30">
  <!-- Header: classify & watch — not a settings form -->
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="flex items-start justify-between gap-2">
      <div>
        <div class="text-sm font-semibold text-surface-100">Clips</div>
        <div class="text-[10px] text-surface-500">Clasifica y mira el 9:16 a la izquierda</div>
      </div>
      {#if run}
        <button
          type="button"
          class="btn-ghost shrink-0 text-[10px]"
          disabled={busy || !projectStore.mediaPath}
          onclick={extractClips}
          title="Volver a extraer del video"
        >
          {busy ? "…" : "↻ Sacar de nuevo"}
        </button>
      {/if}
    </div>

    {#if !run}
      <button
        type="button"
        class="btn-primary mt-3 w-full py-2.5 text-sm font-semibold"
        disabled={busy || !projectStore.mediaPath}
        onclick={extractClips}
      >
        {busy ? "Sacando clips…" : "Sacar clips"}
      </button>
      <p class="mt-2 text-center text-[10px] text-surface-500">
        Solo salen clips con score ≥ {MIN_CLIP_SCORE}. Aquí clasificas y ves el 9:16.
      </p>
      <button
        type="button"
        class="mt-2 w-full text-center text-[10px] text-surface-600 hover:text-surface-400"
        onclick={() => (showAdvanced = !showAdvanced)}
      >
        {showAdvanced ? "Ocultar opciones" : "Opciones (opcional)"}
      </button>
      {#if showAdvanced}
        <div class="mt-2 space-y-2 rounded-lg border border-surface-800 bg-surface-950/60 p-2">
          <div class="grid grid-cols-2 gap-2">
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
              Cantidad
              <select
                class="mt-0.5 w-full rounded-lg border border-surface-700 bg-surface-950 px-2 py-1 text-xs"
                bind:value={options.selectionProfile}
              >
                <option value="conservative">Pocos</option>
                <option value="balanced">Equilibrado</option>
                <option value="broad">Más clips</option>
                <option value="exploratory">Muchos</option>
              </select>
            </label>
          </div>
          <label class="flex items-center gap-2 text-[10px] text-surface-400">
            <input type="checkbox" class="accent-vigil-500" bind:checked={options.preferWhisper} />
            Whisper si no hay subtítulos
          </label>
          <button type="button" class="btn-ghost text-[10px]" onclick={pickSrt}>
            + SRT/VTT {#if options.transcriptPath}<span class="text-keep">✓</span>{/if}
          </button>
        </div>
      {/if}
    {/if}
  </div>

  {#if error}
    <div class="border-b border-cut/30 bg-cut/10 px-3 py-1.5 text-[11px] text-cut">{error}</div>
  {/if}

  {#if run}
    <!-- Classify toolbar -->
    <div class="border-b border-surface-800 px-3 py-2">
      <div class="flex flex-wrap items-center gap-1.5 text-[10px]">
        <button
          type="button"
          class="rounded-full px-2.5 py-1 font-medium transition
            {filter === 'review'
            ? 'bg-amber-500/20 text-amber-100'
            : 'bg-surface-900 text-surface-400 hover:text-surface-200'}"
          onclick={() => (filter = "review")}
        >
          Por revisar {counts.review}
        </button>
        <button
          type="button"
          class="rounded-full px-2.5 py-1 font-medium transition
            {filter === 'approved'
            ? 'bg-keep/20 text-keep'
            : 'bg-surface-900 text-surface-400 hover:text-surface-200'}"
          onclick={() => (filter = "approved")}
        >
          Aprobados {counts.approved}
        </button>
        <button
          type="button"
          class="rounded-full px-2.5 py-1 font-medium transition
            {filter === 'discarded'
            ? 'bg-cut/20 text-cut'
            : 'bg-surface-900 text-surface-400 hover:text-surface-200'}"
          onclick={() => (filter = "discarded")}
        >
          Descartados {counts.discarded}
        </button>
        <button
          type="button"
          class="rounded-full px-2.5 py-1 font-medium transition
            {filter === 'all'
            ? 'bg-surface-700 text-white'
            : 'bg-surface-900 text-surface-400 hover:text-surface-200'}"
          onclick={() => (filter = "all")}
        >
          Todos {counts.total}
        </button>
      </div>
      <div class="mt-2 flex flex-wrap gap-1">
        <button
          type="button"
          class="btn-ghost text-[10px] text-keep"
          onclick={() => bulk("approved", true)}>Aprobar buenos</button
        >
        <button
          type="button"
          class="btn-secondary text-[10px]"
          disabled={busy || counts.approved + counts.review === 0}
          onclick={exportApproved}>Exportar 9:16</button
        >
      </div>
      <p class="mt-1.5 text-[9px] text-surface-600">
        Clic = ver · <kbd class="text-surface-500">A</kbd> aprobar ·
        <kbd class="text-surface-500">R</kbd> rechazar · ↑↓
      </p>
      {#if run.summary.warnings.length}
        <p class="mt-1 text-[10px] text-warning/90">{run.summary.warnings[0]}</p>
      {/if}
    </div>

    {#if selected && showEditor}
      <div class="border-b border-surface-800 bg-surface-950/80 px-3 py-2">
        <div class="mb-1 flex items-center justify-between">
          <span class="text-[11px] font-semibold text-surface-200">Ajustar · {selected.title}</span>
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (showEditor = false)}
            >Cerrar</button
          >
        </div>
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
          <button type="button" class="btn-ghost text-[10px]" onclick={markIn}>I inicio</button>
          <button type="button" class="btn-ghost text-[10px]" onclick={markOut}>O final</button>
          <button type="button" class="btn-ghost text-[10px]" onclick={restoreSpan}>Restaurar</button>
          <button type="button" class="btn-ghost text-[10px]" onclick={() => setFramingMode("auto_center")}
            >Centro</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => setFramingMode("fit_with_bars")}
            >Fit</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => nudgeFraming(-0.05, 0)}
            >←</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => nudgeFraming(0.05, 0)}
            >→</button
          >
        </div>
      </div>
    {/if}

    <!-- Classification list -->
    <div class="min-h-0 flex-1 overflow-y-auto p-2">
      {#if visible.length === 0}
        <p class="p-4 text-center text-xs text-surface-500">
          {filter === "review"
            ? "Nada pendiente. Cambia de filtro o exporta."
            : "No hay clips en este filtro."}
        </p>
      {:else}
        <ul class="space-y-2">
          {#each visible as c (c.id)}
            <li
              class="rounded-xl border p-2.5 transition
                {selected?.id === c.id
                ? 'border-amber-500/50 bg-amber-950/25 ring-1 ring-amber-500/20'
                : 'border-surface-800 bg-surface-950/50 hover:border-surface-700'}"
            >
              <button type="button" class="w-full text-left" onclick={() => seekPlay(c)}>
                <div class="flex items-start justify-between gap-2">
                  <div class="min-w-0">
                    <div class="truncate text-xs font-semibold text-surface-100">{c.title}</div>
                    <div class="mt-0.5 font-mono text-[10px] text-surface-500">
                      {formatTime(c.start)}–{formatTime(c.end)} · {formatTime(c.duration)}
                      {#if variantCount(c) > 0}
                        <span class="text-surface-600"> · +{variantCount(c)} var.</span>
                      {/if}
                    </div>
                  </div>
                  <div class="text-right">
                    <div class="font-mono text-sm font-bold {scoreColor(c.score)}">
                      {Math.round(c.score)}
                    </div>
                    <div class="text-[9px] text-surface-500">{statusLabel(c.status)}</div>
                  </div>
                </div>
                {#if c.summary}
                  <p class="mt-1 line-clamp-2 text-[11px] text-surface-400">{c.summary}</p>
                {/if}
              </button>

              <!-- Classify actions — main job of this panel -->
              <div class="mt-2 grid grid-cols-2 gap-1.5">
                <button
                  type="button"
                  class="rounded-lg border border-keep/40 bg-keep/10 py-1.5 text-[11px] font-semibold text-keep hover:bg-keep/20"
                  onclick={() => setStatus(c.id, "approved")}
                >
                  ✓ Aprobar
                </button>
                <button
                  type="button"
                  class="rounded-lg border border-cut/40 bg-cut/10 py-1.5 text-[11px] font-semibold text-cut hover:bg-cut/20"
                  onclick={() => setStatus(c.id, "rejected")}
                >
                  ✕ Descartar
                </button>
              </div>
              <div class="mt-1.5 flex flex-wrap gap-1">
                <button type="button" class="btn-ghost text-[10px]" onclick={() => seekPlay(c)}
                  >▶ Ver</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px]"
                  onclick={() => {
                    selectedId = c.id;
                    showEditor = true;
                  }}>Ajustar</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-vigil-300"
                  disabled={busy}
                  onclick={() => exportOne(c)}>Export este</button
                >
                {#if variantCount(c) > 0}
                  <button
                    type="button"
                    class="btn-ghost text-[10px]"
                    onclick={() =>
                      (openVariants = {
                        ...openVariants,
                        [c.id]: !openVariants[c.id],
                      })}
                  >
                    {openVariants[c.id] ? "Ocultar" : "Variantes"}
                  </button>
                {/if}
              </div>
              {#if openVariants[c.id]}
                <ul class="mt-2 space-y-1 border-t border-surface-800 pt-2">
                  {#each variantsOf(c) as v (v.id)}
                    <li
                      class="flex items-center justify-between gap-2 rounded-lg bg-surface-900/80 px-2 py-1.5 text-[10px]"
                    >
                      <span class="min-w-0 truncate text-surface-400">
                        {formatTime(v.start)}–{formatTime(v.end)} · {Math.round(v.score)}
                      </span>
                      <span class="flex shrink-0 gap-1">
                        <button type="button" class="btn-ghost text-[10px]" onclick={() => seekPlay(v)}
                          >▶</button
                        >
                        <button
                          type="button"
                          class="btn-ghost text-[10px] text-vigil-300"
                          onclick={() => promoteVariant(v.id)}>Usar</button
                        >
                      </span>
                    </li>
                  {/each}
                </ul>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {:else if !busy}
    <div class="flex flex-1 flex-col items-center justify-center gap-2 p-6 text-center">
      <div class="text-2xl opacity-50">📱</div>
      <p class="text-xs text-surface-400">
        Pulsa <strong class="text-surface-200">Sacar clips</strong>. Luego clasificas y ves cada uno
        en 9:16.
      </p>
    </div>
  {:else}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center">
      <span class="h-3 w-3 animate-pulse rounded-full bg-amber-400"></span>
      <p class="text-xs text-surface-300">Sacando clips del video…</p>
      <p class="text-[10px] text-surface-600">Puede tardar un poco en vídeos largos</p>
    </div>
  {/if}
</div>
