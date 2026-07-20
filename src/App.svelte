<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import VideoPreview from "$lib/components/VideoPreview.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import SidePanel from "$lib/components/SidePanel.svelte";
  import ExceptionQueue from "$lib/components/ExceptionQueue.svelte";
  import BatchPanel from "$lib/components/BatchPanel.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import Welcome from "$lib/components/Welcome.svelte";
  import ActionBar from "$lib/components/ActionBar.svelte";
  import ExportSuccess from "$lib/components/ExportSuccess.svelte";
  import ClippingPanel from "$lib/components/ClippingPanel.svelte";
  import ModeNav from "$lib/components/ModeNav.svelte";
  import ShortPlayer from "$lib/components/ShortPlayer.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import type { FfmpegStatus, JobProgress } from "$lib/types";
  import * as api from "$lib/utils/tauri";

  let ffmpeg = $state<FfmpegStatus | null>(null);
  let version = $state("0.1.0");
  /** Top-level work mode — always visible when a video is open */
  let workspaceTab = $state<"silence" | "clips">("silence");
  let toast = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

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
        // Ctrl+Shift+Enter = elegir destino; Ctrl+Enter = 1-clic al lado del origen
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

  // Surface store errors as toast
  $effect(() => {
    const err = projectStore.error;
    if (err) showToast(err);
  });

  async function openFile() {
    if (api.isTauri()) {
      try {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const selected = await open({
          multiple: false,
          filters: [
            {
              name: "Video",
              extensions: ["mp4", "mov", "mkv", "webm", "avi", "m4v", "wmv"],
            },
          ],
        });
        if (selected && typeof selected === "string") {
          await projectStore.openMedia(selected);
        }
      } catch (e) {
        projectStore.error = String(e);
      }
    } else {
      const path = prompt("Ruta al video:", "");
      if (path) await projectStore.openMedia(path);
    }
  }

  function listenResult() {
    window.dispatchEvent(new CustomEvent("vigilcut:listen-result"));
  }

  /** Factory default: same folder as source, no save dialog. */
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
    // pending-as-keep still yields exportable speech blocks
    return projectStore.segments.some((s) => s.decision !== "cut");
  }

  /** @param saveAs — true = choose path (rare); false = 1-click next to source */
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
      // EDL-first: prefer keepRanges from engine; segments only as override surface
      const result = await api.exportVideo({
        mediaPath: projectStore.mediaPath,
        outputPath: out,
        keepRanges:
          projectStore.keepRanges.length > 0
            ? projectStore.keepRanges
            : undefined,
        segments: projectStore.segments,
        exportOptions: projectStore.project?.preset.export,
        colorOptions: projectStore.project?.preset.color,
        audioOptions: projectStore.project?.preset.audio,
        hasAudio: projectStore.media?.hasAudio ?? true,
      });
      // Multi-artifact factory pack (chapters, shorts, events, edl, manifest)
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

  async function cancelBusyJob() {
    try {
      await api.cancelJob();
      projectStore.statusMessage = "Cancelando…";
    } catch (e) {
      console.warn(e);
    }
  }
</script>

<div class="relative flex h-full flex-col bg-surface-950 text-surface-100">
  <TopBar
    {ffmpeg}
    {version}
    onOpen={openFile}
    onReanalyze={() => projectStore.reanalyze()}
  />

  {#if !projectStore.mediaPath}
    <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
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
        onOpenPath={(path) => {
          void projectStore.openMedia(path);
        }}
      />
      <div class="mx-auto w-full max-w-lg">
        <BatchPanel />
      </div>
    </div>
  {:else}
    <div class="flex min-h-0 flex-1 overflow-hidden">
      <ModeNav mode={workspaceTab} onMode={(m) => (workspaceTab = m)} />

      {#if workspaceTab === "silence"}
        <main
          class="grid min-h-0 flex-1 grid-cols-1 gap-2 overflow-hidden p-2 lg:grid-cols-[minmax(0,1.4fr)_minmax(260px,320px)]"
        >
          <div class="flex min-h-0 min-w-0 flex-col gap-2">
            <div class="flex min-h-[300px] flex-1 flex-col overflow-hidden lg:min-h-[380px]">
              <VideoPreview />
            </div>
            <ActionBar
              onApply={() => exportVideo(false)}
              onApplyAs={() => exportVideo(true)}
              onListenResult={listenResult}
            />
            <details
              class="max-h-[36%] shrink-0 overflow-hidden rounded-xl border border-surface-800 bg-surface-900/40"
            >
              <summary
                class="cursor-pointer select-none px-3 py-2 text-xs font-medium text-surface-400 hover:text-surface-200"
              >
                Supervisión ({projectStore.pendingExceptionCount} excepciones) · timeline
                ({projectStore.segments.length})
              </summary>
              <div class="max-h-56 space-y-2 overflow-y-auto border-t border-surface-800 p-2">
                <ExceptionQueue />
                <details class="rounded-lg border border-surface-800">
                  <summary class="cursor-pointer px-2 py-1 text-[10px] text-surface-500"
                    >Timeline diagnóstico</summary
                  >
                  <Timeline />
                </details>
              </div>
            </details>
          </div>
          <aside class="min-h-0 space-y-2 overflow-y-auto">
            <SidePanel />
            <BatchPanel />
          </aside>
        </main>
      {:else}
        <!-- SHORTS: ModeNav (left) | 9:16 live player | candidate list -->
        <main
          class="grid min-h-0 flex-1 grid-cols-1 gap-2 overflow-hidden p-2 md:grid-cols-[minmax(0,1fr)_minmax(280px,400px)]"
        >
          <div
            class="flex min-h-[420px] min-w-0 flex-col overflow-hidden rounded-2xl border border-amber-700/40 bg-gradient-to-b from-surface-900 via-surface-950 to-black"
          >
            <div
              class="flex shrink-0 items-center justify-between gap-2 border-b border-amber-900/40 px-3 py-2"
            >
              <span class="text-xs font-semibold text-amber-100/95"
                >Vista 9:16 · short seleccionado</span
              >
              <span class="text-[10px] text-surface-500">lista a la derecha = clasificar</span>
            </div>
            <div class="min-h-0 flex-1">
              <ShortPlayer />
            </div>
          </div>
          <aside class="flex min-h-[300px] min-w-0 flex-col overflow-hidden">
            <ClippingPanel />
          </aside>
        </main>
      {/if}
    </div>
  {/if}

  <StatusBar />

  <ExportSuccess onNewVideo={openFile} />

  {#if toast}
    <div class="absolute bottom-14 left-1/2 z-[60] w-[min(92vw,420px)] -translate-x-1/2">
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

  {#if projectStore.busy}
    <div
      class="pointer-events-none absolute inset-0 z-50 flex items-start justify-center bg-surface-950/40 pt-24 backdrop-blur-[1px]"
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
        {#if projectStore.progressPercent != null}
          <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-surface-800">
            <div
              class="h-full rounded-full bg-vigil-500 transition-all duration-300"
              style="width: {Math.min(100, Math.max(2, projectStore.progressPercent))}%"
            ></div>
          </div>
          <div class="mt-1 text-right font-mono text-[10px] text-surface-500">
            {Math.round(projectStore.progressPercent)}%
            {#if projectStore.progressStage}
              · {projectStore.progressStage}{/if}
          </div>
        {/if}
        <button
          type="button"
          class="btn-ghost mt-3 w-full text-xs text-cut hover:bg-cut/10"
          onclick={cancelBusyJob}
        >
          Cancelar
        </button>
      </div>
    </div>
  {/if}
</div>
