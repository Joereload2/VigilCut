/**
 * Workstation model — product language for the operator.
 * Stages remain technical; stations are what the operator sees.
 */
import type { FlowJobView, ProjectStage } from "$lib/vnext/types/flow";
import { jobToUserProgress } from "$lib/vnext/presentation/jobLabels";
import { nextActionHuman } from "$lib/vnext/presentation/deriveStage";

export type StationId =
  | "hub"
  | "import"
  | "analysis"
  | "review"
  | "adjust"
  | "render"
  | "result"
  | "blocked";

export interface StationGuide {
  stationId: StationId;
  /** Short station name in the header. */
  stationName: string;
  /** Step label e.g. "Paso 3 de 6". */
  stepLabel: string;
  stepIndex: number;
  stepTotal: number;
  /** One-line objective. */
  objective: string;
  /** Qué hago (operator). */
  youDo: string;
  /** Qué hace VigilCut. */
  systemDoes: string;
  /** Qué falta. */
  missing: string;
  /** Cuánto falta (human). */
  remaining: string;
  /** Qué debo hacer ahora. */
  nextDo: string;
  /** Primary CTA label; null when wait-only. */
  primaryAction: string | null;
  /** Human status chip. */
  statusLabel: string;
}

const TOTAL = 6;

/** Human names for the 6-step rail (design system). */
export const STATION_STEP_NAMES = [
  "Video",
  "Análisis",
  "Revisión",
  "Ajuste",
  "Render",
  "Listo",
] as const;

export function stageToStationId(stage: ProjectStage, isHub: boolean): StationId {
  if (isHub) return "hub";
  switch (stage) {
    case "source_required":
    case "source_ready":
      return "import";
    case "processing":
      return "analysis";
    case "candidates_ready":
      return "review";
    case "adjusting":
      return "adjust";
    case "rendering":
      return "render";
    case "completed":
      return "result";
    case "interrupted":
    case "failed":
      return "blocked";
    default:
      return "hub";
  }
}

function remainingFromJobs(jobs: FlowJobView[], mode: "analysis" | "render"): string {
  const kinds =
    mode === "analysis"
      ? ["ingest_probe", "transcribe", "generate_short_candidates", "clipping_run"]
      : ["vertical_render"];
  const active = jobs.find(
    (j) =>
      (kinds.some((k) => j.kind === k || j.kind.startsWith(k)) ||
        (mode === "analysis" && j.kind.startsWith("clipping_run"))) &&
      (j.status === "queued" || j.status === "running" || j.status === "cancelling"),
  );
  if (!active) {
    return mode === "analysis" ? "Casi listo…" : "Finalizando…";
  }
  const p = jobToUserProgress(active);
  if (p.percent != null) return `Queda ~${Math.max(1, 100 - p.percent)}% del paso actual`;
  if (active.status === "queued") return "En espera de capacidad — no hace falta hacer nada";
  return "Trabajando… no hace falta hacer nada";
}

/**
 * Build the five answers every station must show.
 */
export function buildStationGuide(args: {
  stage: ProjectStage;
  isHub: boolean;
  hasSource: boolean;
  candidateCount: number;
  hasSelection: boolean;
  jobs: FlowJobView[];
  emptyCandidates?: boolean;
}): StationGuide {
  const { stage, isHub, hasSource, candidateCount, hasSelection, jobs, emptyCandidates } =
    args;
  const stationId = stageToStationId(stage, isHub);
  const statusLabel = (() => {
    switch (stage) {
      case "source_required":
        return "Sin video";
      case "source_ready":
        return "Listo para analizar";
      case "processing":
        return "Procesando video";
      case "candidates_ready":
        return emptyCandidates ? "Sin candidatos" : "Candidatos listos";
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
  })();

  if (stationId === "hub") {
    return {
      stationId,
      stationName: "Proyectos",
      stepLabel: "Inicio",
      stepIndex: 0,
      stepTotal: TOTAL,
      objective: "Elegir o crear un proyecto para producir un Short.",
      youDo: "Abrí un proyecto o creá uno nuevo desde un video.",
      systemDoes: "Guarda el progreso de cada proyecto para que puedas retomar.",
      missing: "Un proyecto activo.",
      remaining: "—",
      nextDo: "Crear Short desde un video",
      primaryAction: "Crear Short desde un video",
      statusLabel: "Tus proyectos",
    };
  }

  if (stationId === "import") {
    return {
      stationId,
      stationName: "Importación",
      stepLabel: `Paso 1 de ${TOTAL}`,
      stepIndex: 1,
      stepTotal: TOTAL,
      objective: "Seleccionar un video de cliente.",
      youDo: hasSource
        ? "Confirmá el video y pulsá Analizar."
        : "Elegí el archivo de video del cliente.",
      systemDoes:
        "VigilCut analizará el video y propondrá fragmentos para crear un Short vertical.",
      missing: hasSource ? "Nada — podés analizar." : "Un archivo de video.",
      remaining: hasSource ? "Un clic" : "Elegir archivo",
      nextDo: hasSource ? "Analizar video" : "Elegir video",
      primaryAction: hasSource ? "Analizar video" : "Elegir video",
      statusLabel,
    };
  }

  if (stationId === "analysis") {
    return {
      stationId,
      stationName: "Análisis",
      stepLabel: `Paso 2 de ${TOTAL}`,
      stepIndex: 2,
      stepTotal: TOTAL,
      objective: "Esperar a que VigilCut prepare candidatos.",
      youDo: "Nada. Dejá esta ventana abierta o volvé más tarde.",
      systemDoes: "Prepara el video, transcribe y busca fragmentos útiles.",
      missing: "Que termine el análisis.",
      remaining: remainingFromJobs(jobs, "analysis"),
      nextDo: "Esperar",
      primaryAction: null,
      statusLabel,
    };
  }

  if (stationId === "review") {
    if (emptyCandidates || candidateCount === 0) {
      return {
        stationId,
        stationName: "Revisión",
        stepLabel: `Paso 3 de ${TOTAL}`,
        stepIndex: 3,
        stepTotal: TOTAL,
        objective: "Elegir el mejor clip.",
        youDo: "No hay clips útiles. Podés reanalizar o usar otro video.",
        systemDoes: "Ya terminó el análisis sin proponer fragmentos.",
        missing: "Al menos un clip reproducible.",
        remaining: "—",
        nextDo: "Analizar de nuevo o elegir otro video",
        primaryAction: "Analizar de nuevo",
        statusLabel,
      };
    }
    return {
      stationId,
      stationName: "Revisión",
      stepLabel: `Paso 3 de ${TOTAL}`,
      stepIndex: 3,
      stepTotal: TOTAL,
      objective: "Elegir momento, cara e imágenes del Short.",
      youDo: hasSelection
        ? "1) Momento  2) Confirmá la cara  3) Elegí B-roll o saltá sin imágenes."
        : "Elegí un momento de la lista (ordenados por interés).",
      systemDoes:
        "Propone momentos con score y razones; busca imágenes en la biblioteca según el texto.",
      missing: hasSelection
        ? "Confirmar cara + imágenes (o Sin B-roll)."
        : "Seleccionar un momento interesante.",
      remaining: `${candidateCount} momento${candidateCount === 1 ? "" : "s"}`,
      nextDo: "Usar este clip (tras las 3 capas)",
      primaryAction: "Usar este clip",
      statusLabel,
    };
  }

  if (stationId === "adjust") {
    return {
      stationId,
      stationName: "Ajuste",
      stepLabel: `Paso 4 de ${TOTAL}`,
      stepIndex: 4,
      stepTotal: TOTAL,
      objective: "Corregir inicio, final, encuadre y subtítulos.",
      youDo: "Ajustá solo lo necesario; el resto ya está listo.",
      systemDoes: "Guardará el tramo y generará el Short 9:16 al crear.",
      missing: "Tu confirmación para crear el Short.",
      remaining: "Un clic cuando estés conforme",
      nextDo: "Crear Short",
      primaryAction: "Crear Short",
      statusLabel,
    };
  }

  if (stationId === "render") {
    return {
      stationId,
      stationName: "Render",
      stepLabel: `Paso 5 de ${TOTAL}`,
      stepIndex: 5,
      stepTotal: TOTAL,
      objective: "Esperar la generación del Short.",
      youDo: "Nada. No hace falta otra acción.",
      systemDoes: "Está creando el video vertical con encuadre y subtítulos.",
      missing: "Que termine el render.",
      remaining: remainingFromJobs(jobs, "render"),
      nextDo: "Esperar",
      primaryAction: null,
      statusLabel,
    };
  }

  if (stationId === "result") {
    return {
      stationId,
      stationName: "Resultado",
      stepLabel: `Paso 6 de ${TOTAL}`,
      stepIndex: 6,
      stepTotal: TOTAL,
      objective: "Consumir el Short terminado.",
      youDo: "Reproducí, abrí la carpeta o creá otro Short.",
      systemDoes: "Ya generó y validó el archivo final.",
      missing: "Nada — el Short está listo.",
      remaining: "Listo",
      nextDo: "Ver el Short o abrir la carpeta",
      primaryAction: "Abrir carpeta",
      statusLabel,
    };
  }

  // blocked
  return {
    stationId: "blocked",
    stationName: stage === "interrupted" ? "Interrumpido" : "Error",
    stepLabel: "Atención",
    stepIndex: 0,
    stepTotal: TOTAL,
    objective: "Recuperar el flujo.",
    youDo: "Leé el mensaje y reintentá o volvé a proyectos.",
    systemDoes: "Detuvo el proceso por un fallo o interrupción.",
    missing: "Una acción de recuperación.",
    remaining: "—",
    nextDo: nextActionHuman(stage),
    primaryAction: "Reintentar",
    statusLabel,
  };
}
