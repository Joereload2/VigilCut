<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";
  import SidePanel from "$lib/components/SidePanel.svelte";
  import ExceptionQueue from "$lib/components/ExceptionQueue.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import BatchPanel from "$lib/components/BatchPanel.svelte";

  type TabId = "resumen" | "supervision" | "timeline" | "lote" | "ajustes";

  interface Props {
    /** When embedded under AuxTabShell, hide internal tab chrome */
    embedded?: boolean;
    forceTab?: TabId | null;
  }
  let { embedded = false, forceTab = null }: Props = $props();

  let tab = $state<TabId>("resumen");

  const pending = $derived(projectStore.pendingExceptionCount);
  const stats = $derived(projectStore.analysisRun?.stats);
  const segs = $derived(projectStore.segments.length);
  const activeTab = $derived((forceTab ?? tab) as TabId);

  const tabs = $derived.by(() => {
    const list: { id: TabId; label: string; badge?: number; alert?: boolean }[] = [
      { id: "resumen", label: "Resumen" },
      {
        id: "supervision",
        label: "Supervisar",
        badge: pending > 0 ? pending : undefined,
        alert: pending > 0,
      },
      { id: "timeline", label: "Timeline", badge: segs > 0 ? segs : undefined },
      { id: "lote", label: "Lote" },
      { id: "ajustes", label: "Ajustes" },
    ];
    return list;
  });
</script>

<div
  class="flex h-full min-h-0 w-full min-w-0 max-w-full flex-col overflow-hidden
    {embedded ? '' : 'rounded-xl border border-surface-800 bg-surface-900/60'}"
  style="box-sizing:border-box"
  aria-label="Herramientas de supervisión"
>
  {#if !embedded}
    <div
      class="flex shrink-0 gap-0.5 overflow-x-auto border-b border-surface-800 bg-surface-950/90 p-1"
      role="tablist"
    >
      {#each tabs as t (t.id)}
        <button
          type="button"
          role="tab"
          aria-selected={activeTab === t.id}
          class="relative shrink-0 rounded-lg px-2.5 py-1.5 text-[11px] font-semibold transition
            {activeTab === t.id
            ? 'bg-surface-800 text-white'
            : 'text-surface-400 hover:bg-surface-800/50 hover:text-surface-200'}"
          onclick={() => (tab = t.id)}
        >
          {t.label}
          {#if t.badge !== undefined && t.badge !== 0}
            <span
              class="ml-1 inline-flex min-w-[1.1rem] items-center justify-center rounded-full px-1 text-[9px] font-bold
                {t.alert ? 'bg-warning/25 text-warning' : 'bg-surface-700 text-surface-300'}"
            >
              {t.badge}
            </span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}

  <div class="min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden">
    {#if activeTab === "resumen"}
      <div class="space-y-3 p-3">
        <div>
          <div class="text-sm font-semibold text-surface-100">Resumen del corte</div>
          <p class="mt-0.5 text-[10px] text-surface-500">
            La fábrica ya aplicó la política. Aquí solo miras y exportas.
          </p>
        </div>

        {#if stats}
          <div
            class="rounded-xl border px-3 py-2 text-[11px]
              {pending > 0
              ? 'border-warning/40 bg-warning/10 text-warning'
              : 'border-keep/30 bg-keep/10 text-keep'}"
          >
            {#if pending > 0}
              <strong>Requiere revisión</strong> · {pending} duda(s). El resto ya se cortó solo.
            {:else}
              <strong>Listo para oír y exportar</strong> · sin dudas pendientes.
            {/if}
          </div>
          <div class="grid grid-cols-2 gap-2 text-[11px]">
            <div class="rounded-xl border border-surface-800 bg-surface-950/80 p-2.5">
              <div class="text-surface-500">Cortes automáticos</div>
              <div class="font-mono text-lg font-bold text-cut">{stats.autoCutCount}</div>
            </div>
            <div class="rounded-xl border border-surface-800 bg-surface-950/80 p-2.5">
              <div class="text-surface-500">Dudas por revisar</div>
              <div
                class="font-mono text-lg font-bold {pending > 0 ? 'text-warning' : 'text-keep'}"
              >
                {pending}
              </div>
            </div>
            <div class="rounded-xl border border-surface-800 bg-surface-950/80 p-2.5">
              <div class="text-surface-500">Duración final</div>
              <div class="font-mono text-lg font-bold text-keep">
                {formatTime(stats.outputDuration)}
              </div>
            </div>
            <div class="rounded-xl border border-surface-800 bg-surface-950/80 p-2.5">
              <div class="text-surface-500">Recortado</div>
              <div class="font-mono text-lg font-bold text-cut">
                −{formatTime(stats.autoRemovedDuration)}
              </div>
            </div>
          </div>
          <p class="text-[10px] text-surface-500">
            Score = estimación operativa (no probabilidad científica). · motor:
            <span class="font-mono text-surface-400">{projectStore.analysisRun?.method ?? "—"}</span>
          </p>
        {:else}
          <p class="text-xs text-surface-500">Abre o re-detecta un video para ver stats.</p>
        {/if}

        <div class="space-y-2 rounded-xl border border-surface-800 bg-surface-950/50 p-3">
          <div class="text-[11px] font-semibold text-surface-300">Siguiente paso</div>
          {#if pending > 0}
            <p class="text-[11px] text-warning">
              Hay {pending} duda(s). Revisa en <strong>Supervisar</strong>, o exporta en modo
              seguro (se conservan).
            </p>
            <button
              type="button"
              class="btn-secondary w-full text-xs"
              onclick={() => (tab = "supervision")}
            >
              Ir a Supervisar →
            </button>
          {:else}
            <ol class="list-decimal space-y-1 pl-4 text-[11px] text-surface-400">
              <li>▶ Oír resultado (o Play)</li>
              <li>Activa Audio enhance si quieres</li>
              <li>Exportar video</li>
            </ol>
          {/if}
        </div>

        <div class="flex flex-wrap gap-1.5">
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (tab = "timeline")}
            >Ver timeline ({segs})</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (tab = "lote")}
            >Lote / inbox</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (tab = "ajustes")}
            >Policy y ajustes</button
          >
        </div>
      </div>
    {:else if activeTab === "supervision"}
      <div class="flex min-h-full flex-col p-2">
        <ExceptionQueue />
      </div>
    {:else if activeTab === "timeline"}
      <div class="space-y-2 p-2">
        <div class="px-1">
          <div class="text-sm font-semibold text-surface-100">Timeline diagnóstico</div>
          <p class="text-[10px] text-surface-500">
            {segs} tramos · clic para seleccionar · K/X para decidir
          </p>
        </div>
        <Timeline />
      </div>
    {:else if activeTab === "lote"}
      <div class="p-2">
        <BatchPanel />
      </div>
    {:else}
      <div class="p-2">
        <!-- SidePanel already has factory stats + policy; show policy section expanded -->
        <SidePanel forceSettingsOpen />
      </div>
    {/if}
  </div>
</div>
