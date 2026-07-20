<script lang="ts">
  interface Props {
    mode: "silence" | "clips";
    onMode: (m: "silence" | "clips") => void;
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
    class="flex w-full flex-col items-start gap-0.5 rounded-xl border px-3 py-3 text-left transition
      {mode === 'silence'
      ? 'border-vigil-500/70 bg-vigil-950/60 text-white'
      : 'border-surface-800 bg-surface-900/40 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("silence")}
  >
    <span class="text-base font-bold">✂ Silencios</span>
    <span class="text-[10px] leading-snug opacity-75">Cortar pausas y exportar el vídeo largo</span>
  </button>

  <button
    type="button"
    class="flex w-full flex-col items-start gap-0.5 rounded-xl border px-3 py-3 text-left transition
      {mode === 'clips'
      ? 'border-amber-500/60 bg-amber-950/50 text-white'
      : 'border-surface-800 bg-surface-900/40 text-surface-400 hover:border-surface-600 hover:text-surface-200'}"
    onclick={() => onMode("clips")}
  >
    <span class="text-base font-bold">📱 Shorts 9:16</span>
    <span class="text-[10px] leading-snug opacity-75">Clips virales · revisar · exportar vertical</span>
  </button>

  <div class="mt-auto rounded-lg border border-surface-800/80 bg-surface-900/50 p-2 text-[9px] leading-relaxed text-surface-500">
    {#if mode === "silence"}
      1) Abre video<br />
      2) Revisa excepciones<br />
      3) ▶ Oír cortado<br />
      4) Exportar
    {:else}
      1) Sacar clips<br />
      2) Ver 9:16 al centro<br />
      3) Aprobar / Descartar<br />
      4) Exportar 9:16
    {/if}
  </div>
</aside>
