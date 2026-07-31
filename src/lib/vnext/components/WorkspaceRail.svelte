<script lang="ts">
  /**
   * Left tool rail from the operator canvas wireframe — only actions useful now.
   */
  let {
    onPrev,
    onNext,
    onDiscard,
    disablePrev = false,
    disableNext = false,
    disableDiscard = false,
  }: {
    onPrev: () => void;
    onNext: () => void;
    onDiscard: () => void;
    disablePrev?: boolean;
    disableNext?: boolean;
    disableDiscard?: boolean;
  } = $props();
</script>

<aside
  class="flex w-11 shrink-0 flex-col items-center justify-center gap-2 py-3 sm:w-12"
  data-testid="workspace-rail"
  aria-label="Acciones rápidas"
>
  <button
    type="button"
    class="h-9 w-9 rounded-lg bg-vigil-600/90 text-sm font-bold text-white shadow hover:bg-vigil-500 disabled:opacity-35"
    title="Clip anterior"
    disabled={disablePrev}
    onclick={onPrev}
    data-testid="rail-prev"
  >↑</button>
  <button
    type="button"
    class="h-9 w-9 rounded-lg bg-vigil-600/90 text-sm font-bold text-white shadow hover:bg-vigil-500 disabled:opacity-35"
    title="Clip siguiente"
    disabled={disableNext}
    onclick={onNext}
    data-testid="rail-next"
  >↓</button>
  <button
    type="button"
    class="h-9 w-9 rounded-lg border border-surface-600 bg-surface-900 text-[10px] font-semibold text-surface-200 hover:border-cut/60 hover:text-cut disabled:opacity-35"
    title="Descartar este clip"
    disabled={disableDiscard}
    onclick={onDiscard}
    data-testid="rail-discard"
  >✕</button>
</aside>
