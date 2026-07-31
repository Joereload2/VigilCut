/**
 * Candidate snapshot slice for vNext MVP.
 */
import type { ClipCandidate } from "$lib/types";

class CandidateStore {
  items = $state<ClipCandidate[]>([]);
  selectedId = $state<string | null>(null);
  runId = $state<string | null>(null);

  setItems(items: ClipCandidate[]) {
    this.items = items;
  }

  setSelectedId(id: string | null) {
    this.selectedId = id;
  }

  get selected(): ClipCandidate | null {
    return this.items.find((c) => c.id === this.selectedId) ?? null;
  }

  clear() {
    this.items = [];
    this.selectedId = null;
    this.runId = null;
  }
}

export const candidateStore = new CandidateStore();
