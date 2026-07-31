/** Human-facing flow types for the MVP shell (not backend enums). */

export type ProjectStage =
  | "source_required"
  | "source_ready"
  | "processing"
  | "candidates_ready"
  | "adjusting"
  | "rendering"
  | "completed"
  | "interrupted"
  | "failed";

export type FlowRoute =
  | "projects"
  | "create"
  | "processing"
  | "candidates"
  | "edit"
  | "rendering"
  | "result"
  | "empty_candidates"
  | "interrupted"
  | "failed";

export interface FlowJobView {
  id: string;
  kind: string;
  status: string;
  progressPct: number;
  resultArtifactId?: string | null;
  updatedAt?: string;
  errorJson?: string | null;
}

export interface FlowCandidateView {
  id: string;
  status: string;
  start: number;
  end: number;
  duration: number;
  score: number;
  title: string;
  transcript: string;
  summary?: string;
  reasons?: { label?: string; code?: string }[];
  sourceMediaPath: string;
  framing: {
    mode: string;
    centerX: number;
    centerY: number;
    zoom: number;
    outputWidth: number;
    outputHeight: number;
  };
}

export interface FlowArtifactView {
  id: string;
  kind: string;
  role: string;
  path: string;
  validationStatus: string;
  byteSize: number;
  mimeType: string;
  createdByJobId: string;
}

export interface FlowProjectView {
  id: string;
  title: string;
  sourceMediaPath: string;
  updatedAt: string;
  createdAt: string;
}

export interface DeriveStageInput {
  project: FlowProjectView | null;
  jobs: FlowJobView[];
  candidates: FlowCandidateView[];
  /** True if any approve decision exists (or candidate status approved). */
  hasApprovedCandidate: boolean;
  artifacts: FlowArtifactView[];
}

export interface UserProgress {
  label: string;
  percent: number | null;
  detail: string;
  indeterminate: boolean;
}
