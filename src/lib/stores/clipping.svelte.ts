import type { ClipCandidate, ClipFraming } from "$lib/types";

type FramingSaver = (clipId: string, framing: ClipFraming) => Promise<void> | void;

/** Shared selection for the main 9:16 short player (clips workspace). */
class ClippingUiStore {
  selected = $state<ClipCandidate | null>(null);
  /** Bump to force player restart */
  playToken = $state(0);
  /** ClippingPanel registers so ShortPlayer can persist pan/focus */
  private framingSaver: FramingSaver | null = null;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;

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

  setFramingSaver(fn: FramingSaver | null) {
    this.framingSaver = fn;
  }

  /**
   * Live pan for 9:16 focus. Updates local framing immediately and
   * debounces persistence to the run store / backend.
   */
  panFraming(centerX: number, centerY: number) {
    const c = this.selected;
    if (!c) return;
    const framing: ClipFraming = {
      ...c.framing,
      mode: "manual",
      centerX: Math.min(0.95, Math.max(0.05, centerX)),
      centerY: Math.min(0.95, Math.max(0.05, centerY)),
    };
    this.selected = { ...c, framing };
    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => {
      const cur = this.selected;
      if (!cur || !this.framingSaver) return;
      void this.framingSaver(cur.id, cur.framing);
    }, 180);
  }

  /** Snap focus to face-friendly default (upper-center). */
  resetFraming() {
    this.panFraming(0.5, 0.42);
  }
}

export const clippingUi = new ClippingUiStore();
