import { invoke } from "@tauri-apps/api/core";
import type {
  AnalysisRun,
  AppInfo,
  ExportEstimate,
  FfmpegStatus,
  MediaInfo,
  PreviewSkipPlan,
  ProcessingPreset,
  Project,
  ProjectSummary,
  Segment,
  SegmentDecision,
  SilenceDetectionOptions,
  SilenceDetectionResult,
  WaveformData,
  ExportOptions,
  ColorOptions,
  PolicyConfig,
} from "$lib/types";

/** Detect browser-only Vite preview (no Tauri runtime). */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function getAppInfo(): Promise<AppInfo> {
  if (!isTauri()) return { name: "VigilCut", version: "0.1.0", os: "web" };
  return invoke("get_app_info");
}

export async function checkFfmpeg(): Promise<FfmpegStatus> {
  if (!isTauri()) {
    return { available: false, error: "Not running inside Tauri" };
  }
  return invoke("check_ffmpeg");
}

export async function probeMedia(path: string): Promise<MediaInfo> {
  return invoke("probe_media", { path });
}

export async function extractWaveform(
  path: string,
  peaksPerSecond = 100,
): Promise<WaveformData> {
  return invoke("extract_waveform", { path, peaksPerSecond });
}

export async function detectSilences(
  path: string,
  options?: SilenceDetectionOptions,
): Promise<SilenceDetectionResult> {
  return invoke("detect_silences", { path, options: options ?? null });
}

/** Engine analysis: events + policy + EDL + exceptions + segment projection. */
export async function runAnalysis(
  path: string,
  options?: SilenceDetectionOptions,
  policy?: PolicyConfig,
): Promise<AnalysisRun> {
  return invoke("run_analysis", {
    path,
    options: options ?? null,
    policy: policy ?? null,
  });
}

export async function resolveAnalysisException(
  runId: string,
  exceptionId: string,
  resolution: "accepted" | "rejected",
): Promise<AnalysisRun> {
  return invoke("resolve_analysis_exception", {
    runId,
    request: { exceptionId, resolution },
  });
}

export async function resolveAllExceptions(
  runId: string,
  accept: boolean,
): Promise<AnalysisRun> {
  return invoke("resolve_all_exceptions", { runId, accept });
}

export async function queueBatchJob(
  mediaPaths: string[],
  outputDir: string,
  autoAcceptExceptions = true,
): Promise<unknown> {
  return invoke("queue_batch_job", {
    mediaPaths,
    outputDir,
    presetId: "factory",
    autoAcceptExceptions,
    options: null,
  });
}

export async function getBatchStatus(id: string): Promise<unknown> {
  return invoke("get_batch_status", { id });
}

export async function listBatchJobs(): Promise<unknown[]> {
  if (!isTauri()) return [];
  return invoke("list_batch_jobs");
}

export async function queueInboxBatch(
  inboxDir: string,
  outputDir: string | null = null,
  autoAcceptExceptions = true,
): Promise<unknown> {
  return invoke("queue_inbox_batch", {
    inboxDir,
    outputDir,
    autoAcceptExceptions,
  });
}

export async function getFactoryPaths(): Promise<{
  appData: string;
  inbox: string;
  outbox: string;
  exports: string;
  models: string;
  cache: string;
}> {
  return invoke("get_factory_paths");
}

export async function writeExportArtifacts(
  runId: string,
  outputPath: string,
): Promise<unknown[]> {
  return invoke("write_export_artifacts", { runId, outputPath });
}

export async function openFactoryFolder(which: string): Promise<string> {
  return invoke("open_factory_folder", { which });
}

export async function createProject(name: string, mediaPath: string): Promise<Project> {
  return invoke("create_project", { name, mediaPath });
}

export async function saveProject(project: Project): Promise<Project> {
  return invoke("save_project", { project });
}

export async function loadProject(id: string): Promise<Project> {
  return invoke("load_project", { id });
}

export async function listRecentProjects(): Promise<ProjectSummary[]> {
  if (!isTauri()) return [];
  return invoke("list_recent_projects");
}

export async function applySegmentEdits(
  segments: Segment[],
  edits: { id: string; decision?: SegmentDecision; start?: number; end?: number; label?: string }[],
): Promise<Segment[]> {
  return invoke("apply_segment_edits", { segments, edits });
}

export async function splitSegmentAt(
  segments: Segment[],
  segmentId: string,
  time: number,
): Promise<Segment[]> {
  return invoke("split_segment_at", { segments, segmentId, time });
}

export async function mergeAdjacentSegments(
  segments: Segment[],
  maxGap?: number,
): Promise<Segment[]> {
  return invoke("merge_adjacent_segments", { segments, maxGap: maxGap ?? null });
}

export async function previewSkipCuts(segments: Segment[]): Promise<PreviewSkipPlan> {
  return invoke("preview_skip_cuts", { segments });
}

export async function estimateExport(
  segments: Segment[],
  sourceDuration: number,
): Promise<ExportEstimate> {
  return invoke("estimate_export", { segments, sourceDuration });
}

export async function exportVideo(params: {
  mediaPath: string;
  outputPath: string;
  segments: Segment[];
  exportOptions?: ExportOptions;
  colorOptions?: ColorOptions;
  hasAudio?: boolean;
}): Promise<{ outputPath: string; duration: number; keepCount: number }> {
  return invoke("export_video", {
    mediaPath: params.mediaPath,
    outputPath: params.outputPath,
    segments: params.segments,
    exportOptions: params.exportOptions ?? null,
    colorOptions: params.colorOptions ?? null,
    hasAudio: params.hasAudio ?? null,
  });
}

export async function listPresets(): Promise<ProcessingPreset[]> {
  if (!isTauri()) return [];
  return invoke("list_presets");
}

export async function savePreset(preset: ProcessingPreset): Promise<ProcessingPreset> {
  return invoke("save_preset", { preset });
}

/** Demo segments for Vite-only UI work without backend. */
export function demoSegments(duration = 60): Segment[] {
  const segs: Segment[] = [];
  let t = 0;
  let i = 0;
  while (t < duration) {
    const speech = 3 + (i % 3) * 1.5;
    const silence = 0.6 + (i % 2) * 0.5;
    segs.push({
      id: `demo-s-${i}`,
      start: t,
      end: Math.min(t + speech, duration),
      kind: "speech",
      decision: "keep",
      confidence: 0.95,
      autoApplied: false,
      needsReview: false,
    });
    t += speech;
    if (t >= duration) break;
    const conf = i % 3 === 0 ? 0.72 : 0.9;
    const auto = conf >= 0.8;
    segs.push({
      id: `demo-z-${i}`,
      start: t,
      end: Math.min(t + silence, duration),
      kind: "silence",
      decision: auto ? "cut" : "pending",
      confidence: conf,
      autoApplied: auto,
      needsReview: !auto,
      label: auto ? "auto" : "revisar",
    });
    t += silence;
    i++;
  }
  return segs;
}
