import type { ClipCandidate, ClipFraming } from "$lib/types";

type FramingSaver = (clipId: string, framing: ClipFraming) => Promise<void> | void;

function clamp01(v: number, lo = 0.05, hi = 0.95) {
  return Math.min(hi, Math.max(lo, v));
}

/** Shared selection + live 9:16 focus for ShortPlayer. */
class ClippingUiStore {
  selected = $state<ClipCandidate | null>(null);
  /** Live crop focus — separate from nested framing so UI always updates */
  focusX = $state(0.5);
  focusY = $state(0.42);
  playToken = $state(0);

  private framingSaver: FramingSaver | null = null;
  private saveTimer: ReturnType<typeof setTimeout> | null = null;
  /** While dragging, panel must not overwrite focus */
  dragging = $state(false);

  select(c: ClipCandidate | null) {
    this.selected = c;
    if (c && !this.dragging) {
      this.focusX = clamp01(c.framing?.centerX ?? 0.5);
      this.focusY = clamp01(c.framing?.centerY ?? 0.42);
    }
  }

  /** Update selected metadata without resetting live focus (used by panel sync). */
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
  }

  beginDrag() {
    this.dragging = true;
  }

  endDrag() {
    this.dragging = false;
    this.flushSave();
  }

  /**
   * Set focus immediately (0..1). Updates video crop + debounced persist.
   */
  setFocus(centerX: number, centerY: number) {
    const x = clamp01(centerX);
    const y = clamp01(centerY);
    this.focusX = x;
    this.focusY = y;

    const c = this.selected;
    if (!c) return;

    const framing: ClipFraming = {
      ...c.framing,
      mode: "manual",
      centerX: x,
      centerY: y,
    };
    this.selected = { ...c, framing };

    if (this.saveTimer) clearTimeout(this.saveTimer);
    this.saveTimer = setTimeout(() => this.flushSave(), this.dragging ? 250 : 80);
  }

  nudge(dx: number, dy: number) {
    this.setFocus(this.focusX + dx, this.focusY + dy);
  }

  resetFraming() {
    this.setFocus(0.5, 0.42);
  }

  private flushSave() {
    if (this.saveTimer) {
      clearTimeout(this.saveTimer);
      this.saveTimer = null;
    }
    const cur = this.selected;
    if (!cur || !this.framingSaver) return;
    void this.framingSaver(cur.id, {
      ...cur.framing,
      mode: "manual",
      centerX: this.focusX,
      centerY: this.focusY,
    });
  }
}

export const clippingUi = new ClippingUiStore();
