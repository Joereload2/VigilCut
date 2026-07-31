import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatDuration,
  jobStateToHumanLabel,
  jobToUserProgress,
  jobTypeToHumanLabel,
  projectStageToHumanLabel,
  scoreToHuman,
} from "$lib/vnext/presentation/jobLabels";
import type { FlowJobView } from "$lib/vnext/types/flow";

function j(over: Partial<FlowJobView> & Pick<FlowJobView, "kind" | "status">): FlowJobView {
  return { id: "x", progressPct: 0, ...over };
}

describe("jobTypeToHumanLabel", () => {
  it("never returns technical enum names", () => {
    for (const kind of [
      "ingest_probe",
      "transcribe",
      "generate_short_candidates",
      "vertical_render",
      "clipping_run:abc",
    ]) {
      const label = jobTypeToHumanLabel(kind);
      expect(label).not.toMatch(/ingest_probe|vertical_render|queued|clipping_run/i);
      expect(label.length).toBeGreaterThan(3);
    }
  });
});

describe("jobStateToHumanLabel", () => {
  it("maps statuses to human Spanish", () => {
    expect(jobStateToHumanLabel("queued")).toBe("En espera");
    expect(jobStateToHumanLabel("completed")).toBe("Terminado");
    expect(jobStateToHumanLabel("interrupted")).toBe("Interrumpido");
    expect(jobStateToHumanLabel("failed")).toBe("Error");
  });
});

describe("jobToUserProgress", () => {
  it("completed always reports 100%", () => {
    const p = jobToUserProgress(
      j({ kind: "vertical_render", status: "completed", progressPct: 0 }),
    );
    expect(p.percent).toBe(100);
    expect(p.indeterminate).toBe(false);
  });

  it("queued is indeterminate without fake 0%", () => {
    const p = jobToUserProgress(j({ kind: "ingest_probe", status: "queued", progressPct: 0 }));
    expect(p.percent).toBeNull();
    expect(p.indeterminate).toBe(true);
    expect(p.detail.toLowerCase()).toMatch(/cola|espera|capacidad/);
  });

  it("running always shows a percent (never silent 0%)", () => {
    const p = jobToUserProgress(j({ kind: "transcribe", status: "running", progressPct: 0 }));
    expect(p.percent).toBeGreaterThanOrEqual(1);
    expect(p.indeterminate).toBe(false);
  });

  it("running with progress shows percent capped below 100 until complete", () => {
    const p = jobToUserProgress(
      j({ kind: "vertical_render", status: "running", progressPct: 88 }),
    );
    expect(p.percent).toBe(88);
    expect(p.indeterminate).toBe(false);
  });
});

describe("projectStageToHumanLabel", () => {
  it("uses allowed MVP labels", () => {
    expect(projectStageToHumanLabel("source_ready")).toBe("Listo para analizar");
    expect(projectStageToHumanLabel("candidates_ready")).toBe("Candidatos listos");
    expect(projectStageToHumanLabel("completed")).toBe("Short terminado");
    expect(projectStageToHumanLabel("processing")).toBe("Procesando video");
  });
});

describe("score and format helpers", () => {
  it("scoreToHuman", () => {
    expect(scoreToHuman(90)).toBe("Muy prometedor");
    expect(scoreToHuman(70)).toBe("Buen fragmento");
    expect(scoreToHuman(55)).toBe("Aceptable");
    expect(scoreToHuman(10)).toBe("Bajo interés");
  });

  it("formatDuration / formatBytes", () => {
    expect(formatDuration(65)).toBe("1:05");
    expect(formatBytes(2048)).toMatch(/KB/);
  });
});
