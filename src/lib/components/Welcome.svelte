<script lang="ts">
  interface Props {
    onOpen: () => void;
    onDemo: () => void;
  }
  let { onOpen, onDemo }: Props = $props();
</script>

<div class="flex flex-1 flex-col items-center justify-center gap-6 p-8 text-center">
  <div
    class="flex h-20 w-20 items-center justify-center rounded-2xl bg-gradient-to-br from-vigil-500 to-emerald-800 text-3xl font-bold text-white shadow-panel"
  >
    V
  </div>
  <div>
    <h1 class="text-2xl font-semibold tracking-tight text-white">VigilCut</h1>
    <p class="mt-2 max-w-md text-sm text-surface-400">
      Editor de video local con <strong class="text-surface-200">supervisión humana</strong>.
      Auto-detecta silencios, tú apruebas cada corte.
    </p>
    <p class="mt-1 max-w-md text-xs text-surface-600">
      Local open-source video editor — silence detection, audio enhance, color, subtitles — you stay
      in control.
    </p>
  </div>

  <div class="flex flex-wrap items-center justify-center gap-3">
    <button class="btn-primary px-5 py-2" onclick={onOpen}>Abrir video / Open video</button>
    <button class="btn-secondary px-5 py-2" onclick={onDemo}>Cargar demo timeline</button>
  </div>

  <div class="mt-4 grid max-w-2xl grid-cols-1 gap-3 text-left sm:grid-cols-3">
    {#each [
      { t: "1. Detectar", d: "Silero VAD + FFmpeg marcan silencios y habla." },
      { t: "2. Revisar", d: "Toggle keep/cut en el timeline. Ajusta padding." },
      { t: "3. Exportar", d: "Preview salta cortes. Export local con FFmpeg." },
    ] as step}
      <div class="panel p-3">
        <div class="text-xs font-semibold text-vigil-400">{step.t}</div>
        <div class="mt-1 text-[11px] text-surface-400">{step.d}</div>
      </div>
    {/each}
  </div>
</div>
