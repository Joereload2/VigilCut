import { describe, expect, it } from "vitest";
import {
  applySessionStageOverrides,
  deriveProjectStage,
  nextActionHuman,
  stageToRoute,
} from "$lib/vnext/presentation/deriveStage";
import type {
  DeriveStageInput,
  FlowArtifactView,
  FlowCandidateView,
  FlowJobView,
  FlowProjectView,
} from "$lib/vnext/types/flow";

const project: FlowProjectView = {
  id: "p1",
  title: "Demo",
  sourceMediaPath: "C:\\videos\\client.mp4",
  updatedAt: "2026-07-29T12:00:00Z",
  createdAt: "2026-07-29T11:00:00Z",
};

function job(
  partial: Partial<FlowJobView> & Pick<FlowJobView, "id" | "kind" | "status">,
): FlowJobView {
  return {
    progressPct: 0,
    ...partial,
  };
}

function cand(
  partial: Partial<FlowCandidateView> & Pick<FlowCandidateView, "id">,
): FlowCandidateView {
  return {
    status: "preselected",
    start: 10,
    end: 40,
    duration: 30,
    score: 72,
    title: "Hook",
    transcript: "hola mundo",
    sourceMediaPath: project.sourceMediaPath,
    framing: {
      mode: "face",
      centerX: 0.5,
      centerY: 0.4,
      zoom: 1,
      outputWidth: 1080,
      outputHeight: 1920,
    },
    ...partial,
  };
}

function art(
  partial: Partial<FlowArtifactView> & Pick<FlowArtifactView, "id">,
): FlowArtifactView {
  return {
    kind: "vertical_mp4",
    role: "final_deliverable",
    path: "C:\\out\\short.mp4",
    validationStatus: "passed",
    byteSize: 1_200_000,
    mimeType: "video/mp4",
    createdByJobId: "j-render",
    ...partial,
  };
}

function input(over: Partial<DeriveStageInput> = {}): DeriveStageInput {
  return {
    project,
    jobs: [],
    candidates: [],
    hasApprovedCandidate: false,
    artifacts: [],
    ...over,
  };
}

describe("deriveProjectStage", () => {
  it("source_required without project or path", () => {
    expect(deriveProjectStage(input({ project: null }))).toBe("source_required");
    expect(
      deriveProjectStage(
        input({ project: { ...project, sourceMediaPath: "  " } }),
      ),
    ).toBe("source_required");
  });

  it("source_ready with source and no analysis", () => {
    expect(deriveProjectStage(input())).toBe("source_ready");
  });

  it("processing while analysis jobs active", () => {
    expect(
      deriveProjectStage(
        input({
          jobs: [job({ id: "1", kind: "ingest_probe", status: "queued", progressPct: 0 })],
        }),
      ),
    ).toBe("processing");
    expect(
      deriveProjectStage(
        input({
          jobs: [job({ id: "2", kind: "transcribe", status: "running", progressPct: 40 })],
        }),
      ),
    ).toBe("processing");
    expect(
      deriveProjectStage(
        input({
          jobs: [
            job({ id: "3", kind: "clipping_run:x", status: "running", progressPct: 10 }),
          ],
        }),
      ),
    ).toBe("processing");
  });

  it("candidates_ready when candidates exist", () => {
    expect(
      deriveProjectStage(input({ candidates: [cand({ id: "c1" })] })),
    ).toBe("candidates_ready");
  });

  it("candidates_ready (empty route) when analysis completed with zero candidates", () => {
    expect(
      deriveProjectStage(
        input({
          jobs: [
            job({
              id: "j",
              kind: "generate_short_candidates",
              status: "completed",
              progressPct: 100,
            }),
          ],
          candidates: [],
        }),
      ),
    ).toBe("candidates_ready");
  });

  it("adjusting when candidate approved without artifact", () => {
    expect(
      deriveProjectStage(
        input({
          candidates: [cand({ id: "c1", status: "approved" })],
          hasApprovedCandidate: true,
        }),
      ),
    ).toBe("adjusting");
  });

  it("rendering when vertical_render active", () => {
    expect(
      deriveProjectStage(
        input({
          hasApprovedCandidate: true,
          jobs: [
            job({ id: "r", kind: "vertical_render", status: "running", progressPct: 55 }),
          ],
        }),
      ),
    ).toBe("rendering");
  });

  it("completed when valid final artifact exists", () => {
    expect(
      deriveProjectStage(
        input({
          hasApprovedCandidate: true,
          jobs: [
            job({
              id: "r",
              kind: "vertical_render",
              status: "completed",
              progressPct: 100,
              resultArtifactId: "a1",
            }),
          ],
          artifacts: [art({ id: "a1" })],
        }),
      ),
    ).toBe("completed");
  });

  it("failed when render completed without artifact (integrity)", () => {
    expect(
      deriveProjectStage(
        input({
          hasApprovedCandidate: true,
          jobs: [
            job({
              id: "r",
              kind: "vertical_render",
              status: "completed",
              progressPct: 100,
              resultArtifactId: null,
            }),
          ],
          artifacts: [],
        }),
      ),
    ).toBe("failed");
  });

  it("failed / interrupted on render terminal states", () => {
    expect(
      deriveProjectStage(
        input({
          hasApprovedCandidate: true,
          jobs: [job({ id: "r", kind: "vertical_render", status: "failed" })],
        }),
      ),
    ).toBe("failed");
    expect(
      deriveProjectStage(
        input({
          hasApprovedCandidate: true,
          jobs: [job({ id: "r", kind: "vertical_render", status: "interrupted" })],
        }),
      ),
    ).toBe("interrupted");
  });

  it("failed analysis with no candidates", () => {
    expect(
      deriveProjectStage(
        input({
          jobs: [job({ id: "t", kind: "transcribe", status: "failed" })],
        }),
      ),
    ).toBe("failed");
  });

  it("active render wins over existing artifact (export in progress)", () => {
    expect(
      deriveProjectStage(
        input({
          jobs: [
            job({ id: "r", kind: "vertical_render", status: "running", progressPct: 10 }),
          ],
          artifacts: [art({ id: "a1" })],
        }),
      ),
    ).toBe("rendering");
  });

  it("preselected candidates win over stale final artifact (no jump after analyze)", () => {
    expect(
      deriveProjectStage(
        input({
          candidates: [cand({ id: "c1", status: "preselected" })],
          artifacts: [art({ id: "old" })],
        }),
      ),
    ).toBe("candidates_ready");
  });

  it("final artifact wins over approved candidates (show result + path)", () => {
    expect(
      deriveProjectStage(
        input({
          candidates: [cand({ id: "c1", status: "approved" })],
          hasApprovedCandidate: true,
          artifacts: [art({ id: "old" })],
        }),
      ),
    ).toBe("completed");
  });

  it("approved without artifact stays adjusting", () => {
    expect(
      deriveProjectStage(
        input({
          candidates: [cand({ id: "c1", status: "approved" })],
          hasApprovedCandidate: true,
          artifacts: [],
        }),
      ),
    ).toBe("adjusting");
  });

  it("pickable moments win over partial approved (no skip Part 1)", () => {
    expect(
      deriveProjectStage(
        input({
          candidates: [
            cand({ id: "c1", status: "approved" }),
            cand({ id: "c2", status: "preselected" }),
          ],
          hasApprovedCandidate: true,
        }),
      ),
    ).toBe("candidates_ready");
  });

  it("out-of-order: stale queued analysis does not override completed artifact", () => {
    expect(
      deriveProjectStage(
        input({
          jobs: [
            job({ id: "old", kind: "ingest_probe", status: "queued" }),
            job({
              id: "r",
              kind: "vertical_render",
              status: "completed",
              resultArtifactId: "a1",
            }),
          ],
          artifacts: [art({ id: "a1" })],
        }),
      ),
    ).toBe("completed");
  });

  it("refresh reconstructs stage from snapshot only", () => {
    const snap: DeriveStageInput = input({
      candidates: [cand({ id: "c1", status: "approved" })],
      hasApprovedCandidate: true,
      jobs: [job({ id: "r", kind: "vertical_render", status: "queued" })],
    });
    expect(deriveProjectStage(snap)).toBe("rendering");
    // same snapshot again
    expect(deriveProjectStage(snap)).toBe("rendering");
  });
});

describe("stageToRoute", () => {
  it("maps stages to flow routes", () => {
    expect(stageToRoute("source_ready")).toBe("create");
    expect(stageToRoute("processing")).toBe("processing");
    expect(stageToRoute("candidates_ready")).toBe("candidates");
    expect(stageToRoute("candidates_ready", { hasCandidates: false })).toBe(
      "empty_candidates",
    );
    expect(stageToRoute("adjusting")).toBe("edit");
    expect(stageToRoute("rendering")).toBe("rendering");
    expect(stageToRoute("completed")).toBe("result");
    expect(stageToRoute("interrupted")).toBe("interrupted");
    expect(stageToRoute("failed")).toBe("failed");
  });
});

describe("integration transitions (derive only)", () => {
  const chain: { name: string; snap: DeriveStageInput; stage: string; route: string }[] =
    [
      {
        name: "source_ready → processing",
        snap: input({
          jobs: [job({ id: "1", kind: "ingest_probe", status: "running" })],
        }),
        stage: "processing",
        route: "processing",
      },
      {
        name: "processing → candidates_ready",
        snap: input({ candidates: [cand({ id: "c1" })] }),
        stage: "candidates_ready",
        route: "candidates",
      },
      {
        name: "candidates_ready → adjusting",
        snap: input({
          candidates: [cand({ id: "c1", status: "approved" })],
          hasApprovedCandidate: true,
        }),
        stage: "adjusting",
        route: "edit",
      },
      {
        name: "adjusting → rendering",
        snap: input({
          hasApprovedCandidate: true,
          jobs: [job({ id: "r", kind: "vertical_render", status: "running" })],
        }),
        stage: "rendering",
        route: "rendering",
      },
      {
        name: "rendering → completed",
        snap: input({
          hasApprovedCandidate: true,
          jobs: [
            job({
              id: "r",
              kind: "vertical_render",
              status: "completed",
              resultArtifactId: "a1",
            }),
          ],
          artifacts: [art({ id: "a1" })],
        }),
        stage: "completed",
        route: "result",
      },
      {
        name: "rendering → failed",
        snap: input({
          hasApprovedCandidate: true,
          jobs: [job({ id: "r", kind: "vertical_render", status: "failed" })],
        }),
        stage: "failed",
        route: "failed",
      },
      {
        name: "rendering → interrupted",
        snap: input({
          hasApprovedCandidate: true,
          jobs: [job({ id: "r", kind: "vertical_render", status: "interrupted" })],
        }),
        stage: "interrupted",
        route: "interrupted",
      },
      {
        name: "completed without artifact",
        snap: input({
          hasApprovedCandidate: true,
          jobs: [
            job({
              id: "r",
              kind: "vertical_render",
              status: "completed",
              resultArtifactId: null,
            }),
          ],
        }),
        stage: "failed",
        route: "failed",
      },
    ];

  for (const step of chain) {
    it(step.name, () => {
      const stage = deriveProjectStage(step.snap);
      expect(stage).toBe(step.stage);
      expect(
        stageToRoute(stage, {
          hasCandidates: step.snap.candidates.length > 0,
        }),
      ).toBe(step.route);
    });
  }

  it("nextActionHuman is operator language", () => {
    expect(nextActionHuman("candidates_ready")).toBe("Elegir momentos");
    expect(nextActionHuman("completed")).toBe("Ver resultado");
    expect(nextActionHuman("source_ready")).not.toMatch(/ingest|queued|vertical/i);
  });
});

describe("applySessionStageOverrides", () => {
  const baseOpts = {
    pendingAction: null as string | null,
    hasProject: true,
    candidateCount: 3,
    part1Confirmed: false,
    preferReviewOverResult: false,
  };

  it("forces processing while analyzing", () => {
    expect(
      applySessionStageOverrides("source_ready", {
        ...baseOpts,
        pendingAction: "analyze",
      }),
    ).toBe("processing");
  });

  it("forces rendering while export pendingAction", () => {
    expect(
      applySessionStageOverrides("adjusting", {
        ...baseOpts,
        pendingAction: "render",
        part1Confirmed: true,
      }),
    ).toBe("rendering");
  });

  it("after analyze keeps Part1 even if base is adjusting (approved cache)", () => {
    expect(
      applySessionStageOverrides("adjusting", {
        ...baseOpts,
        part1Confirmed: false,
      }),
    ).toBe("candidates_ready");
  });

  it("Ir al video completo leaves completed for candidates review", () => {
    expect(
      applySessionStageOverrides("completed", {
        ...baseOpts,
        preferReviewOverResult: true,
        part1Confirmed: false,
      }),
    ).toBe("candidates_ready");
  });

  it("Ir al video completo + Part1 done goes to adjusting", () => {
    expect(
      applySessionStageOverrides("completed", {
        ...baseOpts,
        preferReviewOverResult: true,
        part1Confirmed: true,
      }),
    ).toBe("adjusting");
  });

  it("after Part1 confirm, adjusting is allowed", () => {
    expect(
      applySessionStageOverrides("adjusting", {
        ...baseOpts,
        part1Confirmed: true,
      }),
    ).toBe("adjusting");
  });
});
