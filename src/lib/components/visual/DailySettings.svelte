<script lang="ts">
  let {
    enabled = false,
    weekMsg = "",
    busy = false,
    onToggle,
    onRunNow,
  }: {
    enabled?: boolean;
    weekMsg?: string;
    busy?: boolean;
    onToggle: (v: boolean) => void;
    onRunNow: () => void;
  } = $props();
</script>

<div class="space-y-2 text-[11px]">
  <p class="font-medium text-surface-100">Biblioteca automática</p>
  <p class="text-[10px] text-surface-400">
    Mientras la app está abierta, puede preparar imágenes solo con simulación (mock) o rutas gratis
    verificadas. Los resultados aparecen en <strong class="text-surface-300">Por revisar</strong>.
  </p>
  <label class="flex cursor-pointer items-center gap-2 text-surface-200">
    <input
      type="checkbox"
      class="rounded"
      checked={enabled}
      disabled={busy}
      onchange={(e) => onToggle((e.currentTarget as HTMLInputElement).checked)}
    />
    Activar alimentación automática
  </label>
  {#if weekMsg}
    <p class="text-[10px] text-surface-500">{weekMsg}</p>
  {/if}
  <button type="button" class="btn-secondary text-[10px]" disabled={busy || !enabled} onclick={onRunNow}>
    Ejecutar un ciclo ahora
  </button>
</div>
