/** Types for Image Generation supervision panel (camelCase from Rust). */

export type UiState =
  | "uncovered"
  | "queued"
  | "processing"
  | "cancelling"
  | "reviewing"
  | "needs_human_review"
  | "approved"
  | "rejected"
  | "failed"
  | "cancelled"
  | "skipped";

export interface CoverageSummary {
  total?: number;
  reused?: number;
  generated?: number;
  waiting?: number;
  needsReview?: number;
  uncovered?: number;
  failed?: number;
  skipped?: number;
}

export interface VisualNeedDto {
  id: string;
  projectKey: string;
  label: string;
  terms?: string[];
  requiredContexts?: string[];
  forbiddenContexts?: string[];
  hardExclusions?: string[];
  desiredAspect?: string;
  approxDurationSecs?: number;
  sourceStart?: number | null;
  sourceEnd?: number | null;
  outputStart?: number | null;
  outputEnd?: number | null;
  coverage?: string;
  matchedAssetId?: string | null;
  matchScore?: number | null;
  matchReasons?: string[];
  generationJobId?: string | null;
}

export interface JobView {
  id: string;
  needId?: string | null;
  status: string;
  stage: string;
  provider?: string | null;
  model?: string | null;
  prompt: string;
  negativePrompt: string;
  attempt: number;
  maxAttempts: number;
  lastError?: string | null;
  isPaid: boolean;
  costKind: string;
  freeVerified: boolean;
  promptStrategy?: string | null;
  origin: string;
  cancelRequested: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CandidateView {
  id: string;
  jobId: string;
  needId?: string | null;
  localPath?: string | null;
  status: string;
  technicalScore?: number | null;
  semanticScore?: number | null;
  qaDecision?: string | null;
  qaReason?: string | null;
  approvedAssetId?: string | null;
  origin: string;
  rejectReason?: string | null;
  width?: number | null;
  height?: number | null;
  mimeType?: string | null;
  costKind?: string | null;
  freeVerified: boolean;
  provider?: string | null;
  model?: string | null;
  createdAt: string;
  updatedAt: string;
  fileExists: boolean;
  conceptTitle?: string | null;
  needLabel?: string | null;
  themeTitle?: string | null;
}

export interface NeedSupervision {
  need: VisualNeedDto;
  job?: JobView | null;
  candidate?: CandidateView | null;
  uiState: string;
  uiLabel: string;
  primaryAction: string;
}

export interface SupervisionSnapshot {
  projectKey: string;
  coverage: CoverageSummary;
  needs: NeedSupervision[];
  pendingReview: CandidateView[];
  dailyFeed?: {
    enabled?: boolean;
    maxPerDay?: number;
    intervalMinutes?: number;
  };
}

export function costLabel(kind?: string | null, freeVerified?: boolean): string {
  switch (kind) {
    case "free_verified":
      return "Gratis verificado";
    case "free_configured":
      return freeVerified ? "Gratis verificado" : "Gratis configurado, no verificado";
    case "local":
      return "Simulación local (mock)";
    case "paid":
      return "Pagado";
    default:
      return "Coste desconocido";
  }
}

/** Mock is a synthetic fixture — never market as real local AI (Codex MED-002). */
export function isMockProvider(provider?: string | null): boolean {
  return (provider ?? "").toLowerCase() === "mock";
}

export function stateColor(state: string): string {
  switch (state) {
    case "approved":
      return "text-emerald-400 border-emerald-800/50 bg-emerald-950/30";
    case "needs_human_review":
    case "reviewing":
      return "text-amber-300 border-amber-800/50 bg-amber-950/30";
    case "processing":
    case "queued":
    case "cancelling":
      return "text-sky-300 border-sky-800/50 bg-sky-950/30";
    case "failed":
      return "text-red-300 border-red-900/40 bg-red-950/30";
    case "cancelled":
    case "skipped":
      return "text-surface-400 border-surface-700 bg-surface-900/50";
    default:
      return "text-surface-300 border-surface-700 bg-surface-900/40";
  }
}

export function formatTimeRange(s?: number | null, e?: number | null): string {
  if (s == null || e == null) return "—";
  const f = (t: number) => {
    const m = Math.floor(t / 60);
    const sec = Math.floor(t % 60);
    return `${m.toString().padStart(2, "0")}:${sec.toString().padStart(2, "0")}`;
  };
  return `${f(s)}–${f(e)}`;
}
