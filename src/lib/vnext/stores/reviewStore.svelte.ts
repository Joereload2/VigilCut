/**
 * Ephemeral review / decision UI state for vNext MVP.
 * Decisions themselves are persisted via backend API — this is not a second source of truth.
 */
class ReviewStore {
  /** Blocks double-submit for approve / reject / span / framing / render. */
  pendingAction = $state<string | null>(null);
  lastDecisionMessage = $state<string | null>(null);

  setPending(action: string | null) {
    this.pendingAction = action;
  }

  clear() {
    this.pendingAction = null;
    this.lastDecisionMessage = null;
  }
}

export const reviewStore = new ReviewStore();
