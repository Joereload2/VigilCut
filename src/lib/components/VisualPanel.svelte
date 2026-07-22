<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";
  import VideoPreview from "./VideoPreview.svelte";
  import VisualPlaceForm from "./visual/VisualPlaceForm.svelte";
  import SupervisedTimeline from "./visual/SupervisedTimeline.svelte";
  import HorizontalProps from "./visual/HorizontalProps.svelte";
  import type {
    Asset,
    CompositionIssue,
    DisplayMode,
    Suggestion,
    VisualPlacement,
    VisualPlan,
  } from "./visual/types";

  type AuxId = "props" | "exceptions" | "library" | "texto" | "config" | "colocar";

  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastMessage = $state("");
  /** Active tool in the right column (always one; no hidden panels). */
  let activeAux = $state<AuxId>("colocar");
  /** Pre-resultado tras aplicar imágenes (mismo proyecto, sin pedir otro MP4). */
  let previewPath = $state<string | null>(null);
  /** Cut/source used as overlay base (never a second unrelated pick). */
  let resolvedBasePath = $state<string | null>(null);
  let previewPhase = $state<"idle" | "preparing" | "rendering" | "ready">("idle");

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
  /** Intelligent library coverage (optional project key from detect needs) */
  let projectKey = $state<string | null>(null);
  let coverage = $state<{
    total?: number;
    reused?: number;
    generated?: number;
    waiting?: number;
    needsReview?: number;
    uncovered?: number;
    failed?: number;
    skipped?: number;
  } | null>(null);
  let reviewQueue = $state<{ id: string; qaReason?: string; localPath?: string }[]>([]);
  let intelBusy = $state(false);

  const duration = $derived(
    projectStore.estimate?.estimatedDuration ??
      projectStore.keptDuration ??
      projectStore.duration ??
      60,
  );
  /** Single edit timeline (output) — same clock as live overlays on the main player. */
  const playhead = $derived(projectStore.outputClock());
  const placements = $derived(plan?.placements ?? []);
  const protectedRanges = $derived(plan?.protectedRanges ?? []);
  const selectedPlacement = $derived(
    placements.find((p) => p.id === selectedPlacementId) ?? null,
  );
  const selectedThumb = $derived.by(() => {
    if (!selectedPlacement) return null;
    const a = assets.find((x) => x.id === selectedPlacement.assetId);
    return a?.thumbnailPath ?? a?.managedPath ?? null;
  });

  function imagePathFor(assetId: string, placement?: VisualPlacement): string | null {
    if (placement?.thumbnailPath) return placement.thumbnailPath;
    const a = assets.find((x) => x.id === assetId);
    return a?.managedPath ?? a?.thumbnailPath ?? null;
  }

  function syncPlanToPlayer(p: VisualPlan | null) {
    if (!p) {
      projectStore.setVisualPlan([]);
      return;
    }
    const list = (p.placements ?? []).map((pl) => ({
      id: pl.id,
      assetId: pl.assetId,
      outputStart: pl.outputStart,
      outputEnd: pl.outputEnd,
      mode: pl.mode,
      status: pl.status,
      fit: pl.fit,
      layout: pl.layout,
      label: pl.label,
      imagePath: imagePathFor(pl.assetId, pl),
      relatedText: pl.relatedText,
      confidence: pl.confidence,
      reviewStatus: pl.reviewStatus,
      manualOverride: pl.manualOverride,
    }));
    // Drop legacy auto face/safe_area demos unless user enabled zone display
    const zones = (p.spatialZones ?? []).filter((z) => {
      if (projectStore.visualShowZones) return true;
      // Keep only user/manual zones by default
      return z.kind === "manual" || !!z.label?.toLowerCase().includes("manual");
    });
    projectStore.setVisualPlan(
      list,
      (p.protectedRanges ?? []).map((r) => ({
        id: r.id,
        outputStart: r.outputStart,
        outputEnd: r.outputEnd,
        reason: r.reason,
      })),
      {
        spatialZones: zones,
        issues: p.issues ?? [],
      },
    );
  }

  $effect(() => {
    syncPlanToPlayer(plan);
    void assets.length;
  });

  $effect(() => {
    projectStore.visualSelectedId = selectedPlacementId;
  });

  // Spatial drag from main preview → composition model (debounced write)
  $effect(() => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    const handler = (ev: Event) => {
      const d = (ev as CustomEvent<{ id: string; x: number; y: number; w: number; h?: number }>)
        .detail;
      if (!d?.id) return;
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        selectedPlacementId = d.id;
        void updateSelected({
          positionX: d.x,
          positionY: d.y,
          sizeW: d.w,
          sizeH: d.h,
          manualOverride: true,
        });
      }, 120);
    };
    window.addEventListener("vigilcut:visual-layout", handler);
    return () => {
      if (timer) clearTimeout(timer);
      window.removeEventListener("vigilcut:visual-layout", handler);
    };
  });

  const issues = $derived(plan?.issues ?? []);
  const exceptionCount = $derived(
    issues.filter((i) => i.severity === "warn" || i.severity === "error").length +
      placements.filter((p) => p.reviewStatus === "conflict").length,
  );

  function imagePathForPlacement(pl: VisualPlacement): string | null {
    return imagePathFor(pl.assetId, pl);
  }

  async function movePlacementLocal(id: string, s: number, e: number) {
    // Optimistic UI while dragging
    if (plan?.placements) {
      plan = {
        ...plan,
        placements: plan.placements.map((p) =>
          p.id === id ? { ...p, outputStart: s, outputEnd: e } : p,
        ),
      };
    }
  }

  async function snapPlacement(id: string, s: number, e: number) {
    try {
      const anchors: number[] = [0, duration];
      for (const seg of transcriptSegs) {
        const os =
          projectStore.localKeepRanges().length > 0
            ? projectStore.sourceToEdited(seg.span.start)
            : seg.span.start;
        const oe =
          projectStore.localKeepRanges().length > 0
            ? projectStore.sourceToEdited(seg.span.end)
            : seg.span.end;
        anchors.push(os, oe);
      }
      for (const p of placements) {
        if (p.id !== id) {
          anchors.push(p.outputStart, p.outputEnd);
        }
      }
      const p = (await api.visualSnapPlacement({
        placementId: id,
        outputStart: s,
        outputEnd: e,
        anchors,
        threshold: 0.2,
      })) as VisualPlan;
      plan = p;
      syncPlanToPlayer(plan);
    } catch {
      await updateSelectedTimes(id, s, e);
    }
  }

  async function updateSelectedTimes(id: string, s: number, e: number) {
    try {
      const p = (await api.visualUpdatePlacement({
        placementId: id,
        outputStart: s,
        outputEnd: e,
        manualOverride: true,
      })) as VisualPlan;
      plan = p;
      syncPlanToPlayer(plan);
    } catch (err) {
      error = String(err);
    }
  }

  async function applyIssueSuggestion(iss: CompositionIssue) {
    if (!selectedPlacementId) return;
    try {
      const p = (await api.visualUpdatePlacement({
        placementId: selectedPlacementId,
        positionX: iss.suggestedX ?? undefined,
        positionY: iss.suggestedY ?? undefined,
        sizeW: iss.suggestedW ?? undefined,
        manualOverride: true,
      })) as VisualPlan;
      plan = p;
      lastMessage = "Posición alternativa aplicada";
    } catch (err) {
      error = String(err);
    }
  }

  function fileUrl(path?: string | null) {
    if (!path) return null;
    try {
      // Keep native Windows separators — same as VideoPreview
      return convertFileSrc(path);
    } catch {
      try {
        return convertFileSrc(path.replace(/\\/g, "/"));
      } catch {
        return null;
      }
    }
  }

  function sidePath(mediaPath: string, suffix: string): string {
    const parts = mediaPath.split(/[/\\]/);
    const file = parts.pop() ?? "video.mp4";
    const dir = parts.join(mediaPath.includes("\\") ? "\\" : "/") || ".";
    const sep = mediaPath.includes("\\") ? "\\" : "/";
    const base = file.replace(/\.[^.]+$/, "") || "vigilcut";
    return `${dir}${sep}${base}${suffix}`;
  }

  const baseLabel = $derived.by(() => {
    const p = resolvedBasePath ?? projectStore.lastExport?.path ?? projectStore.mediaPath;
    if (!p) return "—";
    return p.split(/[/\\]/).pop() ?? p;
  });

  function isVisualResultPath(path: string): boolean {
    return /-con-imagenes(-\d+)?\.mp4$/i.test(path);
  }

  /** Same project only: last cut, or auto-cut current media, or the uploaded file. Never another picker. */
  async function ensureProjectBaseVideo(): Promise<string> {
    const media = projectStore.mediaPath;
    if (!media) throw new Error("Abre un video primero");

    // Reuse the base we already prepared for this session (avoid double overlays).
    if (resolvedBasePath && !isVisualResultPath(resolvedBasePath)) {
      return resolvedBasePath;
    }

    const prev = projectStore.lastExport?.path;
    if (prev && !isVisualResultPath(prev)) {
      resolvedBasePath = prev;
      return prev;
    }

    const hasKeep =
      projectStore.keepRanges.length > 0 ||
      projectStore.segments.some((s) => s.decision !== "cut");
    const hasCuts =
      projectStore.segments.some((s) => s.decision === "cut") ||
      (projectStore.cutDuration ?? 0) > 0.2;

    // No silence cuts → overlays go on the video you uploaded (one file only).
    if (!hasCuts || !hasKeep || projectStore.segments.length === 0) {
      resolvedBasePath = media;
      return media;
    }

    // Build the cut of THIS video automatically (no save dialog, no other file).
    previewPhase = "preparing";
    lastMessage = "Preparando el corte de tu video…";
    projectStore.statusMessage = lastMessage;
    const cutOut = sidePath(media, "-corte.mp4");
    const result = await api.exportVideo({
      mediaPath: media,
      outputPath: cutOut,
      keepRanges:
        projectStore.keepRanges.length > 0 ? projectStore.keepRanges : undefined,
      segments: projectStore.segments,
      exportOptions: projectStore.project?.preset.export,
      colorOptions: projectStore.project?.preset.color,
      audioOptions: projectStore.audioEnhance,
      hasAudio: projectStore.media?.hasAudio ?? true,
    });
    projectStore.recordExportSuccess(result.outputPath, result.duration, { silent: true });
    resolvedBasePath = result.outputPath;
    return result.outputPath;
  }

  async function openPreviewFolder() {
    if (!previewPath || !api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-shell");
      const parts = previewPath.split(/[/\\]/);
      parts.pop();
      const dir = parts.join(previewPath.includes("\\") ? "\\" : "/") || previewPath;
      await open(dir);
    } catch (e) {
      lastMessage = previewPath;
      console.error(e);
    }
  }

  async function exportAgreedResult() {
    if (!previewPath) {
      error = "Primero genera el pre-resultado";
      return;
    }
    // Already written next to the project video; confirm + open folder / mark success.
    projectStore.recordExportSuccess(
      previewPath,
      projectStore.keptDuration || projectStore.duration || 0,
    );
    lastMessage = `Exportado: ${previewPath.split(/[/\\]/).pop()}`;
    await openPreviewFolder();
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
    // Place on the same timeline shown in the main player (output clock)
    const t = projectStore.outputClock() || 0;
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
      lastMessage = res.message || "Imagen en la línea de tiempo (vista principal)";
      projectStore.statusMessage = lastMessage;
      // Stay on the single result timeline so overlays appear immediately
      if (projectStore.previewMode !== "edited" && projectStore.localKeepRanges().length > 0) {
        projectStore.previewMode = "edited";
      }
      // Jump playhead into the placement so fullscreen/overlay is visible now
      if (projectStore.localKeepRanges().length > 0) {
        projectStore.currentTime = projectStore.editedToSource(outputStart + 0.05);
      } else {
        projectStore.currentTime = outputStart + 0.05;
      }
      projectStore.isPlaying = false;
      await refreshAssets();
      syncPlanToPlayer(plan);
      openAuxTab("props");
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function openAuxTab(id: AuxId) {
    activeAux = id;
  }

  const toolNav = $derived([
    { id: "colocar" as AuxId, label: "Colocar" },
    { id: "props" as AuxId, label: "Propiedades" },
    {
      id: "exceptions" as AuxId,
      label: "Excepciones",
      badge: exceptionCount > 0 ? exceptionCount : undefined,
    },
    { id: "library" as AuxId, label: "Biblioteca" },
    { id: "texto" as AuxId, label: "Texto" },
    { id: "config" as AuxId, label: "Config" },
  ]);

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
    sizeH?: number;
    fit?: string;
    reviewStatus?: string;
    manualOverride?: boolean;
    restoreAi?: boolean;
    opacity?: number;
  }) {
    if (!selectedPlacementId) return;
    try {
      const p = (await api.visualUpdatePlacement({
        placementId: selectedPlacementId,
        ...patch,
      })) as VisualPlan;
      plan = p;
      syncPlanToPlayer(plan);
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
      lastMessage = "Imagen quitada de la línea de tiempo";
      syncPlanToPlayer(plan);
    } catch (e) {
      error = String(e);
    }
  }

  /**
   * Final bake only — live preview already shows images on the main video.
   * One project video → export MP4 with overlays burned in.
   */
  async function renderPlan() {
    if (!projectStore.mediaPath || !api.isTauri()) {
      error = "Abre un video en VigilCut";
      return;
    }
    const active = placements.filter((p) => p.status === "active").length;
    if (active === 0) {
      error = "Añade al menos una imagen en la pista";
      return;
    }
    busy = true;
    projectStore.busy = true;
    error = null;
    previewPath = null;
    previewPhase = "preparing";
    try {
      const base = await ensureProjectBaseVideo();
      previewPhase = "rendering";
      lastMessage = `Exportando ${active} imagen(es) en el MP4 final…`;
      projectStore.statusMessage = lastMessage;
      projectStore.setProgress(20, "Export visual…", "visual");
      const out = sidePath(projectStore.mediaPath, "-con-imagenes.mp4");
      const path = await api.visualRenderPlan(base, out, projectStore.mediaPath);
      previewPath = path;
      previewPhase = "ready";
      projectStore.setVisualPreview(path, projectStore.keptDuration || duration);
      // recordExportSuccess clears busy + shows modal + status line
      projectStore.recordExportSuccess(path, projectStore.keptDuration || duration, {
        silent: false,
      });
      lastMessage = `Exportado: ${path.split(/[/\\]/).pop()}`;
      openAuxTab("props");
    } catch (e) {
      error = String(e);
      previewPhase = "idle";
      projectStore.error = String(e);
      projectStore.statusMessage = "Error al exportar";
    } finally {
      busy = false;
      projectStore.busy = false;
      projectStore.clearProgress();
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

  // New video → reset visual pre-result (still the same project when path stable)
  $effect(() => {
    const media = projectStore.mediaPath;
    if (!media) {
      previewPath = null;
      resolvedBasePath = null;
      previewPhase = "idle";
    }
  });
</script>

<!--
  Layout per wireframe:
  - Left ~70%: video preview (top) + timeline (bottom ~30% of window height)
  - Right ~30%: tools / others (always fully visible, scroll inside)
-->
<div
  class="grid h-full min-h-0 w-full min-w-0 max-w-full gap-1.5 overflow-hidden"
  style="box-sizing:border-box; height:100%; width:100%; grid-template-columns: minmax(0, 7fr) minmax(12rem, 3fr); grid-template-rows: minmax(0, 1fr);"
>
  <!-- LEFT 70%: video + timeline -->
  <div class="flex min-h-0 min-w-0 flex-col gap-1 overflow-hidden">
    {#if error}
      <div class="shrink-0 break-words rounded-lg border border-cut/30 bg-cut/10 px-2 py-0.5 text-[10px] text-cut">
        {error}
      </div>
    {/if}

    <!-- Video takes most of the left column -->
    <div class="min-h-0 w-full min-w-0 flex-1 overflow-hidden" style="min-height: 8rem">
      <VideoPreview compact />
    </div>

    <!-- Timeline ~50% smaller: fixed compact band, not stretching -->
    <div
      class="w-full min-w-0 shrink-0 overflow-hidden"
      style="height: clamp(88px, 14vh, 120px); box-sizing: border-box"
    >
      <SupervisedTimeline
        duration={duration}
        {placements}
        {protectedRanges}
        transcript={transcriptSegs}
        {issues}
        selectedId={selectedPlacementId}
        imagePathFor={imagePathForPlacement}
        onSelect={(id) => {
          selectedPlacementId = id;
          openAuxTab("props");
        }}
        onMove={movePlacementLocal}
        onSnapMove={snapPlacement}
      />
    </div>
  </div>

  <!-- RIGHT 30%: simple tool switcher (no ×, no “+ Abrir panel”) -->
  <aside
    class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border border-surface-800 bg-surface-900/60"
    style="box-sizing:border-box"
    aria-label="Herramientas Visual"
  >
    <div class="shrink-0 border-b border-surface-800 p-2">
      <div class="mb-1 text-[11px] font-semibold text-surface-200">Herramientas</div>
      <div class="grid grid-cols-2 gap-1" role="tablist">
        {#each toolNav as t (t.id)}
          <button
            type="button"
            role="tab"
            aria-selected={activeAux === t.id}
            class="rounded-lg px-2 py-1.5 text-left text-[11px] font-semibold transition
              {activeAux === t.id
              ? 'bg-sky-600 text-white'
              : 'bg-surface-800 text-surface-300 hover:bg-surface-700'}"
            onclick={() => openAuxTab(t.id)}
          >
            {t.label}
            {#if t.badge}
              <span class="ml-1 rounded-full bg-warning/30 px-1 text-[9px] text-warning"
                >{t.badge}</span
              >
            {/if}
          </button>
        {/each}
      </div>
    </div>

    <div class="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto p-2">
      {#if activeAux === "props"}
        <HorizontalProps
          placement={selectedPlacement}
          thumbPath={selectedThumb}
          {issues}
          {busy}
          onUpdate={updateSelected}
          onRemove={removeSelected}
          onApplySuggestion={applyIssueSuggestion}
          onExport={placements.length > 0 ? renderPlan : undefined}
        />
      {:else if activeAux === "exceptions"}
        <div class="space-y-1 text-[11px]">
          <p class="text-surface-500">Solo excepciones — la IA resolvió el resto.</p>
          {#each issues.filter((i) => i.severity !== "info") as iss (iss.id)}
            <button
              type="button"
              class="flex w-full min-w-0 items-start gap-2 rounded-lg border border-amber-800/40 bg-amber-950/20 px-2 py-1.5 text-left hover:bg-amber-950/40"
              onclick={() => {
                selectedPlacementId = iss.placementId;
                openAuxTab("props");
              }}
            >
              <span class="shrink-0 font-mono text-[9px] text-amber-400">{iss.severity}</span>
              <span class="min-w-0 flex-1 text-amber-50/90">{iss.message}</span>
            </button>
          {:else}
            <p class="text-keep">Sin excepciones visuales.</p>
          {/each}
        </div>
      {:else if activeAux === "colocar"}
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
      {:else if activeAux === "library"}
        <div class="flex min-h-0 min-w-0 flex-col gap-2 overflow-y-auto">
          <!-- Coverage summary (plain language) -->
          {#if coverage && (coverage.total ?? 0) > 0}
            <div class="rounded-lg border border-surface-800 bg-surface-950/70 p-2 text-[10px] text-surface-300">
              <p class="font-medium text-surface-100">
                Cobertura visual: {(coverage.reused ?? 0) + (coverage.generated ?? 0)} de {coverage.total}
              </p>
              <ul class="mt-1 space-y-0.5 text-surface-500">
                <li>{coverage.reused ?? 0} reutilizadas</li>
                <li>{coverage.generated ?? 0} generadas</li>
                <li>{coverage.waiting ?? 0} esperando</li>
                <li>{coverage.needsReview ?? 0} para revisar</li>
                <li>{coverage.uncovered ?? 0} sin imagen</li>
              </ul>
            </div>
          {/if}
          <div class="flex flex-wrap gap-1">
            <button
              type="button"
              class="btn-secondary text-[10px]"
              disabled={busy || intelBusy || !projectStore.mediaPath}
              onclick={async () => {
                if (!projectStore.mediaPath) return;
                intelBusy = true;
                error = null;
                try {
                  const res = (await api.visualDetectNeeds({
                    mediaPath: projectStore.mediaPath,
                    analysisRunId: projectStore.analysisRun?.id ?? null,
                  })) as {
                    projectKey?: string;
                    coverage?: typeof coverage;
                  };
                  projectKey = res.projectKey ?? null;
                  coverage = res.coverage ?? null;
                  lastMessage = "Necesidades visuales detectadas";
                } catch (e) {
                  error = String(e);
                } finally {
                  intelBusy = false;
                }
              }}
            >
              Detectar necesidades
            </button>
            <button
              type="button"
              class="btn-primary text-[10px]"
              disabled={busy || intelBusy || !projectKey}
              onclick={async () => {
                if (!projectKey || !projectStore.mediaPath) return;
                intelBusy = true;
                error = null;
                try {
                  // Always search library first; generation only if user confirms later path
                  const res = (await api.visualCoverNeeds({
                    projectKey,
                    generateMissing: false,
                    maxGenerate: 0,
                  })) as { coverage?: typeof coverage; reused?: number };
                  coverage = res.coverage ?? coverage;
                  lastMessage = `Reutilizadas: ${res.reused ?? 0}. Sin generar (elige Completar si falta).`;
                  const applied = (await api.visualApplyNeedsToPlan({
                    mediaPath: projectStore.mediaPath,
                    analysisRunId: projectStore.analysisRun?.id ?? null,
                    projectKey,
                  })) as { plan?: VisualPlan; added?: number };
                  if (applied.plan) plan = applied.plan;
                  syncPlanToPlayer(plan);
                  lastMessage += ` · ${applied.added ?? 0} en el plan`;
                } catch (e) {
                  error = String(e);
                } finally {
                  intelBusy = false;
                }
              }}
            >
              Usar biblioteca
            </button>
            <button
              type="button"
              class="btn-secondary text-[10px]"
              disabled={busy || intelBusy || !projectKey}
              title="Solo si faltan imágenes. Usa mock/OmniRoute gratis; nunca pago por defecto."
              onclick={async () => {
                if (!projectKey || !projectStore.mediaPath) return;
                intelBusy = true;
                error = null;
                try {
                  const res = (await api.visualCoverNeeds({
                    projectKey,
                    generateMissing: true,
                    maxGenerate: 3,
                  })) as { coverage?: typeof coverage; processed?: number; queued?: number };
                  coverage = res.coverage ?? coverage;
                  lastMessage = `Generación: cola ${res.queued ?? 0}, procesados ${res.processed ?? 0}`;
                  const applied = (await api.visualApplyNeedsToPlan({
                    mediaPath: projectStore.mediaPath,
                    analysisRunId: projectStore.analysisRun?.id ?? null,
                    projectKey,
                  })) as { plan?: VisualPlan };
                  if (applied.plan) plan = applied.plan;
                  syncPlanToPlayer(plan);
                  reviewQueue = ((await api.visualListReviewQueue(20)) as typeof reviewQueue) ?? [];
                  await refreshAssets();
                } catch (e) {
                  error = String(e);
                } finally {
                  intelBusy = false;
                }
              }}
            >
              Completar faltantes
            </button>
            <button
              type="button"
              class="btn-ghost text-[10px]"
              disabled={intelBusy}
              onclick={async () => {
                try {
                  await api.visualSeedThemeEconomy();
                  lastMessage = "Tema economía sembrado (conceptos, sin imágenes)";
                } catch (e) {
                  error = String(e);
                }
              }}
            >
              Seed economía
            </button>
          </div>
          {#if reviewQueue.length > 0}
            <div class="space-y-1 rounded border border-amber-900/40 bg-amber-950/20 p-1.5">
              <p class="text-[10px] font-medium text-amber-200">Revisión humana</p>
              {#each reviewQueue.slice(0, 5) as c (c.id)}
                <div class="flex items-center gap-1 text-[10px]">
                  <span class="min-w-0 flex-1 truncate text-surface-400">{c.qaReason ?? c.id}</span>
                  <button
                    type="button"
                    class="btn-primary px-1 py-0 text-[9px]"
                    onclick={async () => {
                      await api.visualApproveCandidate(c.id);
                      reviewQueue = reviewQueue.filter((x) => x.id !== c.id);
                      await refreshAssets();
                    }}>OK</button
                  >
                  <button
                    type="button"
                    class="btn-ghost px-1 py-0 text-[9px]"
                    onclick={async () => {
                      await api.visualRejectCandidate(c.id);
                      reviewQueue = reviewQueue.filter((x) => x.id !== c.id);
                    }}>No</button
                  >
                </div>
              {/each}
            </div>
          {/if}
          <div class="flex min-w-0 flex-wrap gap-2">
            {#each assets.slice(0, 24) as a (a.id)}
              {@const u = fileUrl(a.thumbnailPath)}
              <button
                type="button"
                class="flex w-[6.5rem] min-w-0 flex-col rounded-lg border border-surface-800 bg-surface-950/60 p-1 text-left hover:border-brand-500/40"
                title="Colocar en el intervalo actual"
                onclick={async () => {
                  if (!projectStore.mediaPath) return;
                  try {
                    busy = true;
                    const res = (await api.visualCreateManualPlacement({
                      mediaPath: projectStore.mediaPath,
                      analysisRunId: projectStore.analysisRun?.id ?? null,
                      assetId: a.id,
                      outputStart,
                      outputEnd,
                      displayMode,
                      sourceDuration: projectStore.duration,
                      label: a.title,
                    })) as { plan?: VisualPlan; message?: string };
                    plan = res.plan ?? plan;
                    lastMessage = res.message || "Asset del plan";
                    syncPlanToPlayer(plan);
                  } catch (e) {
                    error = String(e);
                  } finally {
                    busy = false;
                  }
                }}
              >
                {#if u}
                  <img src={u} alt="" class="h-12 w-full rounded object-cover" />
                {/if}
                <span class="truncate px-0.5 text-[10px] text-surface-300">{a.title}</span>
              </button>
            {:else}
              <p class="text-[11px] text-surface-500">
                Biblioteca vacía. Importa imágenes o completa faltantes (mock offline).
              </p>
            {/each}
          </div>
        </div>
      {:else if activeAux === "texto"}
        <div class="flex min-w-0 flex-col gap-2">
          <div class="flex flex-wrap gap-1">
            <button
              type="button"
              class="btn-secondary text-[10px]"
              disabled={busy || preferBusyWhisper}
              onclick={runWhisper}
            >
              {whisperOk ? "Whisper" : "Whisper…"}
            </button>
            <button type="button" class="btn-ghost text-[10px]" disabled={busy} onclick={pickSrt}
              >SRT</button
            >
          </div>
          <div class="flex min-w-0 flex-col gap-0.5 text-[10px]">
            {#each transcriptSegs.slice(0, 40) as seg}
              <button
                type="button"
                class="w-full truncate rounded bg-surface-800 px-1.5 py-0.5 text-left hover:bg-surface-700"
                onclick={() => {
                  outputStart = seg.span.start;
                  outputEnd = Math.min(duration, seg.span.end + 1);
                  openAuxTab("colocar");
                }}
              >
                <span class="font-mono text-surface-500">{seg.span.start.toFixed(1)}s</span>
                {seg.text}
              </button>
            {:else}
              <p class="text-surface-600">Sin texto.</p>
            {/each}
          </div>
        </div>
      {:else if activeAux === "config"}
        <div class="space-y-2 text-[11px] text-surface-400">
          <label class="flex cursor-pointer items-center gap-1.5">
            <input
              type="checkbox"
              class="rounded"
              checked={projectStore.visualShowZones}
              onchange={() => (projectStore.visualShowZones = !projectStore.visualShowZones)}
            />
            Zonas protegidas en preview
          </label>
          {#if placements.length > 0}
            <button
              type="button"
              class="btn-primary w-full text-[10px]"
              disabled={busy}
              onclick={renderPlan}
            >
              {previewPhase === "rendering" ? "Exportando…" : "Exportar con imágenes"}
            </button>
          {/if}
          {#if lastMessage}
            <p class="text-[10px] text-surface-500">{lastMessage}</p>
          {/if}
        </div>
      {/if}
    </div>
  </aside>
</div>
