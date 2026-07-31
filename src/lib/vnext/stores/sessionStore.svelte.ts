import type { ClipCandidate } from "$lib/types";
import type {
  ArtifactManifestV1,
  ContentProjectSnapshotV1,
  JobSnapshotV1,
} from "$lib/types/vnext/v1";
import type {
  FlowArtifactView,
  FlowCandidateView,
  FlowJobView,
  FlowProjectView,
  FlowRoute,
  ProjectStage,
} from "$lib/vnext/types/flow";
import {
  applySessionStageOverrides,
  deriveProjectStage,
  isPendingExportStatus,
  isPickableCandidateStatus,
  stageToRoute,
} from "$lib/vnext/presentation/deriveStage";
import {
  canStartDecision,
  canStartRender,
  mergeCandidatesAfterRefetch,
} from "$lib/vnext/presentation/actionGuards";
import * as api from "$lib/utils/vnext";
import { projectStore } from "$lib/vnext/stores/projectStore.svelte";
import { candidateStore } from "$lib/vnext/stores/candidateStore.svelte";
import { jobStore } from "$lib/vnext/stores/jobStore.svelte";
import { reviewStore } from "$lib/vnext/stores/reviewStore.svelte";
import { artifactStore } from "$lib/vnext/stores/artifactStore.svelte";

function mapJob(j: JobSnapshotV1): FlowJobView {
  return {
    id: j.id,
    kind: j.kind,
    status: j.status,
    progressPct: j.progressPct,
    resultArtifactId: j.resultArtifactId,
    updatedAt: j.updatedAt,
    errorJson: j.errorJson,
  };
}

function mapCand(c: ClipCandidate): FlowCandidateView {
  return {
    id: c.id,
    status: c.status,
    start: c.start,
    end: c.end,
    duration: c.duration,
    score: c.score,
    title: c.title,
    transcript: c.transcript,
    summary: c.summary,
    reasons: c.reasons,
    sourceMediaPath: c.sourceMediaPath,
    framing: {
      mode: c.framing.mode,
      centerX: c.framing.centerX,
      centerY: c.framing.centerY,
      zoom: c.framing.zoom,
      outputWidth: c.framing.outputWidth,
      outputHeight: c.framing.outputHeight,
    },
  };
}

function mapArt(a: ArtifactManifestV1): FlowArtifactView {
  return {
    id: a.id,
    kind: a.kind,
    role: a.role,
    path: a.path,
    validationStatus: a.validationStatus,
    byteSize: a.byteSize,
    mimeType: a.mimeType,
    createdByJobId: a.createdByJobId,
  };
}

/**
 * Central session for MVP shell. Snapshots from backend + ephemeral UI route.
 * Stage is always re-derived from snapshots (never a second durable source).
 */
class VnextSessionStore {
  /** Which high-level shell screen (projects list vs project flow). */
  screen = $state<"list" | "flow" | "create">("list");

  projects = $state<ContentProjectSnapshotV1[]>([]);
  project = $state<ContentProjectSnapshotV1 | null>(null);
  jobs = $state<JobSnapshotV1[]>([]);
  candidates = $state<ClipCandidate[]>([]);
  runId = $state<string | null>(null);
  artifacts = $state<ArtifactManifestV1[]>([]);
  selectedCandidateId = $state<string | null>(null);

  loading = $state(false);
  error = $state<string | null>(null);
  statusMessage = $state("");
  /** Blocks double-submit for decisions / render. */
  pendingAction = $state<string | null>(null);

  /**
   * Parte 1 confirmada (Continuar → encuadre).
   * Tras analizar / caché, false para NO saltar a marcos verdes.
   */
  part1Confirmed = $state(false);

  /**
   * Operador eligió “Ir al video completo” desde el resultado:
   * no saltar a completed aunque exista un short exportado.
   */
  preferReviewOverResult = $state(false);

  /** Analysis progress 0–100 while createAndAnalyze / reanalyze runs. */
  analysisProgressPct = $state(0);
  analysisProgressMessage = $state("");

  /** Optional override only for create wizard before project exists. */
  draftPath = $state<string | null>(null);
  draftTitle = $state("");

  pollTimer: ReturnType<typeof setInterval> | null = null;

  /**
   * Stage from durable snapshot, with short-lived in-flight overrides so the
   * operator never stays on Import/Ajuste while a long action is running.
   * (Analysis may run as a blocking command without intermediate jobs.)
   */
  get stage(): ProjectStage {
    return applySessionStageOverrides(this.computeStage(), {
      pendingAction: this.pendingAction,
      hasProject: !!this.project,
      candidateCount: this.candidates.length,
      part1Confirmed: this.part1Confirmed,
      preferReviewOverResult: this.preferReviewOverResult,
    });
  }

  get route(): FlowRoute {
    if (this.screen === "list") return "projects";
    // Stay on create only before a project exists
    if (this.screen === "create" && !this.project) return "create";
    const st = this.stage;
    return stageToRoute(st, { hasCandidates: this.candidates.length > 0 });
  }

  get flowProject(): FlowProjectView | null {
    if (!this.project) return null;
    return {
      id: this.project.id,
      title: this.project.title,
      sourceMediaPath: this.project.sourceMediaPath,
      updatedAt: this.project.updatedAt,
      createdAt: this.project.createdAt,
    };
  }

  get flowCandidates(): FlowCandidateView[] {
    return this.candidates.map(mapCand);
  }

  get flowJobs(): FlowJobView[] {
    return this.jobs.map(mapJob);
  }

  get flowArtifacts(): FlowArtifactView[] {
    return this.artifacts.map(mapArt);
  }

  get selectedCandidate(): ClipCandidate | null {
    return this.candidates.find((c) => c.id === this.selectedCandidateId) ?? null;
  }

  get finalArtifact(): ArtifactManifestV1 | null {
    return (
      this.artifacts.find(
        (a) =>
          (a.kind === "vertical_mp4" || a.role === "final_deliverable") &&
          a.validationStatus === "passed",
      ) ?? null
    );
  }

  get activeRenderJob(): JobSnapshotV1 | null {
    return (
      this.jobs.find(
        (j) =>
          j.kind === "vertical_render" &&
          (j.status === "queued" || j.status === "running" || j.status === "cancelling"),
      ) ?? null
    );
  }

  computeStage(): ProjectStage {
    const hasApproved = this.candidates.some(
      (c) => c.status === "approved" || c.status === "modified" || c.status === "exported",
    );
    return deriveProjectStage({
      project: this.flowProject,
      jobs: this.flowJobs,
      candidates: this.flowCandidates,
      hasApprovedCandidate: hasApproved,
      artifacts: this.flowArtifacts,
    });
  }

  clearError() {
    this.error = null;
  }

  /** Keep slice stores aligned with session snapshots (no second stage). */
  private syncSliceStores() {
    projectStore.setList(this.projects);
    projectStore.setCurrent(this.project);
    candidateStore.setItems(this.candidates);
    candidateStore.setSelectedId(this.selectedCandidateId);
    candidateStore.runId = this.runId;
    jobStore.setItems(this.jobs);
    artifactStore.setItems(this.artifacts);
    reviewStore.setPending(this.pendingAction);
  }

  async refreshProjectList() {
    this.loading = true;
    this.error = null;
    try {
      this.projects = await api.listContentProjects(50);
      this.syncSliceStores();
    } catch (e) {
      this.error = String(e);
      this.projects = [];
    } finally {
      this.loading = false;
    }
  }

  /** Quitar un item del historial Continuar. */
  async deleteProjectFromHistory(id: string) {
    if (this.pendingAction) return;
    this.pendingAction = "delete_project";
    this.error = null;
    try {
      await api.deleteContentProject(id);
      this.projects = this.projects.filter((p) => p.id !== id);
      if (this.project?.id === id) {
        this.goProjects();
        return;
      }
      this.syncSliceStores();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  /**
   * Full snapshot load for a project then stage-driven navigation.
   */
  async loadProject(id: string) {
    this.loading = true;
    this.error = null;
    this.screen = "flow";
    this.part1Confirmed = false;
    try {
      this.project = await api.getContentProject(id);
      await this.refetchProjectData();
      this.ensureSelection();
      // Solo reanudar Parte 2 si ya no hay momentos pickable y hay aprobados
      const needsPick = this.candidates.some((c) => isPickableCandidateStatus(c.status));
      const pendingExport = this.candidates.some((c) => isPendingExportStatus(c.status));
      this.part1Confirmed = !needsPick && pendingExport;
      this.syncPolling();
    } catch (e) {
      this.error = String(e);
      this.project = null;
    } finally {
      this.loading = false;
    }
  }

  async refetchProjectData() {
    if (!this.project) return;
    const id = this.project.id;
    const [jobs, cands, arts, runs] = await Promise.all([
      api.listProjectJobs(id),
      api.listShortCandidates(id),
      api.listProjectArtifacts(id),
      api.listClippingRuns(id),
    ]);
    this.jobs = jobs;
    this.artifacts = arts;
    this.runId = runs[0] ?? this.runId;
    const prevCandidates = this.candidates;
    let nextCandidates = cands;
    if (this.runId) {
      try {
        const run = await api.getClippingRun(this.runId);
        nextCandidates = run.candidates;
        this.runId = run.id;
      } catch {
        nextCandidates = cands;
      }
    }
    this.candidates = mergeCandidatesAfterRefetch(
      prevCandidates,
      nextCandidates,
      this.pendingAction,
    );
    // Refresh project meta (nextAction)
    try {
      this.project = await api.getContentProject(id);
    } catch {
      /* keep */
    }
    this.syncSliceStores();
  }

  ensureSelection() {
    if (this.selectedCandidateId && this.candidates.some((c) => c.id === this.selectedCandidateId)) {
      return;
    }
    const approved = this.candidates.find(
      (c) => c.status === "approved" || c.status === "modified",
    );
    this.selectedCandidateId = approved?.id ?? this.candidates[0]?.id ?? null;
  }

  goProjects() {
    this.stopPolling();
    this.screen = "list";
    this.project = null;
    this.jobs = [];
    this.candidates = [];
    this.artifacts = [];
    this.runId = null;
    this.selectedCandidateId = null;
    this.pendingAction = null;
    this.part1Confirmed = false;
    this.preferReviewOverResult = false;
    projectStore.clear();
    candidateStore.clear();
    jobStore.clear();
    artifactStore.clear();
    reviewStore.clear();
    void this.refreshProjectList();
  }

  goCreate() {
    this.stopPolling();
    this.screen = "create";
    this.project = null;
    this.jobs = [];
    this.candidates = [];
    this.artifacts = [];
    this.runId = null;
    this.selectedCandidateId = null;
    this.pendingAction = null;
    this.part1Confirmed = false;
    this.preferReviewOverResult = false;
    this.draftPath = null;
    this.draftTitle = "";
    this.error = null;
    projectStore.clear();
    candidateStore.clear();
    jobStore.clear();
    artifactStore.clear();
    reviewStore.clear();
  }

  syncPolling() {
    this.stopPolling();
    const st = this.computeStage();
    if (st === "processing" || st === "rendering") {
      this.pollTimer = setInterval(() => {
        void this.refetchProjectData().then(() => this.ensureSelection());
      }, 2000);
    }
  }

  stopPolling() {
    if (this.pollTimer) {
      clearInterval(this.pollTimer);
      this.pollTimer = null;
    }
  }

  setAnalysisProgress(percent: number, message?: string) {
    this.analysisProgressPct = Math.min(99, Math.max(0, percent));
    if (message) this.analysisProgressMessage = message;
  }

  async createAndAnalyze(sourceMediaPath: string, title?: string) {
    if (this.pendingAction) return;
    this.pendingAction = "create";
    this.error = null;
    this.analysisProgressPct = 2;
    this.analysisProgressMessage = "Creando proyecto…";
    try {
      const p = await api.createContentProject({
        sourceMediaPath,
        title: title || undefined,
      });
      this.project = p;
      this.screen = "flow";
      // Enter Análisis station immediately (clipping is a blocking command).
      this.pendingAction = "analyze";
      this.part1Confirmed = false; // siempre revisar momentos tras analizar/caché
      this.statusMessage = "Analizando video…";
      this.analysisProgressPct = 5;
      this.analysisProgressMessage = "Preparando video…";
      this.syncSliceStores();
      // force=false: si es el mismo archivo sin cambios, reutiliza análisis + momentos
      const run = await api.runClippingForProject(p.id, false);
      this.analysisProgressPct = 100;
      this.analysisProgressMessage =
        run.candidates?.length > 0 ? "Listo (caché o análisis)" : "Sin momentos";
      this.runId = run.id;
      this.candidates = run.candidates ?? [];
      await this.refetchProjectData();
      this.ensureSelection();
      this.syncPolling();
      this.statusMessage = "";
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  /** Re-run analysis on the open project (explicit user action only). Always force. */
  async reanalyzeCurrent() {
    if (!this.project || this.pendingAction) return;
    this.pendingAction = "analyze";
    this.part1Confirmed = false;
    this.error = null;
    this.statusMessage = "Reanalizando video…";
    this.analysisProgressPct = 5;
    this.analysisProgressMessage = "Análisis forzado (sin caché)…";
    try {
      const run = await api.runClippingForProject(this.project.id, true);
      this.analysisProgressPct = 100;
      this.runId = run.id;
      this.candidates = run.candidates;
      this.screen = "flow";
      await this.refetchProjectData();
      this.ensureSelection();
      this.syncPolling();
      this.statusMessage = "";
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  /** Persist approve without pendingAction gate (internal / chained actions). */
  private async persistApprove(candidateId: string): Promise<ClipCandidate | null> {
    if (!this.runId) return null;
    const updated = await api.saveReviewDecision({
      runId: this.runId,
      candidateId,
      decision: "approve",
    });
    this.candidates = this.candidates.map((c) => (c.id === updated.id ? updated : c));
    this.selectedCandidateId = updated.id;
    return updated;
  }

  async decideUseCandidate(candidateId: string) {
    if (!this.runId || !canStartDecision(this.pendingAction)) return;
    this.pendingAction = "approve";
    this.error = null;
    try {
      await this.persistApprove(candidateId);
      await this.refetchProjectData();
      this.ensureSelection();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  /**
   * Parte 1 → Parte 2: aprueba solo los momentos SELECCIONADOS.
   * Los no marcados se dejan como están (no avanzan).
   */
  async approveSelectedCandidates(candidateIds: string[]) {
    await this.finalizeMomentSelection({
      keep: candidateIds.map((id) => {
        const c = this.candidates.find((x) => x.id === id);
        return { id, start: c?.start ?? 0, end: c?.end ?? 10 };
      }),
      discardIds: [],
    });
  }

  /**
   * Cierra la revisión 1-a-1: guarda duraciones, descarta y aprueba en un solo pending.
   */
  async finalizeMomentSelection(args: {
    keep: { id: string; start: number; end: number }[];
    discardIds: string[];
  }) {
    if (!this.runId || !canStartDecision(this.pendingAction)) return;
    const keep = args.keep.filter((k) => k.id);
    if (keep.length === 0) return;
    this.pendingAction = "approve";
    this.error = null;
    try {
      // Marcar Parte 1 hecha solo si el batch termina bien (al final)
      const runId = this.runId;
      for (const id of args.discardIds) {
        try {
          const updated = await api.saveReviewDecision({
            runId,
            candidateId: id,
            decision: "reject",
            reason: "Descartado en revisión",
          });
          this.candidates = this.candidates.map((c) =>
            c.id === updated.id ? updated : c,
          );
        } catch {
          /* best-effort discard */
        }
      }
      let lastId: string | null = null;
      for (const k of keep) {
        try {
          const spanned = await api.saveReviewDecision({
            runId,
            candidateId: k.id,
            decision: "modify_span",
            startS: k.start,
            endS: k.end,
          });
          this.candidates = this.candidates.map((c) =>
            c.id === spanned.id ? spanned : c,
          );
        } catch {
          /* span optional if already correct */
        }
        const cur = this.candidates.find((c) => c.id === k.id);
        if (
          cur &&
          (cur.status === "approved" ||
            cur.status === "modified" ||
            cur.status === "exported" ||
            cur.status === "exporting")
        ) {
          lastId = k.id;
          continue;
        }
        await this.persistApprove(k.id);
        lastId = k.id;
      }
      if (lastId) this.selectedCandidateId = lastId;
      await this.refetchProjectData();
      const preferred =
        keep
          .map((k) => this.candidates.find((c) => c.id === k.id))
          .find(
            (c) =>
              c &&
              (c.status === "approved" ||
                c.status === "modified" ||
                c.status === "exported"),
          )?.id ?? lastId;
      if (preferred) this.selectedCandidateId = preferred;
      this.ensureSelection();
      this.part1Confirmed = true; // ahora sí → Parte 2 (marcos verdes)
    } catch (e) {
      this.error = String(e);
      this.part1Confirmed = false;
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  /** Volver de encuadre a revisión de momentos (pestañas). */
  backToMomentReview() {
    this.part1Confirmed = false;
    this.preferReviewOverResult = true;
  }

  /**
   * Desde el Short terminado: reabrir el flujo completo (momentos → encuadre),
   * sin borrar el archivo ya exportado.
   */
  openFullVideoFlow() {
    this.preferReviewOverResult = true;
    this.part1Confirmed = false;
    this.error = null;
    this.statusMessage = "";
    this.ensureSelection();
  }

  async decideDiscardCandidate(candidateId: string, reason = "Descartado") {
    if (!this.runId || !canStartDecision(this.pendingAction)) return;
    this.pendingAction = "reject";
    this.error = null;
    try {
      const updated = await api.saveReviewDecision({
        runId: this.runId,
        candidateId,
        decision: "reject",
        reason,
      });
      this.candidates = this.candidates.map((c) => (c.id === updated.id ? updated : c));
      await this.refetchProjectData();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  async saveSpan(start: number, end: number) {
    if (!this.runId || !this.selectedCandidateId || this.pendingAction) return;
    this.pendingAction = "span";
    try {
      const updated = await api.saveReviewDecision({
        runId: this.runId,
        candidateId: this.selectedCandidateId,
        decision: "modify_span",
        startS: start,
        endS: end,
      });
      this.candidates = this.candidates.map((c) => (c.id === updated.id ? updated : c));
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
    }
  }

  async saveFraming(framing: ClipCandidate["framing"]) {
    if (!this.runId || !this.selectedCandidateId || this.pendingAction) return;
    this.pendingAction = "framing";
    try {
      const updated = await api.saveReviewDecision({
        runId: this.runId,
        candidateId: this.selectedCandidateId,
        decision: "modify_framing",
        framing,
      });
      this.candidates = this.candidates.map((c) => (c.id === updated.id ? updated : c));
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
    }
  }

  /**
   * Guarda marcos verdes + lanza render en un solo pending (evita que el 2º paso se bloquee).
   */
  async exportShortWithFraming(
    framing: ClipCandidate["framing"],
    cues: { startS: number; endS: number; text: string }[],
    burnIn = true,
  ) {
    if (this.activeRenderJob) {
      this.error = "Ya hay un Short en creación. Esperá a que termine.";
      return;
    }
    if (!this.runId || !this.selectedCandidateId) {
      this.error = "No hay momento seleccionado para exportar.";
      return;
    }
    if (this.pendingAction) {
      this.error = "Esperá un momento e intentá de nuevo.";
      return;
    }
    const runId = this.runId;
    const candidateId = this.selectedCandidateId;
    this.pendingAction = "render";
    this.error = null;
    this.statusMessage = "Guardando encuadre y creando Short…";
    try {
      // 1) framing + panels
      try {
        const framed = await api.saveReviewDecision({
          runId,
          candidateId,
          decision: "modify_framing",
          framing,
        });
        this.candidates = this.candidates.map((c) =>
          c.id === framed.id ? framed : c,
        );
      } catch (e) {
        this.error = `No se pudo guardar el encuadre: ${e}`;
        return;
      }
      // 2) approve if needed
      const sel = this.candidates.find((c) => c.id === candidateId);
      if (
        sel &&
        sel.status !== "approved" &&
        sel.status !== "modified" &&
        sel.status !== "exported"
      ) {
        await this.persistApprove(candidateId);
      }
      // 3) plan + job
      this.statusMessage = "Renderizando Short 9:16…";
      const plan = await api.createVerticalRenderPlan({
        runId,
        candidateId,
        burnIn,
        cues: cues.map((c, i) => ({
          id: `cue-${i}`,
          startS: c.startS,
          endS: c.endS,
          text: c.text,
        })),
      });
      const job = await api.startVerticalRender(plan.id);
      this.jobs = [job, ...this.jobs.filter((j) => j.id !== job.id)];
      await this.refetchProjectData();
      this.syncPolling();
      // Render es síncrono en MVP: al volver el job ya terminó o falló
      if (job.status === "failed") {
        this.error =
          "No se pudo generar el Short. Revisá FFmpeg y el video fuente.";
        this.statusMessage = "";
        return;
      }
      const art = this.finalArtifact;
      if (art?.path) {
        this.part1Confirmed = true;
        this.preferReviewOverResult = false; // mostrar Resultado
        this.statusMessage = `Guardado: ${art.path}`;
      } else {
        this.statusMessage = "Render terminado — buscando archivo…";
        await this.refetchProjectData();
        if (this.finalArtifact?.path) {
          this.part1Confirmed = true;
          this.preferReviewOverResult = false;
          this.statusMessage = `Guardado: ${this.finalArtifact.path}`;
        }
      }
    } catch (e) {
      this.error = `Error al crear el Short: ${e}`;
      this.statusMessage = "";
    } finally {
      this.pendingAction = null;
      this.syncSliceStores();
    }
  }

  async startCreateShort(cues: { startS: number; endS: number; text: string }[], burnIn: boolean) {
    if (
      !canStartRender({
        pendingAction: this.pendingAction,
        hasActiveRenderJob: !!this.activeRenderJob,
        runId: this.runId,
        candidateId: this.selectedCandidateId,
      })
    ) {
      if (this.activeRenderJob) {
        this.error = "Ya hay un Short en creación. Esperá o cancelá el proceso actual.";
      } else if (!this.runId || !this.selectedCandidateId) {
        this.error = "Falta el momento seleccionado para exportar.";
      }
      return;
    }
    const runId = this.runId!;
    const candidateId = this.selectedCandidateId!;
    this.pendingAction = "render";
    this.error = null;
    this.statusMessage = "Creando Short…";
    try {
      const sel = this.selectedCandidate;
      if (sel && sel.status !== "approved" && sel.status !== "modified" && sel.status !== "exported") {
        await this.persistApprove(sel.id);
      }
      const plan = await api.createVerticalRenderPlan({
        runId,
        candidateId,
        burnIn,
        cues: cues.map((c, i) => ({
          id: `cue-${i}`,
          startS: c.startS,
          endS: c.endS,
          text: c.text,
        })),
      });
      const job = await api.startVerticalRender(plan.id);
      this.jobs = [job, ...this.jobs.filter((j) => j.id !== job.id)];
      await this.refetchProjectData();
      this.syncPolling();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
      this.statusMessage = "";
    }
  }

  async retryFailed() {
    const job =
      this.jobs.find((j) => j.status === "failed" || j.status === "interrupted") ?? null;
    if (!job || this.pendingAction) return;
    this.pendingAction = "retry";
    try {
      await api.retryJob(job.id);
      if (job.kind === "vertical_render") {
        // re-execute render path
        await api.startVerticalRender(job.renderPlanId ?? "");
      }
      await this.refetchProjectData();
      this.syncPolling();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
    }
  }

  async cancelActive() {
    const job = this.jobs.find(
      (j) => j.status === "queued" || j.status === "running",
    );
    if (!job || this.pendingAction) return;
    this.pendingAction = "cancel";
    try {
      await api.cancelJob(job.id);
      await this.refetchProjectData();
      this.syncPolling();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.pendingAction = null;
    }
  }
}

export const vnextSession = new VnextSessionStore();
