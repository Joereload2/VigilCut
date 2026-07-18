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
  import { projectStore } from "$lib/stores/project.svelte";
  import type { FfmpegStatus } from "$lib/types";
  import * as api from "$lib/utils/tauri";

  let ffmpeg = $state<FfmpegStatus | null>(null);
  let version = $state("0.1.0");

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
        void exportVideo();
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

  async function exportVideo() {
    if (!projectStore.mediaPath || projectStore.segments.length === 0) return;
    if (projectStore.mediaPath.startsWith("demo://")) {
      projectStore.statusMessage = "Abre un video real para exportar";
      return;
    }
    if (projectStore.keepCount === 0) {
      projectStore.statusMessage = "Marca al menos un tramo como Mantener";
      return;
    }
    if (!api.isTauri()) {
      projectStore.statusMessage = "Exportar solo funciona en la app de escritorio";
      return;
    }
    try {
      const base =
        projectStore.mediaPath.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") ?? "vigilcut";
      const { save } = await import("@tauri-apps/plugin-dialog");
      const out = await save({
        filters: [{ name: "MP4", extensions: ["mp4"] }],
        defaultPath: `${base}-editado.mp4`,
      });
      if (!out) {
        projectStore.statusMessage = "Exportación cancelada";
        return;
      }
      projectStore.busy = true;
      projectStore.error = null;
      projectStore.statusMessage = "Exportando video…";
      const result = await api.exportVideo({
        mediaPath: projectStore.mediaPath,
        outputPath: out,
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
      <Welcome onOpen={openFile} />
      <div class="mx-auto w-full max-w-lg">
        <BatchPanel />
      </div>
    </div>
  {:else}
    <main
      class="grid min-h-0 flex-1 grid-cols-1 gap-2 p-2 lg:grid-cols-[minmax(0,1fr)_260px]"
    >
      <div class="flex min-h-0 min-w-0 flex-col gap-2">
        <VideoPreview />
        <Timeline />
        <ActionBar onApply={exportVideo} onListenResult={listenResult} />
      </div>
      <aside class="flex min-h-[200px] flex-col gap-2 overflow-y-auto lg:min-h-0">
        <BatchPanel />
        <div class="min-h-[180px] flex-[1.2]">
          <ExceptionQueue />
        </div>
        <div class="min-h-[160px] flex-1">
          <SidePanel />
        </div>
      </aside>
    </main>
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
