import type { FlowJobView, UserProgress } from "$lib/vnext/types/flow";

/** Single place for human labels — never show raw job kinds/statuses in UI. */

export function jobTypeToHumanLabel(kind: string): string {
  switch (kind) {
    case "ingest_probe":
      return "Preparando video";
    case "transcribe":
      return "Transcribiendo";
    case "generate_short_candidates":
      return "Buscando fragmentos";
    case "vertical_render":
      return "Creando Short";
    case "clipping_run":
      return "Analizando video";
    default:
      if (kind.startsWith("clipping_run:")) return "Analizando video";
      return "Procesando";
  }
}

export function jobStateToHumanLabel(status: string): string {
  switch (status) {
    case "queued":
      return "En espera";
    case "running":
      return "En curso";
    case "waiting_review":
      return "Listo para tu revisión";
    case "completed":
      return "Terminado";
    case "failed":
      return "Error";
    case "interrupted":
      return "Interrumpido";
    case "cancelling":
      return "Cancelando";
    case "cancelled":
      return "Cancelado";
    default:
      return "En proceso";
  }
}

/**
 * Normalize progress for the operator.
 * completed → 100%; queued without progress → null (indeterminate message).
 */
export function jobToUserProgress(job: FlowJobView): UserProgress {
  const typeLabel = jobTypeToHumanLabel(job.kind);
  const stateLabel = jobStateToHumanLabel(job.status);

  if (job.status === "completed") {
    return {
      label: typeLabel,
      percent: 100,
      detail: "Listo",
      indeterminate: false,
    };
  }
  if (job.status === "failed") {
    return {
      label: typeLabel,
      percent: null,
      detail: "No se pudo completar. Podés reintentar.",
      indeterminate: false,
    };
  }
  if (job.status === "interrupted") {
    return {
      label: typeLabel,
      percent: null,
      detail: "Se interrumpió. Podés reintentar desde el proyecto.",
      indeterminate: false,
    };
  }
  if (job.status === "queued") {
    return {
      label: typeLabel,
      percent: null,
      detail: "En cola — se iniciará en cuanto haya capacidad.",
      indeterminate: true,
    };
  }
  if (job.status === "running" || job.status === "cancelling") {
    const raw = Number.isFinite(job.progressPct) ? job.progressPct : 0;
    // Always surface a % while running (never "Trabajando…" at 0% without number).
    const pct = Math.min(99, Math.max(1, raw > 0 ? raw : 1));
    return {
      label: typeLabel,
      percent: pct,
      detail: stateLabel,
      indeterminate: false,
    };
  }
  return {
    label: typeLabel,
    percent: null,
    detail: stateLabel,
    indeterminate: true,
  };
}

export function projectStageToHumanLabel(stage: string): string {
  switch (stage) {
    case "source_required":
      return "Sin video";
    case "source_ready":
      return "Listo para analizar";
    case "processing":
      return "Procesando video";
    case "candidates_ready":
      return "Candidatos listos";
    case "adjusting":
      return "Requiere ajustes";
    case "rendering":
      return "Creando Short";
    case "completed":
      return "Short terminado";
    case "interrupted":
      return "Interrumpido";
    case "failed":
      return "Error";
    default:
      return "En curso";
  }
}

export function processingStepLabel(step: 0 | 1 | 2 | 3): string {
  return [
    "Preparando video",
    "Transcribiendo",
    "Buscando fragmentos",
    "Preparando candidatos",
  ][step];
}

/** Map active jobs to a coarse processing step 0–3 for the processing screen. */
export function jobsToProcessingStep(jobs: FlowJobView[]): 0 | 1 | 2 | 3 {
  const active = jobs.filter((j) =>
    ["queued", "running", "cancelling"].includes(j.status),
  );
  if (active.some((j) => j.kind === "generate_short_candidates")) return 2;
  if (active.some((j) => j.kind === "transcribe")) return 1;
  if (active.some((j) => j.kind === "ingest_probe")) return 0;
  if (jobs.some((j) => j.kind === "generate_short_candidates" && j.status === "completed"))
    return 3;
  if (jobs.some((j) => j.kind === "transcribe" && j.status === "completed")) return 2;
  return 0;
}

export function scoreToHuman(score: number): string {
  if (score >= 80) return "Muy prometedor";
  if (score >= 65) return "Buen fragmento";
  if (score >= 50) return "Aceptable";
  return "Bajo interés";
}

export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function formatDuration(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${r.toString().padStart(2, "0")}`;
}
