<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isTauri } from "$lib/utils/tauri";
  import type { ClipFraming } from "$lib/types";
  import { projectStore } from "$lib/stores/project.svelte";

  interface Props {
    framing: ClipFraming;
    /** When set, seek this time; else follow project playhead */
    time?: number | null;
  }
  let { framing, time = null }: Props = $props();

  let canvasEl = $state<HTMLCanvasElement | null>(null);
  let videoEl = $state<HTMLVideoElement | null>(null);
  let ready = $state(false);
  let drawError = $state<string | null>(null);

  const src = $derived.by(() => {
    const p = projectStore.mediaPath;
    if (!p || p.startsWith("demo://")) return null;
    if (isTauri()) {
      try {
        return convertFileSrc(p.replace(/\\/g, "/"));
      } catch {
        return null;
      }
    }
    return null;
  });

  const displayTime = $derived(
    typeof time === "number" && Number.isFinite(time) ? time : projectStore.currentTime,
  );

  $effect(() => {
    const v = videoEl;
    const url = src;
    ready = false;
    drawError = null;
    if (!v || !url) return;
    v.src = url;
    v.load();
  });

  $effect(() => {
    const v = videoEl;
    const c = canvasEl;
    const t = displayTime;
    const f = framing;
    if (!v || !c || !src) return;

    const draw = () => {
      try {
        const vw = v.videoWidth || 0;
        const vh = v.videoHeight || 0;
        if (vw < 2 || vh < 2) return;

        const outW = 108;
        const outH = 192;
        c.width = outW;
        c.height = outH;
        const ctx = c.getContext("2d");
        if (!ctx) return;

        ctx.fillStyle = "#000";
        ctx.fillRect(0, 0, outW, outH);

        if (f.mode === "fit_with_bars") {
          const scale = Math.min(outW / vw, outH / vh);
          const dw = vw * scale;
          const dh = vh * scale;
          ctx.drawImage(v, (outW - dw) / 2, (outH - dh) / 2, dw, dh);
        } else if (f.mode === "blurred_background") {
          // Approximate: cover scale + slight dim (true blur is expensive in canvas without filter)
          const cover = Math.max(outW / vw, outH / vh);
          const dw = vw * cover;
          const dh = vh * cover;
          ctx.filter = "blur(6px) brightness(0.55)";
          ctx.drawImage(v, (outW - dw) / 2, (outH - dh) / 2, dw, dh);
          ctx.filter = "none";
          const fit = Math.min(outW / vw, outH / vh) * 0.92;
          const fw = vw * fit;
          const fh = vh * fit;
          ctx.drawImage(v, (outW - fw) / 2, (outH - fh) / 2, fw, fh);
        } else {
          // auto_center / manual crop 9:16
          const targetAr = 9 / 16;
          const zoom = Math.max(1, f.zoom || 1);
          let cropH = vh / zoom;
          let cropW = cropH * targetAr;
          if (cropW > vw) {
            cropW = vw;
            cropH = cropW / targetAr;
          }
          const cx = (f.centerX ?? 0.5) * vw;
          const cy = (f.centerY ?? 0.45) * vh;
          let sx = cx - cropW / 2;
          let sy = cy - cropH / 2;
          sx = Math.max(0, Math.min(vw - cropW, sx));
          sy = Math.max(0, Math.min(vh - cropH, sy));
          ctx.drawImage(v, sx, sy, cropW, cropH, 0, 0, outW, outH);
        }

        // Safe zone for face (upper-mid)
        ctx.strokeStyle = "rgba(52, 211, 153, 0.55)";
        ctx.lineWidth = 1;
        ctx.strokeRect(outW * 0.12, outH * 0.14, outW * 0.76, outH * 0.42);

        // Center cross
        ctx.strokeStyle = "rgba(16, 185, 129, 0.9)";
        const mx = (f.centerX ?? 0.5) * outW;
        const my = (f.centerY ?? 0.45) * outH;
        ctx.beginPath();
        ctx.moveTo(mx - 4, my);
        ctx.lineTo(mx + 4, my);
        ctx.moveTo(mx, my - 4);
        ctx.lineTo(mx, my + 4);
        ctx.stroke();

        ready = true;
        drawError = null;
      } catch (e) {
        drawError = String(e);
      }
    };

    const seekAndDraw = () => {
      const target = Math.max(0, t);
      const onSeeked = () => {
        v.removeEventListener("seeked", onSeeked);
        draw();
      };
      if (Math.abs(v.currentTime - target) > 0.04) {
        v.addEventListener("seeked", onSeeked);
        try {
          v.currentTime = target;
        } catch {
          draw();
        }
      } else if (v.readyState >= 2) {
        draw();
      } else {
        v.addEventListener("loadeddata", () => draw(), { once: true });
      }
    };

    if (v.readyState >= 1) seekAndDraw();
    else v.addEventListener("loadedmetadata", seekAndDraw, { once: true });
  });
</script>

<div class="flex flex-col items-center gap-1">
  <div
    class="relative overflow-hidden rounded-lg border border-vigil-700/60 bg-black shadow-lg"
    style="width: 108px; height: 192px"
  >
    <canvas bind:this={canvasEl} class="h-full w-full"></canvas>
    <!-- Hidden video source for frame sampling -->
    <!-- svelte-ignore a11y_media_has_caption -->
    <video bind:this={videoEl} class="hidden" muted playsinline preload="auto"></video>
    {#if !ready && !drawError}
      <div
        class="absolute inset-0 flex items-center justify-center text-[9px] text-surface-500"
      >
        …
      </div>
    {/if}
  </div>
  {#if drawError}
    <p class="text-[9px] text-cut">{drawError}</p>
  {/if}
  <p class="text-[9px] text-surface-500">Preview real 9:16</p>
</div>
