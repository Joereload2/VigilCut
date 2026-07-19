<script lang="ts">
  interface Props {
    mode: "silence" | "clips";
    onMode: (m: "silence" | "clips") => void;
  }
  let { mode, onMode }: Props = $props();
</script>

<nav
  class="flex shrink-0 flex-wrap items-center gap-2 border-b border-surface-800 bg-surface-950/90 px-3 py-2"
  aria-label="Modo de trabajo"
>
  <span class="mr-1 text-[10px] font-semibold uppercase tracking-wide text-surface-500"
    >Qué quieres hacer</span
  >

  <button
    type="button"
    class="flex items-center gap-2 rounded-xl border px-3 py-2 text-left transition
      {mode === 'silence'
      ? 'border-vigil-500/60 bg-vigil-950/50 text-white shadow-md shadow-vigil-950/30'
      : 'border-surface-700 bg-surface-900/60 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("silence")}
  >
    <span class="text-lg leading-none">✂</span>
    <span>
      <span class="block text-xs font-bold">Cortar silencios</span>
      <span class="block text-[10px] opacity-70">Limpiar pausas · oír resultado · exportar</span>
    </span>
  </button>

  <button
    type="button"
    class="flex items-center gap-2 rounded-xl border px-3 py-2 text-left transition
      {mode === 'clips'
      ? 'border-amber-500/50 bg-amber-950/40 text-white shadow-md shadow-amber-950/20'
      : 'border-surface-700 bg-surface-900/60 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("clips")}
  >
    <span class="text-lg leading-none">📱</span>
    <span>
      <span class="block text-xs font-bold">Shorts / clips 9:16</span>
      <span class="block text-[10px] opacity-70">Momentos fuertes · revisar · exportar vertical</span>
    </span>
  </button>
</nav>
