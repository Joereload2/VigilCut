import { invoke } from "@tauri-apps/api/core";
import type {
  AnalysisRun,
  AppInfo,
  ClipCandidate,
  ClipFraming,
  ClippingOptions,
  ClippingRun,
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

/** exceptionMode: "safe" | "supervised" | "aggressive" — default safe */
export async function queueBatchJob(
  mediaPaths: string[],
  outputDir: string,
  exceptionMode: "safe" | "supervised" | "aggressive" = "safe",
  policyPackId = "factory",
): Promise<unknown> {
  return invoke("queue_batch_job", {
    mediaPaths,
    outputDir,
    presetId: policyPackId,
    autoAcceptExceptions: exceptionMode === "aggressive",
    exceptionMode,
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
  exceptionMode: "safe" | "supervised" | "aggressive" = "safe",
): Promise<unknown> {
  return invoke("queue_inbox_batch", {
    inboxDir,
    outputDir,
    autoAcceptExceptions: exceptionMode === "aggressive",
    exceptionMode,
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

export async function startInboxWatch(): Promise<{
  running: boolean;
  inbox: string;
  outbox: string;
  processedCount: number;
}> {
  return invoke("start_inbox_watch");
}

export async function stopInboxWatch(): Promise<void> {
  return invoke("stop_inbox_watch");
}

export async function getInboxWatchStatus(): Promise<{
  running: boolean;
  inbox: string;
  outbox: string;
  processedCount: number;
}> {
  return invoke("get_inbox_watch_status");
}

export async function processFactoryInboxNow(): Promise<unknown> {
  return invoke("process_factory_inbox_now");
}

export async function listPolicyPacks(): Promise<
  {
    id: string;
    name: string;
    description?: string | null;
    policy: {
      autoApproveMinScore: number;
      minSilenceDuration: number;
      padding: number;
      threshold: number;
      preferSilero: boolean;
    };
    cutFillers: boolean;
    exportShorts: boolean;
    isBuiltin: boolean;
  }[]
> {
  if (!isTauri()) return [];
  return invoke("list_policy_packs");
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

export async function previewSkipCuts(
  segments: Segment[],
  keepRanges?: [number, number][],
): Promise<PreviewSkipPlan> {
  return invoke("preview_skip_cuts", {
    segments,
    keepRanges: keepRanges ?? null,
  });
}

export async function estimateExport(
  segments: Segment[],
  sourceDuration: number,
  keepRanges?: [number, number][],
): Promise<ExportEstimate> {
  return invoke("estimate_export", {
    segments,
    keepRanges: keepRanges ?? null,
    sourceDuration,
  });
}

export async function exportVideo(params: {
  mediaPath: string;
  outputPath: string;
  segments?: Segment[];
  /** Preferred: EDL / factory keep ranges (source of truth). */
  keepRanges?: [number, number][];
  exportOptions?: ExportOptions;
  colorOptions?: ColorOptions;
  audioOptions?: import("$lib/types").AudioEnhanceOptions;
  hasAudio?: boolean;
}): Promise<{ outputPath: string; duration: number; keepCount: number }> {
  return invoke("export_video", {
    mediaPath: params.mediaPath,
    outputPath: params.outputPath,
    segments: params.segments ?? null,
    keepRanges: params.keepRanges ?? null,
    exportOptions: params.exportOptions ?? null,
    colorOptions: params.colorOptions ?? null,
    audioOptions: params.audioOptions ?? null,
    hasAudio: params.hasAudio ?? null,
  });
}

export async function cancelJob(): Promise<void> {
  if (!isTauri()) return;
  return invoke("cancel_job");
}

export async function listPresets(): Promise<ProcessingPreset[]> {
  if (!isTauri()) return [];
  return invoke("list_presets");
}

export async function savePreset(preset: ProcessingPreset): Promise<ProcessingPreset> {
  return invoke("save_preset", { preset });
}

// ── Intelligent clipping ───────────────────────────────────────────────────

export async function runClipping(
  mediaPath: string,
  options?: ClippingOptions | null,
  analysisRunId?: string | null,
): Promise<ClippingRun> {
  return invoke("run_clipping", {
    mediaPath,
    options: options ?? null,
    analysisRunId: analysisRunId ?? null,
  });
}

export async function getClippingRun(runId: string): Promise<ClippingRun> {
  return invoke("get_clipping_run", { runId });
}

export async function updateClipStatus(
  runId: string,
  candidateId: string,
  status: string,
): Promise<ClipCandidate> {
  return invoke("update_clip_status", { runId, candidateId, status });
}

export async function updateClipSpan(
  runId: string,
  candidateId: string,
  start: number,
  end: number,
): Promise<ClipCandidate> {
  return invoke("update_clip_span", { runId, candidateId, start, end });
}

export async function updateClipFraming(
  runId: string,
  candidateId: string,
  framing: ClipFraming,
): Promise<ClipCandidate> {
  return invoke("update_clip_framing", { runId, candidateId, framing });
}

export async function bulkClipStatus(
  runId: string,
  status: string,
  onlyHighConfidence: boolean,
): Promise<ClippingRun> {
  return invoke("bulk_clip_status", {
    runId,
    status,
    onlyHighConfidence,
  });
}

export async function promoteClipVariant(
  runId: string,
  candidateId: string,
): Promise<ClippingRun> {
  return invoke("promote_clip_variant", { runId, candidateId });
}

export async function exportClips(params: {
  runId: string;
  outputDir: string;
  candidateIds?: string[];
  framingOverride?: ClipFraming | null;
}): Promise<{
  results: { candidateId: string; ok: boolean; outputPath?: string; error?: string }[];
  outputDir: string;
  run: ClippingRun;
}> {
  return invoke("export_clips", {
    runId: params.runId,
    outputDir: params.outputDir,
    candidateIds: params.candidateIds ?? null,
    framingOverride: params.framingOverride ?? null,
  });
}

// ── Visual library + transcript enrichment ───────────────────────────────

export async function visualRunEnrichment(
  mediaPath: string,
  analysisRunId?: string | null,
  transcriptPath?: string | null,
  preferWhisper = false,
): Promise<unknown> {
  return invoke("visual_run_enrichment", {
    mediaPath,
    analysisRunId: analysisRunId ?? null,
    transcriptPath: transcriptPath ?? null,
    preferWhisper,
  });
}

export async function visualTranscribeWhisper(
  mediaPath: string,
  analysisRunId?: string | null,
): Promise<unknown> {
  return invoke("visual_transcribe_whisper", {
    mediaPath,
    analysisRunId: analysisRunId ?? null,
  });
}

export async function visualWhisperStatus(): Promise<{
  available: boolean;
  kind: string;
  detail: string;
  installHint: string;
}> {
  return invoke("visual_whisper_status");
}

export async function visualInstallWhisper(): Promise<string> {
  return invoke("visual_install_whisper");
}

export async function visualListAssets(query?: string | null, limit = 100): Promise<unknown> {
  return invoke("visual_list_assets", { query: query ?? null, limit });
}

// ── Intelligent visual library ───────────────────────────────────────────

export async function visualSeedThemeEconomy(): Promise<unknown> {
  return invoke("visual_seed_theme_economy");
}

export async function visualListConcepts(themeId?: string | null, limit = 100): Promise<unknown> {
  return invoke("visual_list_concepts", { themeId: themeId ?? null, limit });
}

export async function visualDetectNeeds(params: {
  mediaPath: string;
  analysisRunId?: string | null;
  maxNeeds?: number;
}): Promise<unknown> {
  return invoke("visual_detect_needs", {
    mediaPath: params.mediaPath,
    analysisRunId: params.analysisRunId ?? null,
    maxNeeds: params.maxNeeds ?? 24,
  });
}

export async function visualListNeeds(projectKey: string): Promise<unknown> {
  return invoke("visual_list_needs", { projectKey });
}

export async function visualCoverage(projectKey: string): Promise<unknown> {
  return invoke("visual_coverage", { projectKey });
}

export async function visualCoverNeeds(params: {
  projectKey: string;
  generateMissing?: boolean;
  maxGenerate?: number;
}): Promise<unknown> {
  return invoke("visual_cover_needs", {
    projectKey: params.projectKey,
    generateMissing: params.generateMissing ?? false,
    maxGenerate: params.maxGenerate ?? 5,
  });
}

export async function visualApplyNeedsToPlan(params: {
  mediaPath: string;
  analysisRunId?: string | null;
  projectKey?: string | null;
}): Promise<unknown> {
  return invoke("visual_apply_needs_to_plan", {
    mediaPath: params.mediaPath,
    analysisRunId: params.analysisRunId ?? null,
    projectKey: params.projectKey ?? null,
  });
}

export async function visualListReviewQueue(limit = 50): Promise<unknown> {
  return invoke("visual_list_review_queue", { limit });
}

export async function visualApproveCandidate(candidateId: string): Promise<unknown> {
  return invoke("visual_approve_candidate", { candidateId });
}

export async function visualProbeImageProvider(): Promise<unknown> {
  return invoke("visual_probe_image_provider");
}

export async function visualCostPolicy(): Promise<unknown> {
  return invoke("visual_cost_policy");
}

export async function visualLibraryDashboard(): Promise<unknown> {
  return invoke("visual_library_dashboard");
}

export async function visualLibraryConceptCoverage(limit = 100): Promise<unknown> {
  return invoke("visual_library_concept_coverage", { limit });
}

export type CreateLibraryRequestInput = {
  origin: "manual" | "daily_feed" | "broll_missing" | "visual_video_missing" | "import";
  theme: string;
  title: string;
  description: string;
  prompt: string;
  negativePrompt: string;
  targetCount: number;
  desiredFormat: "16:9" | "9:16" | "1:1" | "4:5";
  width: number;
  height: number;
  style: "photorealistic" | "illustration" | "infographic" | "cinematic" | "other";
  positiveContexts: string[];
  negativeContexts: string[];
  hardExclusions: string[];
  priority: number;
};
export async function visualLibraryCreateRequest(
  input: CreateLibraryRequestInput,
): Promise<unknown> {
  return invoke("visual_library_create_request", { input });
}

export async function visualLibraryPreviewRequest(requestId: string): Promise<unknown> {
  return invoke("visual_library_preview_request", { requestId });
}

export async function visualLibraryConfirmRequest(requestId: string): Promise<unknown> {
  return invoke("visual_library_confirm_request", { requestId });
}

export async function visualLibraryUseExisting(requestId: string, assetId: string): Promise<unknown> {
  return invoke("visual_library_use_existing", { requestId, assetId });
}
export async function visualLibraryRegenerateRequest(
  requestId: string,
  prompt: string,
  negativePrompt: string,
): Promise<unknown> {
  return invoke("visual_library_regenerate_request", { requestId, prompt, negativePrompt });
}
export async function visualLibraryListRequests(limit = 30): Promise<unknown> {
  return invoke("visual_library_list_requests", { limit });
}

export async function visualLibraryCancelRequest(requestId: string): Promise<unknown> {
  return invoke("visual_library_cancel_request", { requestId });
}

export async function visualSkipNeed(needId: string): Promise<unknown> {
  return invoke("visual_skip_need", { needId });
}

export async function visualSupervision(projectKey: string): Promise<unknown> {
  return invoke("visual_supervision", { projectKey });
}

export async function visualGenerateNeed(needId: string): Promise<unknown> {
  return invoke("visual_generate_need", { needId });
}

export async function visualSearchLibraryForNeed(needId: string): Promise<unknown> {
  return invoke("visual_search_library_for_need", { needId });
}

export async function visualAssignNeedAsset(needId: string, assetId: string): Promise<unknown> {
  return invoke("visual_assign_need_asset", { needId, assetId });
}

/** Single write: assign scene asset + create/replace one placement (PM-003). */
export async function visualUseAssetForNeed(params: {
  needId: string;
  assetId: string;
  mediaPath: string;
  analysisRunId?: string | null;
}): Promise<unknown> {
  return invoke("visual_use_asset_for_need", {
    needId: params.needId,
    assetId: params.assetId,
    mediaPath: params.mediaPath,
    analysisRunId: params.analysisRunId ?? null,
  });
}

export async function visualCancelJob(jobId: string): Promise<unknown> {
  return invoke("visual_cancel_job", { jobId });
}

export async function visualRegenerateNeed(needId: string): Promise<unknown> {
  return invoke("visual_regenerate_need", { needId });
}

export async function visualApproveAndUse(params: {
  candidateId: string;
  mediaPath?: string | null;
  analysisRunId?: string | null;
  place?: boolean;
}): Promise<unknown> {
  return invoke("visual_approve_and_use", {
    candidateId: params.candidateId,
    mediaPath: params.mediaPath ?? null,
    analysisRunId: params.analysisRunId ?? null,
    place: params.place ?? true,
  });
}

export async function visualRejectCandidate(
  candidateId: string,
  reason?: string | null,
): Promise<unknown> {
  return invoke("visual_reject_candidate", {
    candidateId,
    reason: reason ?? null,
  });
}


export async function visualSupervisionGlobal(): Promise<unknown> {
  return invoke("visual_supervision_global");
}

export async function visualDailyFeedSettings(): Promise<unknown> {
  return invoke("visual_daily_feed_settings");
}

export async function visualDailyFeedSetEnabled(enabled: boolean): Promise<unknown> {
  return invoke("visual_daily_feed_set_enabled", { enabled });
}

export async function visualDailyFeedCycle(): Promise<unknown> {
  return invoke("visual_daily_feed_cycle");
}

export async function visualDailyWeekSummary(): Promise<unknown> {
  return invoke("visual_daily_week_summary");
}

export async function visualImportImage(
  path: string,
  title?: string | null,
  tags: string[] = [],
  concepts: string[] = [],
): Promise<unknown> {
  return invoke("visual_import_image", { path, title: title ?? null, tags, concepts });
}

export async function visualUpdateAsset(params: {
  id: string;
  title?: string | null;
  tags?: string[] | null;
  concepts?: string[] | null;
  license?: string | null;
  status?: string | null;
}): Promise<unknown> {
  return invoke("visual_update_asset", {
    id: params.id,
    title: params.title ?? null,
    tags: params.tags ?? null,
    concepts: params.concepts ?? null,
    license: params.license ?? null,
    status: params.status ?? null,
  });
}

/** Import image and attach as accepted VisualPlan placement at a transcript moment. */
export async function visualAttachImage(params: {
  mediaPath: string;
  analysisRunId?: string | null;
  path: string;
  concept: string;
  sourceStart: number;
  sourceEnd: number;
}): Promise<unknown> {
  return invoke("visual_attach_image", {
    mediaPath: params.mediaPath,
    analysisRunId: params.analysisRunId ?? null,
    path: params.path,
    concept: params.concept,
    sourceStart: params.sourceStart,
    sourceEnd: params.sourceEnd,
  });
}

/** Manual placement on output timeline — transcript optional. */
export async function visualCreateManualPlacement(params: {
  mediaPath: string;
  analysisRunId?: string | null;
  assetId?: string | null;
  imagePath?: string | null;
  outputStart: number;
  outputEnd: number;
  displayMode?: string;
  positionX?: number | null;
  positionY?: number | null;
  sizeW?: number | null;
  fit?: string | null;
  label?: string | null;
  sourceDuration?: number | null;
}): Promise<unknown> {
  return invoke("visual_create_manual_placement", {
    mediaPath: params.mediaPath,
    analysisRunId: params.analysisRunId ?? null,
    assetId: params.assetId ?? null,
    imagePath: params.imagePath ?? null,
    outputStart: params.outputStart,
    outputEnd: params.outputEnd,
    displayMode: params.displayMode ?? "completa",
    positionX: params.positionX ?? null,
    positionY: params.positionY ?? null,
    sizeW: params.sizeW ?? null,
    fit: params.fit ?? null,
    label: params.label ?? null,
    sourceDuration: params.sourceDuration ?? null,
  });
}

export async function visualUpdatePlacement(params: {
  placementId: string;
  outputStart?: number | null;
  outputEnd?: number | null;
  displayMode?: string | null;
  positionX?: number | null;
  positionY?: number | null;
  sizeW?: number | null;
  sizeH?: number | null;
  fit?: string | null;
  status?: string | null;
  reviewStatus?: string | null;
  manualOverride?: boolean | null;
  relatedText?: string | null;
  restoreAi?: boolean | null;
  opacity?: number | null;
}): Promise<unknown> {
  return invoke("visual_update_placement", {
    placementId: params.placementId,
    outputStart: params.outputStart ?? null,
    outputEnd: params.outputEnd ?? null,
    displayMode: params.displayMode ?? null,
    positionX: params.positionX ?? null,
    positionY: params.positionY ?? null,
    sizeW: params.sizeW ?? null,
    sizeH: params.sizeH ?? null,
    fit: params.fit ?? null,
    status: params.status ?? null,
    reviewStatus: params.reviewStatus ?? null,
    manualOverride: params.manualOverride ?? null,
    relatedText: params.relatedText ?? null,
    restoreAi: params.restoreAi ?? null,
    opacity: params.opacity ?? null,
  });
}

export async function visualSnapPlacement(params: {
  placementId: string;
  outputStart: number;
  outputEnd: number;
  anchors: number[];
  threshold?: number | null;
}): Promise<unknown> {
  return invoke("visual_snap_placement", {
    placementId: params.placementId,
    outputStart: params.outputStart,
    outputEnd: params.outputEnd,
    anchors: params.anchors,
    threshold: params.threshold ?? null,
  });
}

export async function visualEvaluateComposition(): Promise<unknown> {
  return invoke("visual_evaluate_composition");
}

export async function visualRemovePlacement(placementId: string): Promise<unknown> {
  return invoke("visual_remove_placement", { placementId });
}

export async function visualAddProtectedRange(params: {
  mediaPath: string;
  analysisRunId?: string | null;
  outputStart: number;
  outputEnd: number;
  reason?: string | null;
  sourceDuration?: number | null;
}): Promise<unknown> {
  return invoke("visual_add_protected_range", {
    mediaPath: params.mediaPath,
    analysisRunId: params.analysisRunId ?? null,
    outputStart: params.outputStart,
    outputEnd: params.outputEnd,
    reason: params.reason ?? null,
    sourceDuration: params.sourceDuration ?? null,
  });
}

export async function visualRemoveProtectedRange(rangeId: string): Promise<unknown> {
  return invoke("visual_remove_protected_range", { rangeId });
}

export async function visualImportFolder(
  path: string,
  tags: string[] = [],
  concepts: string[] = [],
  recursive = false,
): Promise<unknown> {
  return invoke("visual_import_folder", { path, tags, concepts, recursive });
}

export async function visualSetSuggestionStatus(
  suggestionId: string,
  status: string,
): Promise<unknown> {
  return invoke("visual_set_suggestion_status", { suggestionId, status });
}

export async function visualGetSession(): Promise<unknown> {
  return invoke("visual_get_session");
}

export async function visualExportTranscript(
  outDir: string,
  stem?: string | null,
): Promise<unknown> {
  return invoke("visual_export_transcript", { outDir, stem: stem ?? null });
}

export async function visualSavePlan(path?: string | null): Promise<string> {
  return invoke("visual_save_plan", { path: path ?? null });
}

export async function visualListUsage(
  assetId?: string | null,
  limit = 50,
): Promise<unknown> {
  return invoke("visual_list_usage", { assetId: assetId ?? null, limit });
}

export async function visualScanMissing(): Promise<number> {
  return invoke("visual_scan_missing");
}

export async function visualRenderPlan(
  cutVideoPath: string,
  outputPath: string,
  mediaPath: string,
): Promise<string> {
  return invoke("visual_render_plan", { cutVideoPath, outputPath, mediaPath });
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
export async function librarySyncStatus(): Promise<unknown> {
  return invoke("library_sync_status");
}

export async function librarySyncHealthCheck(): Promise<unknown> {
  return invoke("library_sync_health_check");
}

export async function librarySyncEnqueueAsset(assetId: string): Promise<unknown> {
  return invoke("library_sync_enqueue_asset", { assetId });
}

export async function librarySyncRunOnce(): Promise<unknown> {
  return invoke("library_sync_run_once");
}