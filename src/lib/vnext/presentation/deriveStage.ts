import type {
  DeriveStageInput,
  FlowArtifactView,
  FlowJobView,
  FlowRoute,
  ProjectStage,
} from "$lib/vnext/types/flow";

function isActiveJob(j: FlowJobView): boolean {
  return j.status === "queued" || j.status === "running" || j.status === "cancelling";
}

function isAnalysisJob(j: FlowJobView): boolean {
  return (
    j.kind === "ingest_probe" ||
    j.kind === "transcribe" ||
    j.kind === "generate_short_candidates" ||
    j.kind.startsWith("clipping_run")
  );
}

function isRenderJob(j: FlowJobView): boolean {
  return j.kind === "vertical_render";
}

function hasValidFinalArtifact(artifacts: FlowArtifactView[]): boolean {
  return artifacts.some(
    (a) =>
      (a.kind === "vertical_mp4" || a.role === "final_deliverable") &&
      a.validationStatus === "passed" &&
      !!a.path,
  );
}

/** Momentos que el operador aún puede marcar (pre-selección del análisis). */
export function isPickableCandidateStatus(status: string): boolean {
  return (
    status === "suggested" ||
    status === "preselected" ||
    status === "proposed" ||
    status === "pending" ||
    status === "new"
  );
}

/** Aprobados / en edición listos para encuadre o export (no terminados). */
export function isPendingExportStatus(status: string): boolean {
  return status === "approved" || status === "modified" || status === "exporting";
}

/**
 * Single pure function: durable snapshot → operator stage.
 * UI routes must derive from this — never a second independent stage store.
 *
 * Product order:
 * 1) render en curso / fallido
 * 2) momentos aún pickable (preselected/suggested) → Parte 1 (gana sobre approved parciales)
 * 3) solo aprobados pendientes de encuadre → Parte 2 adjusting
 * 4) artefacto final → completed
 */
export function deriveProjectStage(input: DeriveStageInput): ProjectStage {
  const { project, jobs, candidates, hasApprovedCandidate, artifacts } = input;

  if (!project || !project.sourceMediaPath?.trim()) {
    return "source_required";
  }

  const renderJobs = jobs.filter(isRenderJob);
  const activeRender = renderJobs.find(isActiveJob);
  if (activeRender) return "rendering";

  const failedRender = renderJobs.find((j) => j.status === "failed");
  if (failedRender) return "failed";

  const interruptedRender = renderJobs.find((j) => j.status === "interrupted");
  if (interruptedRender) return "interrupted";

  // Completed render job without artifact = integrity failure surface
  const completedRenderNoArt = renderJobs.find(
    (j) => j.status === "completed" && !j.resultArtifactId && !hasValidFinalArtifact(artifacts),
  );
  if (completedRenderNoArt) return "failed";

  const needsPick = candidates.some((c) => isPickableCandidateStatus(c.status));
  const pendingExport = candidates.some((c) => isPendingExportStatus(c.status));

  // Parte 1 PRIMERO: mientras quede algo por revisar, no saltar al encuadre
  if (needsPick) {
    return "candidates_ready";
  }

  // Short ya generado (prioridad sobre "sigue habiendo aprobados")
  // Si no, el operador nunca ve Resultado ni la ruta del archivo.
  if (hasValidFinalArtifact(artifacts)) {
    return "completed";
  }

  // Parte 2: solo cuando ya no hay pickable y hay aprobados pendientes de export
  if (pendingExport) {
    return "adjusting";
  }

  // Aprobado vía flag de sesión sin status aún reflejado
  if (hasApprovedCandidate) {
    return "adjusting";
  }

  if (candidates.length > 0) {
    return "candidates_ready";
  }

  const analysisJobs = jobs.filter(isAnalysisJob);
  if (analysisJobs.some(isActiveJob)) {
    return "processing";
  }
  if (analysisJobs.some((j) => j.status === "queued" || j.status === "running")) {
    return "processing";
  }

  if (analysisJobs.some((j) => j.status === "failed") && candidates.length === 0) {
    return "failed";
  }
  if (analysisJobs.some((j) => j.status === "interrupted") && candidates.length === 0) {
    return "interrupted";
  }

  const analysisCompleted = analysisJobs.some((j) => j.status === "completed");
  if (analysisCompleted && candidates.length === 0) {
    return "candidates_ready";
  }

  return "source_ready";
}

/** Map stage → primary flow route for the shell. */
export function stageToRoute(
  stage: ProjectStage,
  opts?: { hasCandidates?: boolean },
): FlowRoute {
  switch (stage) {
    case "source_required":
      return "create";
    case "source_ready":
      return "create";
    case "processing":
      return "processing";
    case "candidates_ready":
      return opts?.hasCandidates === false ? "empty_candidates" : "candidates";
    case "adjusting":
      return "edit";
    case "rendering":
      return "rendering";
    case "completed":
      return "result";
    case "interrupted":
      return "interrupted";
    case "failed":
      return "failed";
    default:
      return "projects";
  }
}

export function nextActionHuman(stage: ProjectStage): string {
  switch (stage) {
    case "source_ready":
      return "Analizar video";
    case "processing":
      return "Esperar análisis";
    case "candidates_ready":
      return "Elegir momentos";
    case "adjusting":
      return "Encuadre 9:16 y exportar";
    case "rendering":
      return "Esperar el Short";
    case "completed":
      return "Ver resultado";
    case "interrupted":
      return "Reintentar";
    case "failed":
      return "Revisar error";
    default:
      return "Continuar";
  }
}
