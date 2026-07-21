<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";
  import VisualPlaceForm from "./visual/VisualPlaceForm.svelte";
  import VisualTrack from "./visual/VisualTrack.svelte";
  import PlacementInspector from "./visual/PlacementInspector.svelte";
  import type {
    Asset,
    DisplayMode,
    ProtectedRange,
    Suggestion,
    VisualPlacement,
    VisualPlan,
  } from "./visual/types";

  type ToolTab = "colocar" | "plan" | "texto" | "mas";

  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastMessage = $state("");
  let tool = $state<ToolTab>("colocar");

  let displayMode = $state<DisplayMode>("completa");
  let outputStart = $state(0);
  let outputEnd = $state(4);
  let selectedPlacementId = $state<string | null>(null);

  let assets = $state<Asset[]>([]);
  let plan = $state<VisualPlan | null>(null);
  let suggestions = $state<Suggestion[]>([]);
  let transcriptSegs = $state<
    { id?: string; text: string; span: { start: number; end: number } }[]
  >([]);
  let whisperOk = $state(false);
  let whisperKind = $state("");
  let preferBusyWhisper = $state(false);

  const duration = $derived(
    projectStore.estimate?.estimatedDuration ??
      projectStore.keptDuration ??
      projectStore.duration ??
      60,
  );
  const playhead = $derived(projectStore.currentTime);
  const placements = $derived(plan?.placements ?? []);
  const protectedRanges = $derived(plan?.protectedRanges ?? []);
  const selectedPlacement = $derived(
    placements.find((p) => p.id === selectedPlacementId) ?? null,
  );
  const selectedThumb = $derived.by(() => {
    if (!selectedPlacement) return null;
    const a = assets.find((x) => x.id === selectedPlacement.assetId);
    return a?.thumbnailPath ?? null;
  });

  function fileUrl(path?: string | null) {
    if (!path) return null;
    try {
      return convertFileSrc(path.replace(/\\/g, "/"));
    } catch {
      return null;
    }
  }

  async function refreshAssets() {
    try {
      assets = (await api.visualListAssets(null, 80)) as Asset[];
    } catch {
      /* ignore */
    }
  }

  async function refreshSession() {
    try {
      const s = (await api.visualGetSession()) as {
        plan?: VisualPlan;
        suggestions?: Suggestion[];
        transcript?: { segments?: typeof transcriptSegs };
      };
      if (s.plan) plan = s.plan;
      if (s.suggestions) suggestions = s.suggestions;
      if (s.transcript?.segments) transcriptSegs = s.transcript.segments;
    } catch {
      /* ignore */
    }
  }

  async function refreshWhisper() {
    try {
      const w = await api.visualWhisperStatus();
      whisperOk = w.available;
      whisperKind = w.kind;
    } catch {
      whisperOk = false;
    }
  }

  function usePlayhead() {
    const t = projectStore.currentTime || 0;
    outputStart = Math.max(0, t - 0.2);
    outputEnd = Math.min(duration, t + 3.8);
  }

  async function placeImageFile() {
    if (!projectStore.mediaPath || !api.isTauri()) {
      error = "Abre un video primero";
      return;
    }
    if (outputEnd <= outputStart) {
      error = "El fin debe ser mayor que el inicio";
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Imagen", extensions: ["jpg", "jpeg", "png", "webp"] }],
        title: `Imagen ${displayMode} ${outputStart.toFixed(1)}–${outputEnd.toFixed(1)}s`,
      });
      if (typeof p !== "string") return;
      busy = true;
      error = null;
      const res = (await api.visualCreateManualPlacement({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        imagePath: p,
        outputStart,
        outputEnd,
        displayMode,
        sourceDuration: projectStore.duration,
        label: p.split(/[/\\]/).pop() ?? "imagen",
      })) as { plan?: VisualPlan; message?: string; placement?: VisualPlacement };
      plan = res.plan ?? plan;
      if (res.placement) selectedPlacementId = res.placement.id;
      lastMessage = res.message || "Placement añadido";
      projectStore.statusMessage = lastMessage;
      await refreshAssets();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function protectRange() {
    if (!projectStore.mediaPath) return;
    try {
      busy = true;
      const p = (await api.visualAddProtectedRange({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        outputStart,
        outputEnd,
        reason: "Sin B-roll (usuario)",
        sourceDuration: projectStore.duration,
      })) as VisualPlan;
      plan = p;
      lastMessage = `Protegido ${outputStart.toFixed(1)}–${outputEnd.toFixed(1)}s`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function updateSelected(patch: {
    outputStart?: number;
    outputEnd?: number;
    displayMode?: string;
    positionX?: number;
    positionY?: number;
    sizeW?: number;
  }) {
    if (!selectedPlacementId) return;
    try {
      const p = (await api.visualUpdatePlacement({
        placementId: selectedPlacementId,
        ...patch,
      })) as VisualPlan;
      plan = p;
    } catch (e) {
      error = String(e);
    }
  }

  async function removeSelected() {
    if (!selectedPlacementId) return;
    try {
      const p = (await api.visualRemovePlacement(selectedPlacementId)) as VisualPlan;
      plan = p;
      selectedPlacementId = null;
      lastMessage = "Placement eliminado";
    } catch (e) {
      error = String(e);
    }
  }

  async function renderPlan() {
    if (!projectStore.mediaPath || !api.isTauri()) return;
    const cut =
      projectStore.lastExport?.path ||
      (await (async () => {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const p = await open({
          multiple: false,
          filters: [{ name: "Video cortado", extensions: ["mp4"] }],
          title: "MP4 cortado (timeline de salida)",
        });
        return typeof p === "string" ? p : null;
      })());
    if (!cut) {
      error = "Exporta primero el video en Silencios o elige el MP4 editado";
      return;
    }
    const parts = cut.split(/[/\\]/);
    parts.pop();
    const dir = parts.join(cut.includes("\\") ? "\\" : "/") || ".";
    const sep = cut.includes("\\") ? "\\" : "/";
    const out = `${dir}${sep}visual-enriched.mp4`;
    busy = true;
    try {
      const path = await api.visualRenderPlan(cut, out, projectStore.mediaPath);
      lastMessage = `Render → ${path}`;
      projectStore.statusMessage = lastMessage;
      projectStore.recordExportSuccess(path, projectStore.keptDuration);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function runWhisper() {
    if (!projectStore.mediaPath) {
      error = "Abre un video";
      return;
    }
    busy = true;
    preferBusyWhisper = true;
    projectStore.busy = true;
    projectStore.setProgress(5, "Whisper…", "whisper");
    try {
      const res = (await api.visualTranscribeWhisper(
        projectStore.mediaPath,
        projectStore.analysisRun?.id ?? null,
      )) as {
        transcript?: { segments?: typeof transcriptSegs };
        plan?: VisualPlan;
        suggestions?: Suggestion[];
      };
      transcriptSegs = res.transcript?.segments ?? [];
      if (res.plan) plan = res.plan;
      if (res.suggestions) suggestions = res.suggestions;
      lastMessage = `Texto: ${transcriptSegs.length} frases`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      preferBusyWhisper = false;
      projectStore.busy = false;
      projectStore.clearProgress();
      await refreshWhisper();
    }
  }

  async function pickSrt() {
    if (!api.isTauri() || !projectStore.mediaPath) return;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const p = await open({
      multiple: false,
      filters: [{ name: "SRT", extensions: ["srt", "vtt"] }],
    });
    if (typeof p !== "string") return;
    busy = true;
    try {
      const res = (await api.visualRunEnrichment(
        projectStore.mediaPath,
        projectStore.analysisRun?.id ?? null,
        p,
        false,
      )) as { transcript?: { segments?: typeof transcriptSegs }; plan?: VisualPlan };
      transcriptSegs = res.transcript?.segments ?? [];
      if (res.plan) plan = res.plan;
      lastMessage = `SRT cargado · ${transcriptSegs.length} frases`;
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void refreshAssets();
    void refreshSession();
    void refreshWhisper();
    // Init interval from playhead once
    if (outputStart === 0 && outputEnd === 4 && playhead > 0.5) {
      usePlayhead();
    }
  });
</script>

<div class="panel flex min-h-0 flex-col overflow-hidden border-sky-800/30">
  <!-- Header: primary path -->
  <div class="border-b border-surface-800 px-3 py-2">
    <div class="text-sm font-semibold text-surface-100">Visual · B-roll</div>
    <div class="text-[10px] text-surface-500">
      Pausar → + Imagen → modo → ajustar → Render
    </div>
    <div class="mt-2 flex flex-wrap gap-1">
      {#each [
        { id: "colocar" as ToolTab, label: "Colocar" },
        { id: "plan" as ToolTab, label: `Plan (${placements.length})` },
        { id: "texto" as ToolTab, label: "Texto" },
        { id: "mas" as ToolTab, label: "Más" },
      ] as t}
        <button
          type="button"
          class="rounded-lg px-2.5 py-1 text-[10px] font-semibold
            {tool === t.id
            ? 'bg-sky-600 text-white'
            : 'bg-surface-900 text-surface-400 hover:text-surface-200'}"
          onclick={() => (tool = t.id)}
        >
          {t.label}
        </button>
      {/each}
    </div>
  </div>

  {#if error}
    <div class="border-b border-cut/30 bg-cut/10 px-3 py-1.5 text-[11px] text-cut">{error}</div>
  {/if}
  {#if lastMessage}
    <div class="border-b border-surface-800 px-3 py-1 text-[10px] text-surface-400">{lastMessage}</div>
  {/if}

  <div class="min-h-0 flex-1 space-y-2 overflow-y-auto p-2">
    <!-- Always-visible track -->
    <VisualTrack
      duration={duration}
      currentTime={playhead}
      {placements}
      {protectedRanges}
      selectedId={selectedPlacementId}
      onSelect={(id) => {
        selectedPlacementId = id;
        tool = "plan";
      }}
    />

    {#if tool === "colocar"}
      <VisualPlaceForm
        bind:outputStart
        bind:outputEnd
        bind:displayMode
        {duration}
        {busy}
        onPlaceFile={placeImageFile}
        onProtect={protectRange}
        onChangeStart={(v) => (outputStart = v)}
        onChangeEnd={(v) => (outputEnd = v)}
        onChangeMode={(m) => (displayMode = m)}
        onUsePlayhead={usePlayhead}
      />
      <PlacementInspector
        placement={selectedPlacement}
        thumbPath={selectedThumb}
        {busy}
        onUpdate={updateSelected}
        onRemove={removeSelected}
      />
      {#if placements.length > 0}
        <button type="button" class="btn-primary w-full text-xs" disabled={busy} onclick={renderPlan}>
          Render plan ({placements.filter((p) => p.status === "active").length} imágenes)
        </button>
      {/if}
    {:else if tool === "plan"}
      <PlacementInspector
        placement={selectedPlacement}
        thumbPath={selectedThumb}
        {busy}
        onUpdate={updateSelected}
        onRemove={removeSelected}
      />
      <ul class="space-y-1">
        {#each placements as pl (pl.id)}
          {@const a = assets.find((x) => x.id === pl.assetId)}
          {@const u = fileUrl(a?.thumbnailPath)}
          <li>
            <button
              type="button"
              class="flex w-full items-center gap-2 rounded-lg border px-2 py-1.5 text-left text-[10px]
                {selectedPlacementId === pl.id
                ? 'border-sky-500 bg-sky-950/40'
                : 'border-surface-800 bg-surface-950/50'}"
              onclick={() => (selectedPlacementId = pl.id)}
            >
              {#if u}
                <img src={u} alt="" class="h-8 w-8 rounded object-cover" />
              {/if}
              <span class="min-w-0 flex-1 truncate text-surface-200"
                >{pl.label || pl.assetId}</span
              >
              <span class="font-mono text-surface-500"
                >{pl.outputStart.toFixed(1)}–{pl.outputEnd.toFixed(1)}</span
              >
            </button>
          </li>
        {:else}
          <li class="text-[10px] text-surface-500">Sin placements. Usa la pestaña Colocar.</li>
        {/each}
      </ul>
      {#if protectedRanges.length}
        <div class="text-[10px] text-surface-400">
          <div class="mb-1 font-semibold">Zonas protegidas</div>
          {#each protectedRanges as pr (pr.id)}
            <div class="flex items-center justify-between gap-2 py-0.5">
              <span class="font-mono"
                >{pr.outputStart.toFixed(1)}–{pr.outputEnd.toFixed(1)} · {pr.reason}</span
              >
              <button
                type="button"
                class="btn-ghost text-[9px] text-cut"
                onclick={async () => {
                  try {
                    plan = (await api.visualRemoveProtectedRange(pr.id)) as VisualPlan;
                  } catch (e) {
                    error = String(e);
                  }
                }}>Quitar</button
              >
            </div>
          {/each}
        </div>
      {/if}
      <button type="button" class="btn-primary w-full text-xs" disabled={busy} onclick={renderPlan}>
        Render plan
      </button>
    {:else if tool === "texto"}
      <div class="space-y-2">
        <p class="text-[10px] text-surface-500">
          Opcional: ayuda a sugerir conceptos. El placement manual no lo necesita.
        </p>
        <div class="flex flex-wrap gap-1">
          <button
            type="button"
            class="btn-secondary text-[10px]"
            disabled={busy || preferBusyWhisper}
            onclick={runWhisper}
          >
            {whisperOk ? "Transcribir Whisper" : "Whisper…"}
          </button>
          <button type="button" class="btn-ghost text-[10px]" disabled={busy} onclick={pickSrt}
            >Importar SRT</button
          >
        </div>
        {#if whisperOk}
          <span class="text-[9px] text-keep">Whisper listo · {whisperKind}</span>
        {/if}
        <div class="max-h-48 space-y-1 overflow-y-auto text-[10px]">
          {#each transcriptSegs.slice(0, 40) as seg}
            <button
              type="button"
              class="block w-full rounded px-1 py-0.5 text-left hover:bg-surface-800"
              onclick={() => {
                // Map source-ish to place form (approx output if no cuts)
                outputStart = seg.span.start;
                outputEnd = Math.min(duration, seg.span.end + 1);
                tool = "colocar";
              }}
            >
              <span class="font-mono text-surface-600"
                >{seg.span.start.toFixed(1)}–{seg.span.end.toFixed(1)}</span
              >
              {seg.text}
            </button>
          {:else}
            <p class="text-surface-600">Sin texto todavía.</p>
          {/each}
        </div>
      </div>
    {:else}
      <div class="space-y-2 text-[10px] text-surface-400">
        <p>Biblioteca: {assets.length} imágenes</p>
        <ul class="max-h-40 space-y-1 overflow-y-auto">
          {#each assets.slice(0, 12) as a (a.id)}
            {@const u = fileUrl(a.thumbnailPath)}
            <li class="flex items-center gap-2">
              {#if u}<img src={u} alt="" class="h-7 w-7 rounded object-cover" />{/if}
              <span class="truncate">{a.title}</span>
            </li>
          {/each}
        </ul>
        {#if suggestions.length}
          <p class="font-semibold text-surface-300">Sugerencias auto: {suggestions.length}</p>
        {/if}
        <p class="text-surface-600">
          EDL = cortes · VisualPlan = overlays. Originales intactos.
        </p>
      </div>
    {/if}
  </div>
</div>
