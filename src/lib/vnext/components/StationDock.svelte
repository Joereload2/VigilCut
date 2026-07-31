<script lang="ts">
  /**
   * Unified station footer — PM+UX: one next action, no 5-column wall of text.
   */
  import type { StationGuide } from "$lib/vnext/presentation/stations";

  let {
    guide,
    label = null as string | null,
    disabled = false,
    pending = false,
    pendingLabel = "Trabajando…",
    secondaryLabel = null as string | null,
    secondaryDisabled = false,
    /** Hide primary when wait-only stations */
    showPrimary = true,
    onPrimary,
    onSecondary,
    hint = null as string | null,
  }: {
    guide?: StationGuide | null;
    label?: string | null;
    disabled?: boolean;
    pending?: boolean;
    pendingLabel?: string;
    secondaryLabel?: string | null;
    secondaryDisabled?: boolean;
    showPrimary?: boolean;
    onPrimary?: () => void;
    onSecondary?: () => void;
    hint?: string | null;
  } = $props();

  const nextLine = $derived(
    hint ??
      (guide
        ? guide.primaryAction
          ? `Siguiente: ${guide.nextDo}`
          : guide.nextDo
        : null),
  );
  const cta = $derived(label ?? guide?.primaryAction ?? null);
</script>

<footer class="vn-dock" data-testid="station-dock">
  {#if nextLine}
    <p class="vn-dock-next" data-testid="dock-next">
      <strong>{nextLine}</strong>
      {#if guide?.missing && guide.primaryAction}
        <span class="text-surface-600"> · {guide.missing}</span>
      {/if}
    </p>
  {/if}
  <div class="flex flex-wrap items-center justify-center gap-2 sm:justify-end">
    {#if secondaryLabel && onSecondary}
      <button
        type="button"
        class="vn-cta-ghost"
        disabled={secondaryDisabled || pending}
        onclick={onSecondary}
        data-testid="cta-secondary"
      >
        {secondaryLabel}
      </button>
    {/if}
    {#if showPrimary && cta && onPrimary}
      <button
        type="button"
        class="vn-cta"
        disabled={disabled || pending}
        onclick={onPrimary}
        data-testid="cta-primary"
      >
        {pending ? pendingLabel : cta}
      </button>
    {/if}
  </div>
</footer>
