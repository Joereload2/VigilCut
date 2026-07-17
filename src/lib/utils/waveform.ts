import type { Action } from "svelte/action";

/** Canvas action: draw mono peak waveform. */
export const drawWaveform: Action<HTMLCanvasElement, number[]> = (node, peaks) => {
  function paint(p: number[]) {
    const ctx = node.getContext("2d");
    if (!ctx || !p.length) return;
    const w = node.width;
    const h = node.height;
    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = "#3b82f6";
    const mid = h / 2;
    const step = p.length / w;
    for (let x = 0; x < w; x++) {
      const idx = Math.floor(x * step);
      const amp = (p[idx] ?? 0) * (mid - 2);
      ctx.fillRect(x, mid - amp, 1, amp * 2 || 1);
    }
  }
  paint(peaks);
  return {
    update(p: number[]) {
      paint(p);
    },
  };
};
