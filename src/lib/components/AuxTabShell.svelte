<script lang="ts">
  /**
   * Bottom horizontal tool panel with closable tabs.
   * Tab strip is always fully visible (never clipped).
   * Panel body scrolls internally if content is tall.
   */
  export type AuxTabId = string;

  interface TabDef {
    id: AuxTabId;
    label: string;
    badge?: number;
    alert?: boolean;
  }

  interface Props {
    tabs: TabDef[];
    openIds: AuxTabId[];
    activeId: AuxTabId | null;
    onOpen: (id: AuxTabId) => void;
    onClose: (id: AuxTabId) => void;
    onActivate: (id: AuxTabId) => void;
    catalog: TabDef[];
    /** When false, only the tab strip is shown (still fully visible). */
    expanded?: boolean;
    children?: import("svelte").Snippet;
  }

  let {
    tabs,
    openIds,
    activeId,
    onOpen,
    onClose,
    onActivate,
    catalog,
    expanded = true,
    children,
  }: Props = $props();

  let menuOpen = $state(false);

  const closedCatalog = $derived(catalog.filter((t) => !openIds.includes(t.id)));
  const showBody = $derived(expanded && !!activeId && openIds.includes(activeId));
</script>

<section
  class="flex h-full min-h-0 min-w-0 w-full max-w-full flex-col rounded-xl border border-surface-800 bg-surface-900/80"
  style="box-sizing:border-box; height:100%"
>
  <!-- Tab strip: shrink-0, never overflow-hidden on labels -->
  <div
    class="flex min-h-10 w-full min-w-0 shrink-0 items-center gap-1 border-b border-surface-800 bg-surface-950/95 px-1.5 py-1"
    role="tablist"
  >
    <div class="flex min-w-0 flex-1 flex-wrap items-center gap-1">
      {#each tabs as t (t.id)}
        <div
          class="inline-flex max-w-full items-center rounded-lg
            {activeId === t.id ? 'bg-surface-800' : 'hover:bg-surface-800/50'}"
        >
          <button
            type="button"
            role="tab"
            aria-selected={activeId === t.id}
            class="max-w-[10rem] truncate px-2.5 py-1.5 text-[11px] font-semibold leading-none
              {activeId === t.id ? 'text-white' : 'text-surface-400'}"
            title={t.label}
            onclick={() => onActivate(t.id)}
          >
            {t.label}
            {#if t.badge}
              <span
                class="ml-1 inline-block rounded-full px-1 text-[9px] font-bold
                  {t.alert ? 'bg-warning/25 text-warning' : 'bg-surface-700 text-surface-300'}"
                >{t.badge}</span
              >
            {/if}
          </button>
          <button
            type="button"
            class="shrink-0 px-1.5 py-1.5 text-surface-500 hover:text-surface-200"
            title="Cerrar panel (no borra datos)"
            aria-label="Cerrar {t.label}"
            onclick={() => onClose(t.id)}
          >
            ×
          </button>
        </div>
      {/each}
    </div>

    <div class="relative shrink-0">
      <button
        type="button"
        class="rounded-lg px-2 py-1.5 text-[11px] font-semibold leading-none text-sky-300 hover:bg-surface-800"
        onclick={() => (menuOpen = !menuOpen)}
      >
        + Abrir panel
      </button>
      {#if menuOpen}
        <div
          class="absolute bottom-full right-0 z-40 mb-1 max-h-48 min-w-[11rem] overflow-y-auto rounded-lg border border-surface-700 bg-surface-900 py-1 shadow-xl"
        >
          {#each closedCatalog as t (t.id)}
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-[11px] text-surface-200 hover:bg-surface-800"
              onclick={() => {
                onOpen(t.id);
                menuOpen = false;
              }}>{t.label}</button
            >
          {:else}
            <div class="px-3 py-1.5 text-[10px] text-surface-500">Todos los paneles abiertos</div>
          {/each}
          {#each catalog as t (t.id)}
            {#if openIds.includes(t.id)}
              <button
                type="button"
                class="block w-full px-3 py-1.5 text-left text-[11px] text-surface-500 hover:bg-surface-800"
                onclick={() => {
                  onActivate(t.id);
                  menuOpen = false;
                }}
                >Ir a {t.label}</button
              >
            {/if}
          {/each}
        </div>
      {/if}
    </div>
  </div>

  {#if showBody}
    <!-- In side column: fill height; scroll only inside the panel body -->
    <div
      class="min-h-0 min-w-0 w-full max-w-full flex-1 overflow-x-hidden overflow-y-auto p-1.5 sm:p-2"
      style="box-sizing: border-box"
    >
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</section>
