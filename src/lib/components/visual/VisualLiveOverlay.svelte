<script lang="ts">
  /**
   * Live composition on the main video.
   * Geometry: center_norm_v1 — same as Rust `pipeline::visual::layout` / FFmpeg bake.
   */
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { projectStore } from "$lib/stores/project.svelte";
  import type { CompositionIssue, SpatialZone } from "./types";
  import { isFullscreenMode } from "./types";
  import { placementCssBox } from "./layout";

  interface Props {
    issues?: CompositionIssue[];
    spatialZones?: SpatialZone[];
    showZones?: boolean;
    onLayoutChange?: (id: string, x: number, y: number, w: number, h?: number) => void;
  }

  let {
    issues = [],
    spatialZones = [],
    showZones = false,
    onLayoutChange,
  }: Props = $props();

  const t = $derived(projectStore.outputClock());

  const active = $derived(
    projectStore.visualPlacements.filter(
      (p) =>
        p.status === "active" &&
        t >= p.outputStart - 0.05 &&
        t < p.outputEnd + 0.05 &&
        !!p.imagePath,
    ),
  );

  const selected = $derived(
    projectStore.visualPlacements.find((p) => p.id === projectStore.visualSelectedId) ?? null,
  );
  const selectedOffClock = $derived(
    !!selected &&
      selected.status === "active" &&
      (t < selected.outputStart - 0.05 || t >= selected.outputEnd + 0.05),
  );

  const zonesNow = $derived(
    spatialZones.length > 0 ? spatialZones : projectStore.visualSpatialZones,
  );

  const showZonesNow = $derived.by(() => {
    if (showZones || projectStore.visualShowZones) return true;
    if (active.some((p) => !isFullscreenMode(p.mode))) {
      return issues.some(
        (i) =>
          (i.severity === "warn" || i.severity === "error") &&
          (i.kind.includes("face") ||
            i.kind.includes("subtitle") ||
            i.kind.includes("safe") ||
            i.kind.includes("covered")),
      );
    }
    return false;
  });

  const visibleZones = $derived(
    showZonesNow
      ? zonesNow.filter((z) => {
          if (z.outputStart != null && t < z.outputStart) return false;
          if (z.outputEnd != null && t >= z.outputEnd) return false;
          return true;
        })
      : [],
  );

  function imgUrl(path?: string | null): string | null {
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      try {
        return convertFileSrc(path.replace(/\\/g, "/"));
      } catch {
        return null;
      }
    }
  }

  function borderFor(id: string, full: boolean): string {
    const mine = issues.filter((i) => i.placementId === id);
    if (mine.some((i) => i.severity === "error")) return "ring-2 ring-cut";
    if (mine.some((i) => i.severity === "warn")) return "ring-2 ring-amber-400";
    if (projectStore.visualSelectedId === id && !full) return "ring-2 ring-sky-400";
    return full ? "" : "ring-1 ring-white/20";
  }

  function seekToPlacement(pl: { outputStart: number }) {
    if (projectStore.localKeepRanges().length > 0) {
      projectStore.previewMode = "edited";
      projectStore.currentTime = projectStore.editedToSource(pl.outputStart);
    } else {
      projectStore.currentTime = pl.outputStart;
    }
    projectStore.isPlaying = false;
  }

  /** Box style from shared layout contract (top-left %, same as FFmpeg pixels). */
  function boxStyle(p: (typeof active)[0]): string {
    const b = placementCssBox(p);
    if (b.fullframe) {
      return `inset:0;width:100%;height:100%;object-fit:${b.objectFit};opacity:${b.opacity}`;
    }
    return `left:${b.left}%;top:${b.top}%;width:${b.width}%;height:${b.height}%;object-fit:${b.objectFit};opacity:${b.opacity}`;
  }

  // Drag updates center (layout.x/y) — same as store/backend
  let interact = $state<null | {
    id: string;
    kind: "move" | "resize";
    startX: number;
    startY: number;
    ox: number;
    oy: number;
    ow: number;
  }>(null);

  function onDown(e: PointerEvent, id: string, kind: "move" | "resize") {
    const pl = active.find((p) => p.id === id);
    if (!pl || isFullscreenMode(pl.mode)) return;
    e.preventDefault();
    e.stopPropagation();
    projectStore.visualSelectedId = id;
    interact = {
      id,
      kind,
      startX: e.clientX,
      startY: e.clientY,
      ox: pl.layout?.x ?? 0.5,
      oy: pl.layout?.y ?? 0.5,
      ow: pl.layout?.w ?? 0.3,
    };
    (e.currentTarget as HTMLElement).setPointerCapture?.(e.pointerId);
  }

  function onMove(e: PointerEvent) {
    if (!interact || !onLayoutChange) return;
    const host = (e.currentTarget as HTMLElement)?.closest?.(
      "[data-compose-host]",
    ) as HTMLElement | null;
    const box = host ?? (e.currentTarget as HTMLElement);
    const r = box.getBoundingClientRect();
    const dx = (e.clientX - interact.startX) / Math.max(1, r.width);
    const dy = (e.clientY - interact.startY) / Math.max(1, r.height);
    if (interact.kind === "move") {
      onLayoutChange(
        interact.id,
        Math.min(1, Math.max(0, interact.ox + dx)),
        Math.min(1, Math.max(0, interact.oy + dy)),
        interact.ow,
      );
    } else {
      const nw = Math.min(0.9, Math.max(0.1, interact.ow + dx * 1.2));
      onLayoutChange(interact.id, interact.ox, interact.oy, nw);
    }
  }

  function onUp() {
    interact = null;
  }

  function zoneStyle(z: SpatialZone): string {
    return `left:${z.x * 100}%;top:${z.y * 100}%;width:${z.w * 100}%;height:${z.h * 100}%;`;
  }

  function zoneColor(z: SpatialZone): string {
    if (z.kind === "face") return "border-rose-400/70 bg-rose-500/10";
    if (z.kind === "subtitle" || z.kind === "safe_area")
      return "border-amber-400/60 bg-amber-500/10";
    if (z.kind === "logo" || z.kind === "product") return "border-violet-400/60 bg-violet-500/10";
    return "border-sky-400/50 bg-sky-500/10";
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="absolute inset-0 z-[15] overflow-hidden"
  class:pointer-events-none={!active.some((p) => !isFullscreenMode(p.mode))}
  data-compose-host
  onpointermove={onMove}
  onpointerup={onUp}
  onpointercancel={onUp}
>
  {#each visibleZones as z (z.id)}
    <div
      class="pointer-events-none absolute rounded border border-dashed {zoneColor(z)}"
      style={zoneStyle(z)}
      title={z.label || z.kind}
    >
      <span
        class="absolute left-0.5 top-0.5 rounded bg-black/50 px-1 text-[8px] uppercase text-white/80"
        >{z.kind}</span
      >
    </div>
  {/each}

  {#each active as p (p.id)}
    {@const u = imgUrl(p.imagePath)}
    {@const full = isFullscreenMode(p.mode)}
    {@const b = placementCssBox(p)}
    {#if u}
      {#if full}
        <img
          src={u}
          alt=""
          class="pointer-events-none absolute inset-0 z-[16] h-full w-full {borderFor(p.id, true)}"
          style="object-fit:{b.objectFit};opacity:{b.opacity}"
          draggable="false"
        />
      {:else}
        <!-- top-left % box — identical contract to FFmpeg overlay x/y -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="pointer-events-auto absolute z-[16] cursor-move overflow-hidden rounded shadow-lg {borderFor(
            p.id,
            false,
          )}"
          style="left:{b.left}%;top:{b.top}%;width:{b.width}%;height:{b.height}%"
          onpointerdown={(e) => onDown(e, p.id, "move")}
        >
          <img
            src={u}
            alt=""
            class="h-full w-full"
            style="object-fit:{b.objectFit};opacity:{b.opacity}"
            draggable="false"
          />
          {#if projectStore.visualSelectedId === p.id}
            <div
              class="absolute -bottom-1 -right-1 h-3.5 w-3.5 cursor-se-resize rounded-sm border border-white bg-sky-500"
              onpointerdown={(e) => onDown(e, p.id, "resize")}
            ></div>
          {/if}
        </div>
      {/if}
    {/if}
  {/each}

  {#if selectedOffClock && selected}
    <div class="pointer-events-auto absolute bottom-3 left-1/2 z-[25] -translate-x-1/2">
      <button
        type="button"
        class="rounded-full border border-sky-500/60 bg-sky-950/95 px-3 py-1.5 text-[11px] font-semibold text-sky-100 shadow-lg hover:bg-sky-900"
        onclick={() => seekToPlacement(selected)}
      >
        B-roll {selected.outputStart.toFixed(1)}–{selected.outputEnd.toFixed(1)}s · Saltar y ver
      </button>
    </div>
  {/if}
</div>
