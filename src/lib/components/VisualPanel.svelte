<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";
  import VideoPreview from "./VideoPreview.svelte";
  import SupervisedTimeline from "./visual/SupervisedTimeline.svelte";
  import VisualWorkspace from "./visual/VisualWorkspace.svelte";
  import type {
    Asset,
    CompositionIssue,
    DisplayMode,
    Suggestion,
    VisualPlacement,
    VisualPlan,
  } from "./visual/types";

  let busy = $state(false);
  let error = $state<string | null>(null);
  let lastMessage = $state("");
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
        }}
        onMove={movePlacementLocal}
        onSnapMove={snapPlacement}
      />
    </div>
  </div>

  <!-- RIGHT 30%: Visuales unificado (Este video | Biblioteca | Por revisar) -->
  <aside
    class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-xl border border-surface-800 bg-surface-900/60 p-2"
    style="box-sizing:border-box"
    aria-label="Visuales"
  >
    {#if error}
      <div class="mb-1 shrink-0 break-words rounded border border-cut/30 bg-cut/10 px-2 py-0.5 text-[10px] text-cut">
        {error}
      </div>
    {/if}
    {#if lastMessage}
      <p class="mb-1 shrink-0 truncate text-[10px] text-surface-500">{lastMessage}</p>
    {/if}
    <div class="min-h-0 flex-1 overflow-hidden">
      <VisualWorkspace
        compact
        bind:projectKey
        onMessage={(m) => {
          lastMessage = m;
          projectStore.statusMessage = m;
        }}
        onError={(e) => {
          error = e;
        }}
        onPlanUpdated={(p) => {
          plan = p as VisualPlan;
          syncPlanToPlayer(plan);
        }}
      />
    </div>
    {#if placements.length > 0}
      <button
        type="button"
        class="btn-primary mt-1 w-full shrink-0 text-[10px]"
        disabled={busy}
        onclick={renderPlan}
      >
        {previewPhase === "rendering" ? "Exportando…" : "Exportar con imágenes"}
      </button>
    {/if}
  </aside>
</div>
