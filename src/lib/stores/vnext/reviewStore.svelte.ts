import type { ClipCandidate, ClipFraming, ClippingRun } from "$lib/types";
import type { SubtitleCueV1, VerticalRenderPlanV1 } from "$lib/types/vnext/v1";
import * as vnext from "$lib/utils/vnext";
import { vnextUiStore } from "./uiStore.svelte";
import { vnextJobStore } from "./jobStore.svelte";

class VnextReviewStore {
  runId = $state<string | null>(null);
  run = $state<ClippingRun | null>(null);
  candidates = $state<ClipCandidate[]>([]);
  selectedId = $state<string | null>(null);
  draftStart = $state(0);
  draftEnd = $state(0);
  draftFraming = $state<ClipFraming | null>(null);
  cues = $state<SubtitleCueV1[]>([]);
  lastPlan = $state<VerticalRenderPlanV1 | null>(null);

  get selected(): ClipCandidate | null {
    return this.candidates.find((c) => c.id === this.selectedId) ?? null;
  }

  async loadForProject(contentProjectId: string) {
    vnextUiStore.setError(null);
    try {
      this.candidates = await vnext.listShortCandidates(contentProjectId);
      const runs = await vnext.listClippingRuns(contentProjectId);
      this.runId = runs[0] ?? null;
      if (this.runId) {
        this.run = await vnext.getClippingRun(this.runId);
        this.candidates = this.run.candidates;
      }
      if (this.candidates.length && !this.selectedId) {
        this.selectCandidate(this.candidates[0].id);
      }
    } catch (e) {
      vnextUiStore.setError(String(e));
    }
  }

  selectCandidate(id: string) {
    this.selectedId = id;
    const c = this.candidates.find((x) => x.id === id);
    if (c) {
      this.draftStart = c.start;
      this.draftEnd = c.end;
      this.draftFraming = { ...c.framing };
      this.cues = c.transcript
        ? [
            {
              id: "auto-0",
              startS: 0,
              endS: Math.max(0.5, c.duration),
              text: c.transcript.slice(0, 200),
            },
          ]
        : [];
    }
  }

  async generateCandidates(contentProjectId: string) {
    vnextUiStore.setBusy(true, "Generando candidatos…");
    try {
      const run = await vnext.runClippingForProject(contentProjectId);
      this.run = run;
      this.runId = run.id;
      this.candidates = run.candidates;
      if (run.candidates[0]) this.selectCandidate(run.candidates[0].id);
      vnextUiStore.statusMessage = `${run.candidates.length} candidatos`;
    } catch (e) {
      vnextUiStore.setError(String(e));
    } finally {
      vnextUiStore.setBusy(false);
    }
  }

  async approve() {
    await this.decide("approve");
  }

  async reject(reason = "Rechazo humano") {
    await this.decide("reject", reason);
  }

  async applySpan() {
    await this.decide("modify_span");
  }

  async applyFraming() {
    await this.decide("modify_framing");
  }

  private async decide(decision: string, reason?: string) {
    if (!this.runId || !this.selectedId) return;
    vnextUiStore.setBusy(true);
    vnextUiStore.setError(null);
    try {
      const updated = await vnext.saveReviewDecision({
        runId: this.runId,
        candidateId: this.selectedId,
        decision,
        reason,
        startS: this.draftStart,
        endS: this.draftEnd,
        framing: this.draftFraming,
      });
      this.candidates = this.candidates.map((c) => (c.id === updated.id ? updated : c));
      vnextUiStore.statusMessage = `Decisión: ${decision}`;
    } catch (e) {
      vnextUiStore.setError(String(e));
    } finally {
      vnextUiStore.setBusy(false);
    }
  }

  async createPlanAndRender(burnIn = true) {
    if (!this.runId || !this.selectedId) return;
    vnextUiStore.setBusy(true, "Plan + render…");
    try {
      this.lastPlan = await vnext.createVerticalRenderPlan({
        runId: this.runId,
        candidateId: this.selectedId,
        burnIn,
        cues: this.cues,
      });
      const job = await vnextJobStore.startRender(this.lastPlan.id);
      vnextUiStore.statusMessage =
        job.status === "completed"
          ? "Render completado"
          : `Render: ${job.status}`;
      return job;
    } catch (e) {
      vnextUiStore.setError(String(e));
      throw e;
    } finally {
      vnextUiStore.setBusy(false);
    }
  }
}

export const vnextReviewStore = new VnextReviewStore();
