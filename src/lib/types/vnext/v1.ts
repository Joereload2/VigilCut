/** VigilCut vNext contract types (camelCase). No `unknown` / `any`. */

export type JobStatusV1 =
  | "queued"
  | "running"
  | "waiting_review"
  | "completed"
  | "failed"
  | "interrupted"
  | "cancelling"
  | "cancelled";

export type NextActionV1 =
  | "ingest"
  | "transcribe"
  | "generate_candidates"
  | "review"
  | "render"
  | "done"
  | "resolve_failed_job";

export interface ApplicationErrorV1 {
  contractVersion: "v1" | string;
  code: string;
  message: string;
  retryable: boolean;
  jobId?: string | null;
}

export interface JobProgressV1 {
  contractVersion: "v1" | string;
  jobId: string;
  contentProjectId: string;
  status: string;
  stage: string;
  message: string;
  percent: number;
  updatedAt: string;
}

export interface ContentProjectSnapshotV1 {
  contractVersion: "v1" | string;
  id: string;
  title: string;
  clientLabel?: string | null;
  privacyMode: string;
  status: string;
  sourceMediaPath: string;
  workDir: string;
  nextAction?: NextActionV1 | string | null;
  createdAt: string;
  updatedAt: string;
}

export interface JobSnapshotV1 {
  contractVersion: "v1" | string;
  id: string;
  contentProjectId: string;
  kind: string;
  status: JobStatusV1 | string;
  idempotencyKey: string;
  attempt: number;
  maxAttempts: number;
  stage: string;
  progressPct: number;
  resultArtifactId?: string | null;
  renderPlanId?: string | null;
  errorJson?: string | null;
  cancelRequested: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface ArtifactManifestV1 {
  contractVersion: "v1" | string;
  id: string;
  contentProjectId: string;
  kind: string;
  role: string;
  path: string;
  sha256: string;
  byteSize: number;
  mimeType: string;
  createdByJobId: string;
  validationStatus: string;
  validationNotes?: string | null;
  createdAt: string;
}

export interface SubtitleCueV1 {
  id?: string;
  startS: number;
  endS: number;
  text: string;
}

export interface CreateProjectRequestV1 {
  title?: string;
  sourceMediaPath: string;
}

export interface SaveReviewDecisionRequestV1 {
  runId: string;
  candidateId: string;
  decision: string;
  reason?: string | null;
  startS?: number | null;
  endS?: number | null;
  framing?: {
    mode: string;
    centerX: number;
    centerY: number;
    zoom: number;
    outputWidth: number;
    outputHeight: number;
    trackingReady?: boolean;
    widthFrac?: number;
    heightFrac?: number;
    panels?: {
      centerX: number;
      centerY: number;
      widthFrac: number;
      heightFrac: number;
    }[];
  } | null;
}

export interface CreateRenderPlanRequestV1 {
  runId: string;
  candidateId: string;
  burnIn?: boolean;
  cues?: SubtitleCueV1[];
}

export interface VerticalRenderPlanV1 {
  contractVersion: string;
  id: string;
  contentProjectId: string;
  recipeId: string;
  candidateId: string;
  sourceMediaPath: string;
  sourceStartS: number;
  sourceEndS: number;
  framing: SaveReviewDecisionRequestV1["framing"];
  subtitles: {
    enabled: boolean;
    presetId: string;
    cues: SubtitleCueV1[];
  };
  outputSpec: {
    width: number;
    height: number;
    videoCodec: string;
    audioCodec: string;
  };
  createdAt: string;
  createdBy: string;
}

export type ProjectSectionV1 = "summary" | "candidates" | "review" | "exports";
export type VnextNavV1 = "projects" | "queue" | "library" | "legacy";
