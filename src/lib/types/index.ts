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
}

export interface SilenceDetectionOptions {
  minSilenceDuration: number;
  padding: number;
  threshold: number;
  preferSilero: boolean;
  autoCutSilence: boolean;
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
