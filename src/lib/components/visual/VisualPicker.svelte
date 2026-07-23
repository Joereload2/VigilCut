<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { MediaAsset } from "./library/libraryTypes";
  import type { NeedSupervision } from "./imageGenTypes";
  import { formatTimeRange } from "./imageGenTypes";

  let {
    open = false,
    need = null as NeedSupervision | null,
    matches = [] as MediaAsset[],
    matchesLoading = false,
    busy = false,
    pendingImportId = null as string | null,
    onClose,
    onUseAsset,
    onImport,
    onGenerate,
    onSkip,
  }: {
    open?: boolean;
    need?: NeedSupervision | null;
    matches?: MediaAsset[];
    matchesLoading?: boolean;
    busy?: boolean;
    pendingImportId?: string | null;
    onClose: () => void;
    onUseAsset: (assetId: string) => void;
    onImport: () => void;
    onGenerate: () => void;
    onSkip: () => void;
  } = $props();

  let moreOpen = $state(false);

  function thumb(a: MediaAsset) {
    const p = a.thumbnailPath || a.managedPath;
    if (!p) return null;
    try {
      return convertFileSrc(p);
    } catch {
      return null;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

{#if open && need}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="fixed inset-0 z-[80] flex items-end justify-center bg-black/50 p-2 sm:items-center"
    role="presentation"
    onkeydown={onKey}
    onclick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
  >
    <div
      class="flex max-h-[min(90vh,640px)] w-full max-w-md flex-col overflow-hidden rounded-xl border border-surface-700 bg-surface-900 shadow-2xl"
      role="dialog"
      aria-modal="true"
      aria-labelledby="picker-title"
    >
      <div class="flex items-start justify-between gap-2 border-b border-surface-800 p-3">
        <div class="min-w-0">
          <h2 id="picker-title" class="text-sm font-semibold text-surface-50">Buscar imagen</h2>
          <p class="text-[11px] text-surface-400">
            {formatTimeRange(need.need.outputStart, need.need.outputEnd)}
            · {need.need.label}
          </p>
        </div>
        <button type="button" class="btn-ghost text-[11px]" onclick={onClose}>Cerrar</button>
      </div>

      <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-3 text-[11px]">
        <section>
          <h3 class="mb-1 text-[10px] font-semibold uppercase text-surface-500">
            Coincidencias de la Biblioteca
          </h3>
          {#if matchesLoading}
            <p class="text-surface-500">Buscando…</p>
          {:else if matches.length === 0}
            <p class="text-surface-500">No hay coincidencias claras. Puedes importar o generar.</p>
          {:else}
            <ul class="space-y-1.5">
              {#each matches.slice(0, 8) as a (a.id)}
                {@const u = thumb(a)}
                <li
                  class="flex items-center gap-2 rounded-lg border border-surface-800 bg-surface-950/60 p-1.5"
                >
                  <div class="h-12 w-16 shrink-0 overflow-hidden rounded bg-black/40">
                    {#if u}
                      <img src={u} alt={a.title} class="h-full w-full object-cover" />
                    {/if}
                  </div>
                  <div class="min-w-0 flex-1">
                    <p class="truncate font-medium text-surface-100">{a.title}</p>
                    <p class="truncate text-[9px] text-surface-500">
                      {(a.concepts ?? a.tags ?? []).slice(0, 2).join(" · ")}
                    </p>
                  </div>
                  <button
                    type="button"
                    class="btn-primary shrink-0 text-[10px]"
                    disabled={busy}
                    onclick={() => onUseAsset(a.id)}
                  >
                    Usar esta imagen
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </section>

        {#if pendingImportId}
          <div class="rounded-lg border border-violet-800/50 bg-violet-950/30 p-2">
            <p class="text-[10px] text-violet-100">
              Imagen importada. Confirma si quieres usarla en esta escena (no se colocó aún).
            </p>
            <button
              type="button"
              class="btn-primary mt-1.5 w-full text-[11px]"
              disabled={busy}
              onclick={() => onUseAsset(pendingImportId!)}
            >
              Usar esta imagen
            </button>
          </div>
        {/if}

        <div class="flex flex-col gap-1.5 border-t border-surface-800 pt-2">
          <button type="button" class="btn-secondary w-full text-[11px]" disabled={busy} onclick={onImport}>
            Importar imagen
          </button>
          <button type="button" class="btn-primary w-full text-[11px]" disabled={busy} onclick={onGenerate}>
            Generar una nueva
          </button>
        </div>

        <button
          type="button"
          class="text-[10px] text-surface-500 underline"
          onclick={() => (moreOpen = !moreOpen)}
        >
          {moreOpen ? "Menos opciones" : "Más opciones"}
        </button>
        {#if moreOpen}
          <button
            type="button"
            class="btn-ghost w-full text-[10px]"
            disabled={busy}
            onclick={onSkip}
          >
            Dejar esta escena sin imagen
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}
