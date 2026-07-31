<script lang="ts">
  let {
    title = "Algo salió mal",
    message,
    onRetry = undefined as (() => void) | undefined,
    onBack = undefined as (() => void) | undefined,
  }: {
    title?: string;
    message: string;
    onRetry?: () => void;
    onBack?: () => void;
  } = $props();
</script>

<div class="mx-auto max-w-md rounded-2xl border border-cut/40 bg-cut/10 p-6 text-center" data-testid="recoverable-error">
  <h2 class="text-lg font-semibold text-white">{title}</h2>
  <p class="mt-2 text-sm text-surface-300">{message}</p>
  <div class="mt-5 flex justify-center gap-2">
    {#if onBack}
      <button type="button" class="btn-secondary text-xs" onclick={onBack}>Volver</button>
    {/if}
    {#if onRetry}
      <button type="button" class="btn-primary text-xs" onclick={onRetry}>Reintentar</button>
    {/if}
  </div>
</div>
