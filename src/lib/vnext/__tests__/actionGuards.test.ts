import { describe, expect, it } from "vitest";
import {
  applyApproveLocally,
  canStartDecision,
  canStartRender,
  mergeCandidatesAfterRefetch,
} from "$lib/vnext/presentation/actionGuards";

describe("double-submit guards", () => {
  it("blocks second decision while pending", () => {
    expect(canStartDecision(null)).toBe(true);
    expect(canStartDecision("approve")).toBe(false);
    expect(canStartDecision("reject")).toBe(false);
  });

  it("blocks second render while pending or active job", () => {
    expect(
      canStartRender({
        pendingAction: null,
        hasActiveRenderJob: false,
        runId: "r1",
        candidateId: "c1",
      }),
    ).toBe(true);
    expect(
      canStartRender({
        pendingAction: "render",
        hasActiveRenderJob: false,
        runId: "r1",
        candidateId: "c1",
      }),
    ).toBe(false);
    expect(
      canStartRender({
        pendingAction: null,
        hasActiveRenderJob: true,
        runId: "r1",
        candidateId: "c1",
      }),
    ).toBe(false);
    expect(
      canStartRender({
        pendingAction: null,
        hasActiveRenderJob: false,
        runId: null,
        candidateId: "c1",
      }),
    ).toBe(false);
  });

  it("approve applies once locally", () => {
    const list = [
      { id: "c1", status: "proposed" },
      { id: "c2", status: "proposed" },
    ];
    const next = applyApproveLocally(list, { id: "c1", status: "approved" });
    expect(next.find((c) => c.id === "c1")?.status).toBe("approved");
    expect(next.find((c) => c.id === "c2")?.status).toBe("proposed");
  });

  it("stale refetch during pending does not revert approve", () => {
    const local = [
      { id: "c1", status: "approved" },
      { id: "c2", status: "proposed" },
    ];
    const server = [
      { id: "c1", status: "proposed" },
      { id: "c2", status: "proposed" },
    ];
    const merged = mergeCandidatesAfterRefetch(local, server, "approve");
    expect(merged.find((c) => c.id === "c1")?.status).toBe("approved");
  });

  it("after pending clears, server wins", () => {
    const local = [{ id: "c1", status: "approved" }];
    const server = [{ id: "c1", status: "approved" }];
    expect(mergeCandidatesAfterRefetch(local, server, null)[0].status).toBe("approved");
  });
});
