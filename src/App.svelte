<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import VideoPreview from "$lib/components/VideoPreview.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import Welcome from "$lib/components/Welcome.svelte";
  import ActionBar from "$lib/components/ActionBar.svelte";
  import ExportSuccess from "$lib/components/ExportSuccess.svelte";
  import ClippingPanel from "$lib/components/ClippingPanel.svelte";
  import ShortPlayer from "$lib/components/ShortPlayer.svelte";
  import RightDock from "$lib/components/RightDock.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import VisualPanel from "$lib/components/VisualPanel.svelte";
  import VisualWorkspace from "$lib/components/visual/VisualWorkspace.svelte";
  import AuxTabShell from "$lib/components/AuxTabShell.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import type { FfmpegStatus, JobProgress } from "$lib/types";
  import * as api from "$lib/utils/tauri";

  let ffmpeg = $state<FfmpegStatus | null>(null);
  let version = $state("0.1.0");
  /** Top-level work mode — tabs in TopBar, no left sidebar */
  let workspaceTab = $state<"silence" | "clips" | "visual">("silence");
  let toast = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  /** Silence mode: closable bottom dock (was right sidebar). */
  type SilenceAux = "resumen" | "supervision" | "timeline" | "lote" | "ajustes";
  let silenceOpen = $state<SilenceAux[]>(["supervision", "resumen"]);
  let silenceActive = $state<SilenceAux | null>("supervision");

  type ClipsAux = "candidatos" | "export";
  let clipsOpen = $state<ClipsAux[]>(["candidatos"]);
  let clipsActive = $state<ClipsAux | null>("candidatos");

  function showToast(msg: string) {
    toast = msg;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toast = null;
    }, 6000);
  }

  onMount(() => {
    void (async () => {
      try {
        const info = await api.getAppInfo();
        version = info.version;
      } catch {
        /* ignore */
      }
      try {
        ffmpeg = await api.checkFfmpeg();
      } catch {
        ffmpeg = { available: false, error: "check failed" };
      }
      await projectStore.refreshPresets();
    })();

    let unlistenProgress: (() => void) | undefined;
    if (api.isTauri()) {
      void import("@tauri-apps/api/event").then(({ listen }) => {
        void listen<JobProgress>("vigilcut://progress", (ev) => {
          const p = ev.payload;
          if (!p) return;
          projectStore.setProgress(p.percent, p.message, p.stage);
        }).then((fn) => {
          unlistenProgress = fn;
        });
      });
    }

    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
      if ((e.target as HTMLElement)?.isContentEditable) return;
      if (!projectStore.mediaPath) return;
      if (projectStore.showExportSuccess) return;

      if (e.code === "Space") {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("vigilcut:toggle-play"));
        return;
      }
      if (e.key === "k" || e.key === "K") {
        e.preventDefault();
        const seg = projectStore.selectedSegment;
        if (seg) {
          projectStore.markAndAdvance(seg.id, "keep");
          projectStore.statusMessage = "Queda · siguiente";
        }
        return;
      }
      if (e.key === "x" || e.key === "X") {
        e.preventDefault();
        const seg = projectStore.selectedSegment;
        if (seg) {
          projectStore.markAndAdvance(seg.id, "cut");
          projectStore.statusMessage = "Cortar · siguiente";
        }
        return;
      }
      if (e.key === "ArrowRight" && !e.ctrlKey) {
        e.preventDefault();
        const segs = projectStore.segments;
        const idx = segs.findIndex((s) => s.id === projectStore.selectedSegmentId);
        if (idx >= 0 && idx < segs.length - 1) projectStore.selectSegment(segs[idx + 1].id);
        return;
      }
      if (e.key === "ArrowLeft" && !e.ctrlKey) {
        e.preventDefault();
        const segs = projectStore.segments;
        const idx = segs.findIndex((s) => s.id === projectStore.selectedSegmentId);
        if (idx > 0) projectStore.selectSegment(segs[idx - 1].id);
        return;
      }
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        void exportVideo(e.shiftKey);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      unlistenProgress?.();
      if (toastTimer) clearTimeout(toastTimer);
    };
  });

  $effect(() => {
    const err = projectStore.error;
    if (err) showToast(err);
  });

  async function openFile() {
    if (!api.isTauri()) {
      projectStore.statusMessage = "Abre la app de escritorio para cargar videos";
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm", "m4v"] }],
      });
      if (typeof p === "string") await projectStore.openMedia(p);
    } catch (e) {
      projectStore.error = String(e);
    }
  }

  function listenResult() {
    window.dispatchEvent(new CustomEvent("vigilcut:listen-result"));
  }

  function defaultExportPath(mediaPath: string): string {
    const parts = mediaPath.split(/[/\\]/);
    const file = parts.pop() ?? "video.mp4";
    const dir = parts.join(mediaPath.includes("\\") ? "\\" : "/") || ".";
    const sep = mediaPath.includes("\\") ? "\\" : "/";
    const base = file.replace(/\.[^.]+$/, "") || "vigilcut";
    return `${dir}${sep}${base}-editado.mp4`;
  }

  function canExport(): boolean {
    if (!projectStore.mediaPath || projectStore.mediaPath.startsWith("demo://")) return false;
    if (projectStore.keepRanges.length > 0) return true;
    if ((projectStore.estimate?.estimatedDuration ?? 0) > 0.05) return true;
    if (projectStore.keepCount > 0) return true;
    return projectStore.segments.some((s) => s.decision !== "cut");
  }

  async function exportVideo(saveAs = false) {
    if (!projectStore.mediaPath || projectStore.segments.length === 0) return;
    if (projectStore.mediaPath.startsWith("demo://")) {
      projectStore.statusMessage = "Abre un video real para exportar";
      return;
    }
    if (!canExport()) {
      projectStore.statusMessage = "No hay tramos que conservar para exportar";
      return;
    }
    if (!api.isTauri()) {
      projectStore.statusMessage = "Exportar solo funciona en la app de escritorio";
      return;
    }
    try {
      const base =
        projectStore.mediaPath.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") ?? "vigilcut";
      let out: string;
      if (saveAs) {
        const { save } = await import("@tauri-apps/plugin-dialog");
        const picked = await save({
          filters: [{ name: "MP4", extensions: ["mp4"] }],
          defaultPath: `${base}-editado.mp4`,
        });
        if (!picked) {
          projectStore.statusMessage = "Exportación cancelada";
          return;
        }
        out = picked;
      } else {
        out = defaultExportPath(projectStore.mediaPath);
      }
      projectStore.busy = true;
      projectStore.error = null;
      projectStore.clearProgress();
      projectStore.statusMessage = "Exportando video…";
      projectStore.progressPercent = 5;
      const result = await api.exportVideo({
        mediaPath: projectStore.mediaPath,
        outputPath: out,
        keepRanges:
          projectStore.keepRanges.length > 0 ? projectStore.keepRanges : undefined,
        segments: projectStore.segments,
        exportOptions: projectStore.project?.preset.export,
        colorOptions: projectStore.project?.preset.color,
        audioOptions: projectStore.audioEnhance,
        hasAudio: projectStore.media?.hasAudio ?? true,
      });
      if (projectStore.analysisRun?.id) {
        try {
          await api.writeExportArtifacts(projectStore.analysisRun.id, result.outputPath);
        } catch (artErr) {
          console.warn("artifacts", artErr);
        }
      }
      projectStore.recordExportSuccess(result.outputPath, result.duration);
    } catch (e) {
      const msg = String(e);
      if (msg.toLowerCase().includes("cancel")) {
        projectStore.statusMessage = "Exportación cancelada";
        projectStore.error = null;
      } else {
        projectStore.error = msg;
        projectStore.statusMessage = "Error al exportar";
      }
    } finally {
      projectStore.busy = false;
      projectStore.clearProgress();
    }
  }

  const silenceCatalog = $derived([
    { id: "resumen" as SilenceAux, label: "Resumen" },
    {
      id: "supervision" as SilenceAux,
      label: "Excepciones",
      badge:
        projectStore.pendingExceptionCount > 0
          ? projectStore.pendingExceptionCount
          : undefined,
      alert: projectStore.pendingExceptionCount > 0,
    },
    { id: "timeline" as SilenceAux, label: "Tramos" },
    { id: "lote" as SilenceAux, label: "Lote" },
    { id: "ajustes" as SilenceAux, label: "Ajustes" },
  ]);

  function openSilence(id: SilenceAux) {
    if (!silenceOpen.includes(id)) silenceOpen = [...silenceOpen, id];
    silenceActive = id;
  }
  function closeSilence(id: SilenceAux) {
    silenceOpen = silenceOpen.filter((x) => x !== id);
    if (silenceActive === id) silenceActive = silenceOpen.at(-1) ?? null;
  }
</script>

<!-- Shell: fills #app; StatusBar shrink-0 always fully visible -->
<div
  class="relative flex h-full min-h-0 min-w-0 w-full max-w-full flex-1 flex-col overflow-hidden bg-surface-950 text-surface-100"
  style="box-sizing:border-box"
>
  <TopBar
    {ffmpeg}
    {version}
    mode={workspaceTab}
    onMode={(m) => (workspaceTab = m)}
    onOpen={openFile}
    onReanalyze={() => projectStore.reanalyze()}
  />

  {#if workspaceTab === "visual" && !projectStore.mediaPath}
    <!-- Visuales sin video: Biblioteca + Por revisar (UNIFIED_VISUALS_UX_SPEC) -->
    <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden p-2">
      <VisualWorkspace
        onMessage={(m) => {
          projectStore.statusMessage = m;
          showToast(m);
        }}
        onError={(e) => {
          projectStore.error = e;
        }}
      />
    </div>
  {:else if !projectStore.mediaPath}
    <div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <Welcome
        onOpen={openFile}
        onGoSilence={() => {
          workspaceTab = "silence";
          void openFile();
        }}
        onGoClips={() => {
          workspaceTab = "clips";
          void openFile();
        }}
        onGoVisual={() => {
          workspaceTab = "visual";
        }}
        onOpenPath={(path) => {
          void projectStore.openMedia(path);
        }}
      />
    </div>
  {:else}
    <!--
      Priority: tools + timeline always fully on screen.
      Video is compact (scaled), not full native size — frees vertical space.
    -->
    <!--
      Full window width (no empty "others" column).
      Vertical: compact video → timeline fills free space (tline) → tools → status.
    -->
    <main
      class="flex min-h-0 min-w-0 w-full max-w-full flex-1 flex-col gap-1 overflow-hidden p-1.5 sm:p-2"
      style="box-sizing:border-box"
    >
      {#if workspaceTab === "silence"}
        <div class="w-full min-w-0 max-w-full shrink-0">
          <VideoPreview compact />
        </div>
        <div class="min-h-0 w-full min-w-0 max-w-full flex-1 overflow-hidden rounded-xl border border-surface-800">
          <Timeline />
        </div>
        <div class="w-full min-w-0 max-w-full shrink-0">
          <ActionBar
            onApply={() => exportVideo(false)}
            onApplyAs={() => exportVideo(true)}
            onListenResult={listenResult}
          />
        </div>
        <div class="w-full min-w-0 max-w-full shrink-0">
          <AuxTabShell
            tabs={silenceCatalog.filter((t) => silenceOpen.includes(t.id))}
            openIds={silenceOpen}
            activeId={silenceActive}
            catalog={silenceCatalog}
            expanded={!!silenceActive && silenceOpen.length > 0}
            onOpen={(id) => openSilence(id as SilenceAux)}
            onClose={(id) => closeSilence(id as SilenceAux)}
            onActivate={(id) => (silenceActive = id as SilenceAux)}
          >
            <RightDock forceTab={silenceActive} embedded />
          </AuxTabShell>
        </div>
      {:else if workspaceTab === "clips"}
        <div
          class="mx-auto w-full min-w-0 max-w-full shrink-0 overflow-hidden rounded-xl border border-amber-700/40 bg-surface-950"
          style="height: clamp(140px, 22vh, 200px); max-width: min(100%, 18rem); box-sizing: border-box"
        >
          <ShortPlayer />
        </div>
        <div class="min-h-0 w-full min-w-0 max-w-full flex-1 overflow-hidden">
          <AuxTabShell
            tabs={[
              { id: "candidatos", label: "Candidatos" },
              { id: "export", label: "Export" },
            ].filter((t) => clipsOpen.includes(t.id as ClipsAux))}
            openIds={clipsOpen}
            activeId={clipsActive}
            catalog={[
              { id: "candidatos", label: "Candidatos" },
              { id: "export", label: "Export" },
            ]}
            expanded={!!clipsActive && clipsOpen.length > 0}
            onOpen={(id) => {
              if (!clipsOpen.includes(id as ClipsAux)) clipsOpen = [...clipsOpen, id as ClipsAux];
              clipsActive = id as ClipsAux;
            }}
            onClose={(id) => {
              clipsOpen = clipsOpen.filter((x) => x !== id);
              if (clipsActive === id) clipsActive = clipsOpen.at(-1) ?? null;
            }}
            onActivate={(id) => (clipsActive = id as ClipsAux)}
          >
            <ClippingPanel />
          </AuxTabShell>
        </div>
      {:else}
        <!-- Visual: 70% video+timeline | 30% tools (layout inside VisualPanel) -->
        <div class="min-h-0 w-full min-w-0 max-w-full flex-1 overflow-hidden">
          <VisualPanel />
        </div>
      {/if}
    </main>
  {/if}

  <!-- Always fully visible — never inside a clipped overflow region -->
  <!-- Status always last in flex stack = always visible (never under full-screen overlays) -->
  <StatusBar />

  <!-- Success dialog: does not cover StatusBar (bottom inset) -->
  <ExportSuccess onNewVideo={openFile} />

  {#if toast}
    <div class="absolute bottom-14 left-1/2 z-[60] w-[min(92vw,420px)] max-w-full -translate-x-1/2 px-2">
      <div
        class="flex items-start gap-2 rounded-xl border border-cut/50 bg-cut/95 px-4 py-3 text-sm text-white shadow-2xl"
      >
        <span class="mt-0.5 shrink-0">⚠</span>
        <p class="min-w-0 flex-1 break-words text-[13px] leading-snug">{toast}</p>
        <button
          type="button"
          class="shrink-0 text-xs text-white/80 hover:text-white"
          onclick={() => {
            toast = null;
            projectStore.error = null;
          }}>Cerrar</button
        >
      </div>
    </div>
  {/if}

  <!-- Busy overlay: only while processing; never blocks StatusBar -->
  {#if projectStore.busy && !projectStore.showExportSuccess}
    <div
      class="pointer-events-none absolute inset-x-0 top-0 bottom-8 z-50 flex items-start justify-center bg-surface-950/40 pt-24 backdrop-blur-[1px]"
    >
      <div
        class="pointer-events-auto w-[min(92vw,360px)] rounded-xl border border-surface-700 bg-surface-900 px-5 py-4 shadow-xl"
      >
        <div class="flex items-center gap-3">
          <span class="h-2.5 w-2.5 shrink-0 animate-pulse rounded-full bg-vigil-400"></span>
          <span class="min-w-0 text-sm text-surface-200"
            >{projectStore.statusMessage || "Procesando…"}</span
          >
        </div>
        <div class="mt-3 h-2 overflow-hidden rounded-full bg-surface-800">
          <div
            class="h-full rounded-full bg-vigil-500 transition-all duration-300"
            style="width: {Math.max(8, projectStore.progressPercent ?? 15)}%"
          ></div>
        </div>
        {#if projectStore.progressStage}
          <p class="mt-1 text-[10px] text-surface-500">{projectStore.progressStage}</p>
        {/if}
        <button
          type="button"
          class="btn-ghost mt-3 w-full text-xs"
          onclick={() => {
            void api.cancelJob().catch(() => {});
            projectStore.busy = false;
            projectStore.clearProgress();
            projectStore.statusMessage = "Cancelado";
          }}
        >
          Cancelar
        </button>
      </div>
    </div>
  {/if}
</div>
