<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { isMockAsset, normalizeTags, previewPath } from "./libraryFilters";
  import {
    formatBytes,
    licenseLabel,
    type AssetUsageRow,
    type LicenseStatus,
    type MediaAsset,
  } from "./libraryTypes";

  let {
    asset,
    usage = [] as AssetUsageRow[],
    usageLoading = false,
    busy = false,
    sceneLabel = null as string | null,
    onSave,
    onArchive,
    onRestore,
    onBlock,
    onUseInScene = undefined as (() => void) | undefined,
  }: {
    asset: MediaAsset;
    usage?: AssetUsageRow[];
    usageLoading?: boolean;
    busy?: boolean;
    sceneLabel?: string | null;
    onSave: (patch: {
      title: string;
      tags: string[];
      concepts: string[];
      license: LicenseStatus;
    }) => Promise<void>;
    onArchive: () => Promise<void>;
    onRestore: () => Promise<void>;
    onBlock: () => Promise<void>;
    onUseInScene?: (() => void) | undefined;
  } = $props();

  let title = $state("");
  let tagsRaw = $state("");
  let conceptsRaw = $state("");
  let license = $state<LicenseStatus>("unknown");
  let detailsOpen = $state(false);
  let dirty = $state(false);

  $effect(() => {
    title = asset.title ?? "";
    tagsRaw = (asset.tags ?? []).join(", ");
    conceptsRaw = (asset.concepts ?? []).join(", ");
    license = (asset.licenseStatus as LicenseStatus) || "unknown";
    dirty = false;
  });

  function markDirty() {
    dirty = true;
  }

  const src = $derived.by(() => {
    const p = previewPath(asset);
    if (!p) return null;
    try {
      return convertFileSrc(p);
    } catch {
      return null;
    }
  });
</script>

<div class="flex min-h-0 flex-col gap-2 overflow-y-auto text-[11px]">
  <div class="aspect-video w-full overflow-hidden rounded-lg border border-surface-800 bg-black/40">
    {#if src}
      <img src={src} alt={asset.title} class="h-full w-full object-contain" />
    {:else}
      <p class="flex h-full items-center justify-center text-surface-500">Sin vista previa</p>
    {/if}
  </div>

  {#if isMockAsset(asset)}
    <p class="rounded bg-violet-950/50 px-2 py-1 text-[10px] font-medium text-violet-200">
      Simulación (mock) — no es IA
    </p>
  {/if}

  {#if onUseInScene && sceneLabel}
    <button type="button" class="btn-primary text-[10px]" disabled={busy} onclick={onUseInScene}>
      Usar en {sceneLabel}
    </button>
  {/if}

  <label class="block space-y-0.5">
    <span class="text-surface-500">Título</span>
    <input
      class="w-full rounded border border-surface-700 bg-surface-950 px-2 py-1"
      bind:value={title}
      oninput={markDirty}
    />
  </label>
  <label class="block space-y-0.5">
    <span class="text-surface-500">Etiquetas (coma)</span>
    <input
      class="w-full rounded border border-surface-700 bg-surface-950 px-2 py-1"
      bind:value={tagsRaw}
      oninput={markDirty}
    />
  </label>
  <label class="block space-y-0.5">
    <span class="text-surface-500">Conceptos (coma)</span>
    <input
      class="w-full rounded border border-surface-700 bg-surface-950 px-2 py-1"
      bind:value={conceptsRaw}
      oninput={markDirty}
    />
  </label>
  <label class="block space-y-0.5">
    <span class="text-surface-500">Licencia</span>
    <select
      class="w-full rounded border border-surface-700 bg-surface-950 px-2 py-1"
      bind:value={license}
      onchange={markDirty}
    >
      <option value="unknown">Desconocida</option>
      <option value="owned">Propia</option>
      <option value="licensed">Licenciada</option>
      <option value="public_domain">Dominio público</option>
      <option value="attribution_required">Requiere atribución</option>
    </select>
  </label>

  {#if license === "unknown"}
    <p class="text-[10px] text-amber-200/90">
      Uso comercial no verificado. Revisa la licencia antes de publicar.
    </p>
  {/if}

  <p class="text-surface-500">
    Usada {asset.timesUsed ?? 0} vez(es)
    {#if asset.lastUsedAt}
      · última {asset.lastUsedAt.slice(0, 10)}
    {/if}
  </p>

  {#if usageLoading}
    <p class="text-surface-500">Cargando usos…</p>
  {:else if usage.length === 0}
    <p class="text-surface-500">Aún no se ha usado en ningún video.</p>
  {:else}
    <ul class="space-y-0.5 text-[10px] text-surface-400">
      {#each usage.slice(0, 8) as u (u.id)}
        <li class="truncate" title={u.mediaPath}>{u.usedAt.slice(0, 16)} · {u.mediaPath.split(/[/\\]/).pop()}</li>
      {/each}
    </ul>
  {/if}

  <div class="flex flex-wrap gap-1">
    <button
      type="button"
      class="btn-primary text-[10px]"
      disabled={busy || !dirty}
      onclick={() =>
        onSave({
          title: title.trim() || asset.title,
          tags: normalizeTags(tagsRaw),
          concepts: normalizeTags(conceptsRaw),
          license,
        })}
    >
      Guardar cambios
    </button>
    {#if asset.status === "archived"}
      <button type="button" class="btn-secondary text-[10px]" disabled={busy} onclick={onRestore}
        >Restaurar</button
      >
    {:else}
      <button type="button" class="btn-ghost text-[10px]" disabled={busy} onclick={onArchive}
        >Archivar</button
      >
    {/if}
    <button type="button" class="btn-ghost text-[10px] text-red-300" disabled={busy} onclick={onBlock}
      >Bloquear</button
    >
  </div>

  <button
    type="button"
    class="text-left text-[10px] text-brand-300 underline"
    onclick={() => (detailsOpen = !detailsOpen)}
  >
    {detailsOpen ? "Ocultar detalles técnicos" : "Detalles técnicos"}
  </button>
  {#if detailsOpen}
    <div class="space-y-0.5 break-all rounded bg-surface-900/80 p-2 font-mono text-[9px] text-surface-400">
      <p>id: {asset.id}</p>
      <p>origen: {asset.source ?? "—"} / {asset.provenance?.source ?? "—"}</p>
      <p>provider: {asset.provenance?.provider ?? "—"} · {asset.provenance?.model ?? "—"}</p>
      <p>QA: {asset.qaStatus ?? "—"}</p>
      <p>{asset.width}×{asset.height} · {asset.mimeType} · {formatBytes(asset.fileSize)}</p>
      <p>licencia UI: {licenseLabel(asset.licenseStatus)}</p>
      {#if asset.provenance?.prompt}
        <p class="whitespace-pre-wrap">prompt: {asset.provenance.prompt}</p>
      {/if}
      <p>path: {asset.managedPath}</p>
    </div>
  {/if}
</div>
