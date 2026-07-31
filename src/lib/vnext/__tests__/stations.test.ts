import { describe, expect, it } from "vitest";
import { buildStationGuide, stageToStationId } from "$lib/vnext/presentation/stations";
import type { ProjectStage } from "$lib/vnext/types/flow";

describe("stageToStationId", () => {
  it("maps stages to workstations", () => {
    expect(stageToStationId("source_ready", false)).toBe("import");
    expect(stageToStationId("processing", false)).toBe("analysis");
    expect(stageToStationId("candidates_ready", false)).toBe("review");
    expect(stageToStationId("adjusting", false)).toBe("adjust");
    expect(stageToStationId("rendering", false)).toBe("render");
    expect(stageToStationId("completed", false)).toBe("result");
    expect(stageToStationId("source_ready", true)).toBe("hub");
  });
});

describe("buildStationGuide", () => {
  const base = {
    isHub: false,
    hasSource: true,
    candidateCount: 3,
    hasSelection: true,
    jobs: [] as { id: string; kind: string; status: string; progressPct: number }[],
  };

  it("each productive station answers the five questions", () => {
    const stages: ProjectStage[] = [
      "source_ready",
      "processing",
      "candidates_ready",
      "adjusting",
      "rendering",
      "completed",
    ];
    for (const stage of stages) {
      const g = buildStationGuide({ ...base, stage, hasSource: stage !== "source_required" });
      expect(g.youDo.length).toBeGreaterThan(5);
      expect(g.systemDoes.length).toBeGreaterThan(5);
      expect(g.missing.length).toBeGreaterThan(2);
      expect(g.remaining.length).toBeGreaterThan(0);
      expect(g.nextDo.length).toBeGreaterThan(2);
      expect(g.objective.length).toBeGreaterThan(5);
      // never leak internals
      expect(JSON.stringify(g)).not.toMatch(
        /ingest_probe|vertical_render|queued|waiting_review|UUID/i,
      );
    }
  });

  it("import primary is Analizar when source present", () => {
    const g = buildStationGuide({
      stage: "source_ready",
      isHub: false,
      hasSource: true,
      candidateCount: 0,
      hasSelection: false,
      jobs: [],
    });
    expect(g.stationName).toBe("Importación");
    expect(g.primaryAction).toBe("Analizar video");
  });

  it("analysis has no primary action (wait)", () => {
    const g = buildStationGuide({
      stage: "processing",
      isHub: false,
      hasSource: true,
      candidateCount: 0,
      hasSelection: false,
      jobs: [{ id: "1", kind: "transcribe", status: "running", progressPct: 40 }],
    });
    expect(g.stationName).toBe("Análisis");
    expect(g.primaryAction).toBeNull();
    expect(g.nextDo).toMatch(/Esperar/i);
  });

  it("review primary is Usar este clip", () => {
    const g = buildStationGuide({
      stage: "candidates_ready",
      isHub: false,
      hasSource: true,
      candidateCount: 4,
      hasSelection: true,
      jobs: [],
    });
    expect(g.primaryAction).toBe("Usar este clip");
    expect(g.stepIndex).toBe(3);
  });

  it("adjust primary is Crear Short", () => {
    const g = buildStationGuide({
      stage: "adjusting",
      isHub: false,
      hasSource: true,
      candidateCount: 1,
      hasSelection: true,
      jobs: [],
    });
    expect(g.primaryAction).toBe("Crear Short");
  });

  it("render has no primary (wait)", () => {
    const g = buildStationGuide({
      stage: "rendering",
      isHub: false,
      hasSource: true,
      candidateCount: 1,
      hasSelection: true,
      jobs: [{ id: "r", kind: "vertical_render", status: "running", progressPct: 50 }],
    });
    expect(g.primaryAction).toBeNull();
  });

  it("result primary opens folder", () => {
    const g = buildStationGuide({
      stage: "completed",
      isHub: false,
      hasSource: true,
      candidateCount: 1,
      hasSelection: true,
      jobs: [],
    });
    expect(g.primaryAction).toBe("Abrir carpeta");
  });
});
