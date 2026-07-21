<script lang="ts">
  import type { DisplayMode } from "./types";

  interface Props {
    outputStart: number;
    outputEnd: number;
    displayMode: DisplayMode;
    duration: number;
    busy?: boolean;
    onPlaceFile: () => void;
    onProtect: () => void;
    onChangeStart: (v: number) => void;
    onChangeEnd: (v: number) => void;
    onChangeMode: (m: DisplayMode) => void;
    onUsePlayhead: () => void;
  }

  let {
    outputStart = $bindable(),
    outputEnd = $bindable(),
    displayMode = $bindable(),
    duration,
    busy = false,
    onPlaceFile,
    onProtect,
    onChangeStart,
    onChangeEnd,
    onChangeMode,
    onUsePlayhead,
  }: Props = $props();

  const maxT = $derived(Math.max(duration, 1));
</script>

<section class="space-y-2 rounded-xl border border-sky-800/40 bg-sky-950/20 p-3">
  <div class="text-xs font-semibold text-sky-100">Colocar imagen (manual)</div>
  <p class="text-[10px] leading-snug text-surface-500">
    Pausa el video, ajusta el intervalo y el modo. No hace falta transcripción.
  </p>

  <div class="grid grid-cols-2 gap-2 text-[10px]">
    <label class="block text-surface-400">
      Inicio (s)
      <input
        type="number"
        min="0"
        max={maxT}
        step="0.1"
        class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
        value={outputStart.toFixed(1)}
        onchange={(e) => onChangeStart(parseFloat((e.currentTarget as HTMLInputElement).value) || 0)}
      />
    </label>
    <label class="block text-surface-400">
      Fin (s)
      <input
        type="number"
        min="0"
        max={maxT}
        step="0.1"
        class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
        value={outputEnd.toFixed(1)}
        onchange={(e) => onChangeEnd(parseFloat((e.currentTarget as HTMLInputElement).value) || 0)}
      />
    </label>
  </div>

  <button type="button" class="btn-ghost w-full text-[10px]" onclick={onUsePlayhead}>
    Usar playhead (±2s)
  </button>

  <div class="flex flex-wrap gap-1">
    {#each [
      { id: "completa" as DisplayMode, label: "Completa" },
      { id: "parcial" as DisplayMode, label: "Parcial" },
      { id: "flotante" as DisplayMode, label: "Flotante" },
    ] as m}
      <button
        type="button"
        class="rounded-lg border px-2.5 py-1 text-[10px] font-semibold transition
          {displayMode === m.id
          ? 'border-sky-400 bg-sky-600 text-white'
          : 'border-surface-700 bg-surface-900 text-surface-400 hover:text-surface-200'}"
        onclick={() => onChangeMode(m.id)}
      >
        {m.label}
      </button>
    {/each}
  </div>

  <div class="flex flex-col gap-1.5">
    <button type="button" class="btn-primary text-xs" disabled={busy} onclick={onPlaceFile}>
      + Imagen en este intervalo
    </button>
    <button type="button" class="btn-secondary text-[10px]" disabled={busy} onclick={onProtect}>
      Proteger intervalo (sin B-roll)
    </button>
  </div>
</section>
