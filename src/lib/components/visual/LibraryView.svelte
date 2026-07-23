<script lang="ts">
  import LibraryControlCenter from "./LibraryControlCenter.svelte";
  import AssetCard from "./library/AssetCard.svelte";
  import AssetInspector from "./library/AssetInspector.svelte";
  import { filterAssets, sortAssets, type QuickFilter } from "./library/libraryFilters";
  import type {
    AssetUsageRow,
    LicenseStatus,
    MediaAsset,
  } from "./library/libraryTypes";

  let {
    assets = [] as MediaAsset[],
    loading = false,
    selectedId = null as string | null,
    usage = [] as AssetUsageRow[],
    usageLoading = false,
    busy = false,
    sceneLabel = null as string | null,
    onSelect,
    onSave,
    onArchive,
    onRestore,
    onBlock,
    onUseInScene = undefined as ((id: string) => void) | undefined,
    onReview = () => {},
  }: {
    assets?: MediaAsset[];
    loading?: boolean;
    selectedId?: string | null;
    usage?: AssetUsageRow[];
    usageLoading?: boolean;
    busy?: boolean;
    sceneLabel?: string | null;
    onSelect: (id: string | null) => void;
    onSave: (
      id: string,
      patch: { title: string; tags: string[]; concepts: string[]; license: LicenseStatus },
    ) => Promise<void>;
    onArchive: (id: string) => Promise<void>;
    onRestore: (id: string) => Promise<void>;
    onBlock: (id: string) => Promise<void>;
    onUseInScene?: ((id: string) => void) | undefined;
    onReview?: () => void;
  } = $props();

  let section = $state<"control" | "assets">("control");
  let quick = $state<QuickFilter>("all");
  let sort = $state<"recent" | "used" | "quality" | "title">("recent");
  let more = $state(false);
  let statusFilter = $state("all");

  const filtered = $derived(
    sortAssets(
      filterAssets(assets, { quick, status: statusFilter }),
      sort,
    ),
  );

  const selected = $derived(filtered.find((a) => a.id === selectedId) ?? assets.find((a) => a.id === selectedId) ?? null);
</script>

<div class="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden text-[11px]">
  <div class="grid shrink-0 grid-cols-2 gap-1 rounded-lg bg-surface-900 p-1" role="tablist" aria-label="Secciones de Biblioteca">
    <button
      type="button"
      role="tab"
      aria-selected={section === "control"}
      class="rounded-md px-2 py-1.5 text-[10px] font-semibold {section === 'control' ? 'bg-violet-600 text-white' : 'text-surface-400 hover:bg-surface-800'}"
      onclick={() => (section = "control")}
    >
      Buscar y completar
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={section === "assets"}
      class="rounded-md px-2 py-1.5 text-[10px] font-semibold {section === 'assets' ? 'bg-violet-600 text-white' : 'text-surface-400 hover:bg-surface-800'}"
      onclick={() => (section = "assets")}
    >
      Imágenes alojadas ({assets.length})
    </button>
  </div>

  {#if section === "control"}
    <div class="min-h-0 flex-1 overflow-y-auto pr-1">
      <LibraryControlCenter {onReview} />
    </div>
  {:else}
  <div class="flex shrink-0 flex-wrap items-center gap-1">
    {#each [
      ["all", "Todas"],
      ["landscape", "Horizontal"],
      ["portrait", "Vertical"],
      ["ai", "IA"],
      ["imported", "Importadas"],
    ] as [id, label]}
      <button
        type="button"
        class="rounded-full px-2 py-0.5 text-[10px]
          {quick === id ? 'bg-violet-600 text-white' : 'bg-surface-800 text-surface-300'}"
        onclick={() => (quick = id as QuickFilter)}
      >
        {label}
      </button>
    {/each}
    <button
      type="button"
      class="text-[10px] text-surface-500 underline"
      onclick={() => (more = !more)}>Más filtros</button
    >
    <span class="ml-auto text-[10px] text-surface-500">{filtered.length} de {assets.length}</span>
  </div>
  {#if more}
    <div class="flex shrink-0 flex-wrap gap-2">
      <label class="flex items-center gap-1 text-[10px] text-surface-400">
        Estado
        <select class="rounded border border-surface-700 bg-surface-950 px-1" bind:value={statusFilter}>
          <option value="all">Todos</option>
          <option value="active">Activos</option>
          <option value="archived">Archivados</option>
          <option value="missing">Ausentes</option>
          <option value="blocked">Bloqueados</option>
        </select>
      </label>
      <label class="flex items-center gap-1 text-[10px] text-surface-400">
        Orden
        <select class="rounded border border-surface-700 bg-surface-950 px-1" bind:value={sort}>
          <option value="recent">Más recientes</option>
          <option value="used">Más usadas</option>
          <option value="quality">Mejor calidad</option>
          <option value="title">Título A–Z</option>
        </select>
      </label>
    </div>
  {/if}

  <div
    class="grid min-h-0 flex-1 gap-2 overflow-hidden"
    style="grid-template-columns: minmax(0,1fr) minmax(280px, 34%)"
  >
    <div class="min-h-0 overflow-y-auto">
      {#if loading}
        <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(170px, 1fr))">
          {#each Array(6) as _}
            <div class="aspect-[4/3] animate-pulse rounded-xl bg-surface-800"></div>
          {/each}
        </div>
      {:else if filtered.length === 0}
        <p class="rounded-lg border border-dashed border-surface-700 p-3 text-surface-400">
          {assets.length === 0
            ? "Tu biblioteca está vacía. Importa imágenes para reutilizarlas."
            : "No hay resultados con estos filtros."}
        </p>
      {:else}
        <div class="grid gap-2" style="grid-template-columns: repeat(auto-fill, minmax(170px, 1fr))">
          {#each filtered as a (a.id)}
            <AssetCard
              asset={a}
              selected={a.id === selectedId}
              sceneLabel={sceneLabel}
              onSelect={() => onSelect(a.id)}
              onUseInScene={onUseInScene && sceneLabel ? () => onUseInScene(a.id) : undefined}
            />
          {/each}
        </div>
      {/if}
    </div>
    <div class="min-h-0 min-w-[280px] overflow-y-auto border-l border-surface-800 pl-2">
      {#if selected}
        <AssetInspector
          asset={selected}
          {usage}
          {usageLoading}
          {busy}
          {sceneLabel}
          onSave={(patch) => onSave(selected.id, patch)}
          onArchive={() => onArchive(selected.id)}
          onRestore={() => onRestore(selected.id)}
          onBlock={() => onBlock(selected.id)}
          onUseInScene={onUseInScene && sceneLabel ? () => onUseInScene(selected.id) : undefined}
        />
      {:else}
        <p class="text-surface-500">Selecciona una imagen</p>
      {/if}
    </div>
  </div>
  {/if}
</div>
