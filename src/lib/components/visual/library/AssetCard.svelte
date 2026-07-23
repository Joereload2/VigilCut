<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import {
    isMockAsset,
    isAiOrigin,
    previewPath,
  } from "./libraryFilters";
  import { licenseLabel, type MediaAsset } from "./libraryTypes";

  let {
    asset,
    selected = false,
    sceneLabel = null as string | null,
    onSelect,
    onUseInScene = undefined as (() => void) | undefined,
  }: {
    asset: MediaAsset;
    selected?: boolean;
    sceneLabel?: string | null;
    onSelect: () => void;
    onUseInScene?: (() => void) | undefined;
  } = $props();

  let imgOk = $state(true);

  const src = $derived.by(() => {
    const p = previewPath(asset);
    if (!p || !imgOk) return null;
    try {
      return convertFileSrc(p);
    } catch {
      return null;
    }
  });

  const chips = $derived((asset.concepts?.length ? asset.concepts : asset.tags).slice(0, 2));
  const warn =
    $derived(
      asset.status === "missing"
        ? "Ausente"
        : asset.status === "blocked"
          ? "Bloqueada"
          : isMockAsset(asset)
            ? "Mock"
            : asset.licenseStatus === "unknown"
              ? "Licencia ?"
              : isAiOrigin(asset)
                ? "IA"
                : null,
    );
</script>

<div
  class="flex w-full min-w-0 flex-col overflow-hidden rounded-xl border text-left transition
    {selected
    ? 'border-violet-500 bg-surface-900 ring-1 ring-violet-500/40'
    : 'border-surface-800 bg-surface-950/60 hover:border-surface-600'}"
>
  <button type="button" class="w-full text-left" aria-pressed={selected} onclick={onSelect}>
    <div class="relative aspect-[4/3] w-full bg-black/40">
      {#if src}
        <img
          src={src}
          alt={asset.title}
          class="h-full w-full object-contain"
          loading="lazy"
          onerror={() => (imgOk = false)}
        />
      {:else}
        <div
          class="flex h-full items-center justify-center p-2 text-center text-[10px] text-surface-500"
        >
          Archivo no disponible
        </div>
      {/if}
      {#if warn}
        <span
          class="absolute left-1 top-1 rounded bg-surface-950/90 px-1 py-0.5 text-[9px] font-medium
            {warn === 'Mock' ? 'text-violet-200' : warn === 'Ausente' ? 'text-red-300' : 'text-amber-200'}"
        >
          {warn === "Mock" ? "Simulación" : warn}
        </span>
      {/if}
    </div>
    <div class="space-y-0.5 p-1.5">
      <p class="line-clamp-2 text-[11px] font-medium text-surface-100">
        {asset.title || "Sin título"}
      </p>
      {#if chips.length}
        <p class="truncate text-[9px] text-surface-500">{chips.join(" · ")}</p>
      {/if}
      <p class="text-[9px] text-surface-500">
        {licenseLabel(asset.licenseStatus)}
        {#if asset.timesUsed > 0}
          · {asset.timesUsed} usos
        {/if}
      </p>
    </div>
  </button>
  {#if onUseInScene && sceneLabel}
    <div class="px-1.5 pb-1.5">
      <button type="button" class="btn-primary w-full py-0.5 text-[9px]" onclick={onUseInScene}>
        Usar en {sceneLabel}
      </button>
    </div>
  {/if}
</div>
