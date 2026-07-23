<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { DisplayMode, VisualPlacement } from "./types";

  interface Props {
    placement: VisualPlacement | null;
    thumbPath?: string | null;
    busy?: boolean;
    onUpdate: (patch: {
      outputStart?: number;
      outputEnd?: number;
      displayMode?: string;
      positionX?: number;
      positionY?: number;
      sizeW?: number;
    }) => void;
    onRemove: () => void;
  }

  let { placement, thumbPath = null, busy = false, onUpdate, onRemove }: Props = $props();

  function thumbUrl(p?: string | null) {
    if (!p) return null;
    try {
      return convertFileSrc(p.replace(/\\/g, "/"));
    } catch {
      return null;
    }
  }

  const url = $derived(thumbUrl(thumbPath));
</script>

{#if !placement}
  <section class="rounded-xl border border-dashed border-surface-800 p-3 text-[10px] text-surface-500">
    Selecciona un bloque en la pista para editar duración, modo y posición.
  </section>
{:else}
  <section class="space-y-2 rounded-xl border border-surface-800 bg-surface-950/60 p-3">
    <div class="flex items-center gap-2">
      {#if url}
        <img src={url} alt="" class="h-12 w-12 rounded object-cover ring-1 ring-surface-700" />
      {/if}
      <div class="min-w-0 flex-1">
        <div class="truncate text-xs font-semibold text-surface-100">
          {placement.label || placement.assetId}
        </div>
        <div class="font-mono text-[9px] text-surface-500">
          {placement.outputStart.toFixed(1)}–{placement.outputEnd.toFixed(1)}s · {placement.mode}
        </div>
      </div>
    </div>

    <div class="grid grid-cols-2 gap-2 text-[10px]">
      <label class="text-surface-400">
        Inicio
        <input
          type="number"
          step="0.1"
          class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
          value={placement.outputStart.toFixed(1)}
          onchange={(e) =>
            onUpdate({ outputStart: parseFloat((e.currentTarget as HTMLInputElement).value) })}
        />
      </label>
      <label class="text-surface-400">
        Fin
        <input
          type="number"
          step="0.1"
          class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
          value={placement.outputEnd.toFixed(1)}
          onchange={(e) =>
            onUpdate({ outputEnd: parseFloat((e.currentTarget as HTMLInputElement).value) })}
        />
      </label>
    </div>

    <div class="flex flex-wrap gap-1">
      {#each [
        { id: "completa", label: "Completa" },
        { id: "parcial", label: "Parcial" },
        { id: "flotante", label: "Flotante" },
      ] as m}
        <button
          type="button"
          class="rounded border px-2 py-0.5 text-[10px]
            {placement.mode === m.id ||
            (m.id === 'completa' && placement.mode === 'fullframe') ||
            (m.id === 'parcial' && placement.mode.includes('picture')) ||
            (m.id === 'flotante' && placement.mode.includes('lower'))
              ? 'border-sky-400 bg-sky-700 text-white'
              : 'border-surface-700 text-surface-400'}"
          onclick={() => onUpdate({ displayMode: m.id })}
        >
          {m.label}
        </button>
      {/each}
    </div>

    {#if placement.mode !== "fullframe" && placement.mode !== "completa"}
      <div class="grid grid-cols-3 gap-1 text-[10px]">
        <label class="text-surface-500">
          X
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={placement.layout?.x ?? 0.5}
            oninput={(e) =>
              onUpdate({ positionX: parseFloat((e.currentTarget as HTMLInputElement).value) })}
            class="w-full"
          />
        </label>
        <label class="text-surface-500">
          Y
          <input
            type="range"
            min="0"
            max="1"
            step="0.01"
            value={placement.layout?.y ?? 0.5}
            oninput={(e) =>
              onUpdate({ positionY: parseFloat((e.currentTarget as HTMLInputElement).value) })}
            class="w-full"
          />
        </label>
        <label class="text-surface-500">
          Tamaño
          <input
            type="range"
            min="0.1"
            max="0.8"
            step="0.01"
            value={placement.layout?.w ?? 0.3}
            oninput={(e) =>
              onUpdate({ sizeW: parseFloat((e.currentTarget as HTMLInputElement).value) })}
            class="w-full"
          />
        </label>
      </div>
    {/if}

    <button type="button" class="btn-ghost w-full text-[10px] text-cut" disabled={busy} onclick={onRemove}>
      Quitar del plan
    </button>
  </section>
{/if}
