<script lang="ts">
  import { onMount } from "svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import VideoPreview from "$lib/components/VideoPreview.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import SegmentList from "$lib/components/SegmentList.svelte";
  import Inspector from "$lib/components/Inspector.svelte";
  import PresetPanel from "$lib/components/PresetPanel.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import Welcome from "$lib/components/Welcome.svelte";
  import { projectStore } from "$lib/stores/project.svelte";
  import type { FfmpegStatus } from "$lib/types";
  import * as api from "$lib/utils/tauri";
  import { demoSegments } from "$lib/utils/tauri";

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
      // Web fallback: fake path for demo UI
      const path = prompt("Ruta al video (demo web) / Video path:", "C:/Videos/demo.mp4");
      if (path) await projectStore.openMedia(path);
    }
  }

  function loadDemo() {
    projectStore.mediaPath = "demo://timeline";
    projectStore.media = {
      path: "demo://timeline",
      duration: 60,
      width: 1920,
      height: 1080,
      fps: 30,
      hasAudio: true,
      hasVideo: true,
      sizeBytes: 0,
    };
    projectStore.segments = demoSegments(60);
    projectStore.statusMessage = "Demo timeline cargado";
    void projectStore.refreshKeepRanges();
  }

  async function exportVideo() {
    if (!projectStore.mediaPath || projectStore.segments.length === 0) return;
    if (!api.isTauri()) {
      projectStore.statusMessage = "Export solo disponible en la app Tauri";
      return;
    }
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const out = await save({
        filters: [{ name: "MP4", extensions: ["mp4"] }],
        defaultPath: "vigilcut-export.mp4",
      });
      if (!out) return;
      projectStore.busy = true;
      projectStore.statusMessage = "Exportando…";
      const result = await api.exportVideo({
        mediaPath: projectStore.mediaPath,
        outputPath: out,
        segments: projectStore.segments,
        exportOptions: projectStore.project?.preset.export,
        colorOptions: projectStore.project?.preset.color,
        hasAudio: projectStore.media?.hasAudio ?? true,
      });
      projectStore.statusMessage = `Exportado: ${result.outputPath}`;
    } catch (e) {
      projectStore.error = String(e);
      projectStore.statusMessage = "Error de export";
    } finally {
      projectStore.busy = false;
    }
  }
</script>

<div class="flex h-full flex-col bg-surface-950 text-surface-100">
  <TopBar
    {ffmpeg}
    {version}
    onOpen={openFile}
    onExport={exportVideo}
    onReanalyze={() => projectStore.reanalyze()}
  />

  {#if !projectStore.mediaPath}
    <Welcome onOpen={openFile} onDemo={loadDemo} />
  {:else}
    <main class="grid min-h-0 flex-1 grid-cols-1 gap-2 p-2 lg:grid-cols-[1fr_280px]">
      <div class="flex min-h-0 min-w-0 flex-col gap-2">
        <VideoPreview />
        <Timeline />
      </div>
      <aside class="flex min-h-0 flex-col gap-2 overflow-y-auto">
        <PresetPanel />
        <Inspector />
        <SegmentList />
      </aside>
    </main>
  {/if}

  <StatusBar />
</div>
