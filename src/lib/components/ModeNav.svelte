<script lang="ts">
  interface Props {
    mode: "silence" | "clips" | "visual";
    onMode: (m: "silence" | "clips" | "visual") => void;
  }
  let { mode, onMode }: Props = $props();
</script>

<aside
  class="flex w-[168px] shrink-0 flex-col gap-2 border-r border-surface-800 bg-surface-950 p-2.5"
  aria-label="Modo de trabajo"
>
  <div class="mb-0.5 px-1 text-[10px] font-bold uppercase tracking-wider text-surface-500">
    Modo
  </div>

  <button
    type="button"
    class="flex w-full flex-col items-start gap-0.5 rounded-xl border px-3 py-2.5 text-left transition
      {mode === 'silence'
      ? 'border-vigil-500/70 bg-vigil-950/60 text-white'
      : 'border-surface-800 bg-surface-900/40 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("silence")}
  >
    <span class="text-sm font-bold">✂ Silencios</span>
    <span class="text-[10px] leading-snug opacity-75">Cortar pausas</span>
  </button>

  <button
    type="button"
    class="flex w-full flex-col items-start gap-0.5 rounded-xl border px-3 py-2.5 text-left transition
      {mode === 'clips'
      ? 'border-amber-500/60 bg-amber-950/50 text-white'
      : 'border-surface-800 bg-surface-900/40 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("clips")}
  >
    <span class="text-sm font-bold">📱 Shorts 9:16</span>
    <span class="text-[10px] leading-snug opacity-75">Clips verticales</span>
  </button>

  <button
    type="button"
    class="flex w-full flex-col items-start gap-0.5 rounded-xl border px-3 py-2.5 text-left transition
      {mode === 'visual'
      ? 'border-sky-500/60 bg-sky-950/40 text-white'
      : 'border-surface-800 bg-surface-900/40 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("visual")}
  >
    <span class="text-sm font-bold">🖼 Visual</span>
    <span class="text-[10px] leading-snug opacity-75">B-roll / biblioteca</span>
  </button>

  <div class="mt-auto rounded-lg border border-surface-800/80 bg-surface-900/50 p-2 text-[9px] leading-relaxed text-surface-500">
    {#if mode === "silence"}
      1) Analiza<br />2) Supervisa<br />3) Oye<br />4) Exporta
    {:else if mode === "clips"}
      1) Sacar clips<br />2) Clasificar<br />3) Export 9:16
    {:else}
      1) Analiza silencios<br />2) + Imágenes<br />3) Sugerencias<br />4) Aceptar · Render
    {/if}
  </div>
</aside>
