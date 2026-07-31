/**
 * Pure guards for double-submit prevention.
 * UI and session store share these so tests can verify without Tauri.
 */

export function canStartDecision(pendingAction: string | null | undefined): boolean {
  return !pendingAction;
}

export function canStartRender(args: {
  pendingAction: string | null | undefined;
  hasActiveRenderJob: boolean;
  runId: string | null | undefined;
  candidateId: string | null | undefined;
}): boolean {
  if (args.pendingAction) return false;
  if (args.hasActiveRenderJob) return false;
  if (!args.runId || !args.candidateId) return false;
  return true;
}

/** After approve API returns, local candidates must reflect approved status. */
export function applyApproveLocally<T extends { id: string; status: string }>(
  candidates: T[],
  updated: T,
): T[] {
  return candidates.map((c) => (c.id === updated.id ? updated : c));
}

/** Stale snapshot with older status must not wipe a newer local approve if ids match and server is lagging.
 *  Policy: prefer server snapshot after refetch, but while pendingAction is set keep optimistic status. */
export function mergeCandidatesAfterRefetch<T extends { id: string; status: string }>(
  local: T[],
  server: T[],
  pendingAction: string | null,
): T[] {
  if (!pendingAction) return server;
  // During pending, preserve optimistic approved if server still shows pickable status
  return server.map((s) => {
    const l = local.find((x) => x.id === s.id);
    if (
      l &&
      (l.status === "approved" || l.status === "modified") &&
      (s.status === "proposed" ||
        s.status === "pending" ||
        s.status === "new" ||
        s.status === "suggested" ||
        s.status === "preselected")
    ) {
      return l;
    }
    return s;
  });
}
