<script lang="ts">
  /** Horizontal work-mode switcher for the top bar (no left sidebar). */
  interface Props {
    mode: "silence" | "clips" | "visual" | "library";
    onMode: (m: "silence" | "clips" | "visual" | "library") => void;
  }
  let { mode, onMode }: Props = $props();
</script>

<div
  class="flex min-w-0 max-w-full items-center gap-0.5 overflow-x-auto rounded-lg border border-surface-800 bg-surface-950 p-0.5"
  role="tablist"
  aria-label="Modo de trabajo"
>
  <button
    type="button"
    role="tab"
    aria-selected={mode === "silence"}
    class="shrink-0 rounded-md px-2.5 py-1 text-[11px] font-semibold transition sm:px-3 sm:text-xs
      {mode === 'silence'
      ? 'bg-vigil-600 text-white'
      : 'text-surface-400 hover:bg-surface-800 hover:text-surface-200'}"
    onclick={() => onMode("silence")}
  >
    Silencios
  </button>
  <button
    type="button"
    role="tab"
    aria-selected={mode === "clips"}
    class="shrink-0 rounded-md px-2.5 py-1 text-[11px] font-semibold transition sm:px-3 sm:text-xs
      {mode === 'clips'
      ? 'bg-amber-600 text-white'
      : 'text-surface-400 hover:bg-surface-800 hover:text-surface-200'}"
    onclick={() => onMode("clips")}
  >
    Shorts 9:16
  </button>
  <button
    type="button"
    role="tab"
    aria-selected={mode === "visual"}
    class="shrink-0 rounded-md px-2.5 py-1 text-[11px] font-semibold transition sm:px-3 sm:text-xs
      {mode === 'visual'
      ? 'bg-sky-600 text-white'
      : 'text-surface-400 hover:bg-surface-800 hover:text-surface-200'}"
    onclick={() => onMode("visual")}
  >
    Visual/B-roll
  </button>
  <button
    type="button"
    role="tab"
    aria-selected={mode === "library"}
    class="shrink-0 rounded-md px-2.5 py-1 text-[11px] font-semibold transition sm:px-3 sm:text-xs
      {mode === 'library'
      ? 'bg-violet-600 text-white'
      : 'text-surface-400 hover:bg-surface-800 hover:text-surface-200'}"
    onclick={() => onMode("library")}
  >
    Biblioteca
  </button>
</div>
