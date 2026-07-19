import type { ClipCandidate, ClipFraming } from "$lib/types";

/** Shared selection for the main 9:16 short player (clips workspace). */
class ClippingUiStore {
  selected = $state<ClipCandidate | null>(null);
  /** Bump to force player restart */
  playToken = $state(0);

  select(c: ClipCandidate | null) {
    this.selected = c;
  }

  play(c: ClipCandidate) {
    this.selected = c;
    this.playToken += 1;
  }

  get framing(): ClipFraming | null {
    return this.selected?.framing ?? null;
  }
}

export const clippingUi = new ClippingUiStore();
