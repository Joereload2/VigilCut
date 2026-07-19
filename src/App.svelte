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
  import type { FfmpegStatus } from "$lib/types";
  import * as api from "$lib/utils/tauri";

  let ffmpeg = $state<FfmpegStatus | null>(null);
  let version = $state("0.1.0");
  /** Top-level work mode — always visible when a video is open */
  let workspaceTab = $state<"silence" | "clips">("silence");

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
    return () => window.removeEventListener("keydown", onKey);
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
      projectStore.statusMessage = "Exportando video…";
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
      projectStore.error = String(e);
      projectStore.statusMessage = "Error al exportar";
    } finally {
      projectStore.busy = false;
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
                >Short seleccionado · vista 9:16 en vivo</span
              >
              <span class="text-[10px] text-surface-500">clic en candidato = play</span>
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

  {#if projectStore.busy}
    <div
      class="pointer-events-none absolute inset-0 z-50 flex items-start justify-center bg-surface-950/40 pt-24 backdrop-blur-[1px]"
    >
      <div
        class="pointer-events-auto flex items-center gap-3 rounded-xl border border-surface-700 bg-surface-900 px-5 py-3 shadow-xl"
      >
        <span class="h-2.5 w-2.5 animate-pulse rounded-full bg-vigil-400"></span>
        <span class="text-sm text-surface-200">{projectStore.statusMessage || "Procesando…"}</span>
      </div>
    </div>
  {/if}
</div>
