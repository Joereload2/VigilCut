/** Shared domain types mirroring the Rust backend (camelCase JSON). */

export type SegmentKind =
  | "speech"
  | "silence"
  | "music"
  | "noise"
  | "clip_candidate"
  | "manual";

export type SegmentDecision = "keep" | "cut" | "pending";

export type ProjectMode = "silence_cut" | "clip_select" | "full";

export interface Segment {
  id: string;
  start: number;
  end: number;
  kind: SegmentKind;
  decision: SegmentDecision;
  confidence: number;
  label?: string | null;
  energyDb?: number | null;
  eventId?: string | null;
  autoApplied?: boolean;
  needsReview?: boolean;
}

export interface Span {
  start: number;
  end: number;
}

export interface AnalysisEvent {
  id: string;
  runId: string;
  type: string;
  detector: string;
  span: Span;
  score: number;
  payload: unknown;
  tags: string[];
}

export interface ExceptionItem {
  id: string;
  eventIds: string[];
  reason: "low_confidence" | "policy_conflict" | "duration_edge";
  span: Span;
  confidence: number;
  suggestedOp: "remove_span" | "keep_span";
  rationale: string;
  resolution: "pending" | "accepted" | "rejected";
}

export interface Edl {
  mediaPath: string;
  sourceDuration: number;
  videoTrack: Span[];
  outputDuration: number;
  removedDuration: number;
}

export interface AnalysisStats {
  eventCount: number;
  silenceEventCount: number;
  autoCutCount: number;
  exceptionCount: number;
  pendingExceptionCount: number;
  speechDuration: number;
  silenceDuration: number;
  autoRemovedDuration: number;
  outputDuration: number;
}

export interface PolicyConfig {
  autoApproveMinScore: number;
  minSilenceDuration: number;
  padding: number;
  threshold: number;
  preferSilero: boolean;
}

export interface AnalysisRun {
  id: string;
  mediaPath: string;
  duration: number;
  method: string;
  policy: PolicyConfig;
  events: AnalysisEvent[];
  editOps: unknown[];
  exceptions: ExceptionItem[];
  edl: Edl;
  segments: Segment[];
  stats: AnalysisStats;
}

export interface SilenceDetectionOptions {
  minSilenceDuration: number;
  padding: number;
  threshold: number;
  preferSilero: boolean;
  autoCutSilence: boolean;
  /** Min confidence for auto-cut without human review */
  autoApproveMinScore: number;
}

export interface SilenceDetectionResult {
  mediaPath: string;
  duration: number;
  segments: Segment[];
  method: string;
  speechDuration: number;
  silenceDuration: number;
  cutDuration: number;
}

export interface MediaInfo {
  path: string;
  duration: number;
  width: number;
  height: number;
  fps: number;
  videoCodec?: string | null;
  audioCodec?: string | null;
  sampleRate?: number | null;
  channels?: number | null;
  bitrate?: number | null;
  hasAudio: boolean;
  hasVideo: boolean;
  formatName?: string | null;
  sizeBytes: number;
}

export interface WaveformData {
  path: string;
  sampleRate: number;
  peaks: number[];
  duration: number;
}

export interface AudioEnhanceOptions {
  enabled: boolean;
  denoise: boolean;
  denoiseStrength: number;
  normalize: boolean;
  targetLufs: number;
  highpassHz?: number | null;
  compress: boolean;
}

export interface ColorOptions {
  enabled: boolean;
  brightness: number;
  contrast: number;
  saturation: number;
  gamma: number;
  autoLevels: boolean;
}

export interface ExportOptions {
  container: string;
  videoCodec: string;
  audioCodec: string;
  crf: number;
  preset: string;
  audioBitrateK: number;
  reencode: boolean;
  applyCuts: boolean;
}

export interface ProcessingPreset {
  id: string;
  name: string;
  description?: string | null;
  silence: SilenceDetectionOptions;
  audio: AudioEnhanceOptions;
  color: ColorOptions;
  export: ExportOptions;
  isBuiltin: boolean;
}

export interface Project {
  id: string;
  name: string;
  mediaPath: string;
  media?: MediaInfo | null;
  segments: Segment[];
  preset: ProcessingPreset;
  subtitles?: unknown;
  createdAt: string;
  updatedAt: string;
  workDir?: string | null;
  notes?: string | null;
  mode: ProjectMode;
}

export interface ProjectSummary {
  id: string;
  name: string;
  mediaPath: string;
  updatedAt: string;
  mode: ProjectMode;
}

export interface FfmpegStatus {
  available: boolean;
  ffmpegPath?: string | null;
  ffprobePath?: string | null;
  version?: string | null;
  error?: string | null;
}

export interface ExportEstimate {
  estimatedDuration: number;
  keepRanges: [number, number][];
  cutDuration: number;
  sourceDuration: number;
}

export interface PreviewSkipPlan {
  keepRanges: [number, number][];
  estimatedDuration: number;
}

export interface AppInfo {
  name: string;
  version: string;
  os: string;
}

export const DEFAULT_SILENCE_OPTIONS: SilenceDetectionOptions = {
  minSilenceDuration: 0.4,
  padding: 0.12,
  threshold: 0.5,
  preferSilero: true,
  autoCutSilence: true,
  autoApproveMinScore: 0.8,
};

export function segmentDuration(s: Segment): number {
  return Math.max(0, s.end - s.start);
}

export function formatTime(seconds: number, withMs = false): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0;
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = Math.floor(seconds % 60);
  const ms = Math.floor((seconds % 1) * 1000);
  const base =
    h > 0
      ? `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
      : `${m}:${String(s).padStart(2, "0")}`;
  return withMs ? `${base}.${String(ms).padStart(3, "0")}` : base;
}
