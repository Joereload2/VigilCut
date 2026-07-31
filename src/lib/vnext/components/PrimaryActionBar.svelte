<script lang="ts">
  let {
    label,
    disabled = false,
    pending = false,
    pendingLabel = "Trabajando…",
    secondaryLabel = null as string | null,
    secondaryDisabled = false,
    onPrimary,
    onSecondary,
  }: {
    label: string;
    disabled?: boolean;
    pending?: boolean;
    pendingLabel?: string;
    secondaryLabel?: string | null;
    secondaryDisabled?: boolean;
    onPrimary: () => void;
    onSecondary?: () => void;
  } = $props();
</script>

<div
  class="flex shrink-0 flex-wrap items-center justify-center gap-3 border-t border-surface-800/80 bg-surface-950/95 px-4 py-3"
  data-testid="primary-action-bar"
>
  {#if secondaryLabel && onSecondary}
    <button
      type="button"
      class="rounded-xl border border-surface-700 px-4 py-2.5 text-sm text-surface-300 hover:bg-surface-900 disabled:opacity-40"
      disabled={secondaryDisabled || pending}
      onclick={onSecondary}
      data-testid="cta-secondary"
    >
      {secondaryLabel}
    </button>
  {/if}
  <button
    type="button"
    class="min-w-[12rem] rounded-xl bg-vigil-600 px-6 py-2.5 text-sm font-semibold text-white shadow-lg shadow-vigil-900/30 hover:bg-vigil-500 disabled:opacity-40"
    disabled={disabled || pending}
    onclick={onPrimary}
    data-testid="cta-primary"
  >
    {pending ? pendingLabel : label}
  </button>
</div>
