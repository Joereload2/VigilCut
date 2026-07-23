import type { ClipCandidate, ClipFraming } from "$lib/types";

type FramingSaver = (clipId: string, framing: ClipFraming) => Promise<void> | void;

function clamp01(v: number, lo = 0.05, hi = 0.95) {
  return Math.min(hi, Math.max(lo, v));
}

/** Shared selection for the main 9:16 short player (clips workspace). */
class ClippingUiStore {
  selected = $state<ClipCandidate | null>(null);
  focusX = $state(0.5);
  focusY = $state(0.42);
  playToken = $state(0);
  dragging = $state(false);

  /** Set by ShortPlayer / ClippingPanel for persistence */
  persistFraming: FramingSaver | null = null;

  private framingSaver: FramingSaver | null = null;

  select(c: ClipCandidate | null) {
    this.selected = c;
    if (c && !this.dragging) {
      this.focusX = clamp01(c.framing?.centerX ?? 0.5);
      this.focusY = clamp01(c.framing?.centerY ?? 0.42);
    }
  }

  mergeSelected(c: ClipCandidate) {
    this.selected = {
      ...c,
      framing: {
        ...c.framing,
        mode: "manual",
        centerX: this.focusX,
        centerY: this.focusY,
      },
    };
  }

  play(c: ClipCandidate) {
    this.dragging = false;
    this.selected = c;
    this.focusX = clamp01(c.framing?.centerX ?? 0.5);
    this.focusY = clamp01(c.framing?.centerY ?? 0.42);
    this.playToken += 1;
  }

  get framing(): ClipFraming | null {
    return this.selected?.framing ?? null;
  }

  setFramingSaver(fn: FramingSaver | null) {
    this.framingSaver = fn;
    this.persistFraming = fn;
  }

  beginDrag() {
    this.dragging = true;
  }

  endDrag() {
    this.dragging = false;
  }

  setFocus(centerX: number, centerY: number) {
    this.focusX = clamp01(centerX);
    this.focusY = clamp01(centerY);
    const c = this.selected;
    if (!c) return;
    this.selected = {
      ...c,
      framing: {
        ...c.framing,
        mode: "manual",
        centerX: this.focusX,
        centerY: this.focusY,
      },
    };
  }

  nudge(dx: number, dy: number) {
    this.setFocus(this.focusX + dx, this.focusY + dy);
  }

  resetFraming() {
    this.setFocus(0.5, 0.42);
  }
}

export const clippingUi = new ClippingUiStore();
