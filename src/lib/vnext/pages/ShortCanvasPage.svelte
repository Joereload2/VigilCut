<script lang="ts">
  /**
   * PARTE 2 — Qué se ve en el vertical (solo marcos verdes).
   * Solo los momentos SELECCIONADOS en Parte 1 llegan aquí.
   * Luego el short queda listo para exportar.
   */
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import MultiCropSource from "$lib/vnext/components/MultiCropSource.svelte";
  import SplitShortPreview from "$lib/vnext/components/SplitShortPreview.svelte";
  import { formatDuration } from "$lib/vnext/presentation/jobLabels";
  import { type CropRegion, defaultSection, clampSection } from "$lib/vnext/types/crop";
  import { isPendingExportStatus } from "$lib/vnext/presentation/deriveStage";

  /** Solo shorts elegidos (aprobados / en edición), no el resto de propuestos */
  const selectedList = $derived(
    [...vnextSession.candidates]
      .filter(
        (c) =>
          isPendingExportStatus(c.status) ||
          c.status === "exported" ||
          c.id === vnextSession.selectedCandidateId,
      )
      .filter((c) => c.status !== "rejected" && c.status !== "discarded")
      .sort((a, b) => a.start - b.start),
  );

  const sel = $derived(
    selectedList.find((c) => c.id === vnextSession.selectedCandidateId) ??
      selectedList.find((c) => isPendingExportStatus(c.status)) ??
      selectedList[0] ??
      null,
  );
  const media = $derived(vnextSession.project?.sourceMediaPath ?? null);

  const start = $derived(sel?.start ?? 0);
  const end = $derived(sel?.end ?? 10);
  const duration = $derived(Math.max(0, end - start));

  let regions = $state<CropRegion[]>([defaultSection(0)]);
  let activeCropId = $state<string | null>(null);
  let seededFor = $state<string | null>(null);

  const primary = $derived(regions[0] ?? defaultSection(0));
  const pendingCount = $derived(
    selectedList.filter((c) => isPendingExportStatus(c.status)).length,
  );

  $effect(() => {
    if (selectedList.length === 0) return;
    if (!selectedList.some((c) => c.id === vnextSession.selectedCandidateId)) {
      const next =
        selectedList.find((c) => isPendingExportStatus(c.status)) ?? selectedList[0];
      if (next) vnextSession.selectedCandidateId = next.id;
    }
  });

  $effect(() => {
    const c = sel;
    if (!c) return;
    if (seededFor !== c.id) {
      seededFor = c.id;
      const panels = c.framing?.panels;
      if (panels && panels.length > 0) {
        regions = panels.map((p, i) =>
          clampSection({
            id: `sec-${i}-${c.id}`,
            centerX: p.centerX,
            centerY: p.centerY,
            widthFrac: p.widthFrac,
            heightFrac: p.heightFrac,
          }),
        );
      } else {
        regions = [
          clampSection({
            id: `sec-0-${c.id}`,
            centerX: c.framing?.centerX ?? 0.5,
            centerY: c.framing?.centerY ?? 0.42,
            widthFrac: c.framing?.widthFrac ?? 0.32,
            heightFrac: c.framing?.heightFrac ?? 0.9,
          }),
        ];
      }
      activeCropId = regions[0]?.id ?? null;
    }
  });

  function switchShort(id: string) {
    vnextSession.selectedCandidateId = id;
  }

  async function createShort() {
    if (!sel || vnextSession.pendingAction || vnextSession.activeRenderJob) return;
    const panels = regions.map((r) => ({
      centerX: r.centerX,
      centerY: r.centerY,
      widthFrac: r.widthFrac,
      heightFrac: r.heightFrac,
    }));
    const framing = {
      ...sel.framing,
      mode: "manual" as const,
      centerX: primary.centerX,
      centerY: primary.centerY,
      widthFrac: primary.widthFrac,
      heightFrac: primary.heightFrac,
      zoom: Math.max(1, 0.4 / Math.max(0.12, primary.widthFrac)),
      panels,
    };
    // Sin burn-in de subtítulos por defecto (evita letras basura / [habla …] en el MP4).
    // El recorte visual es lo que importa; el audio del tramo ya va en el archivo.
    await vnextSession.exportShortWithFraming(framing, [], false);
  }
</script>

<div class="flex h-full max-h-full min-h-0 flex-col overflow-hidden bg-surface-950" data-testid="page-short-canvas">
  <div class="flex shrink-0 items-center justify-between gap-2 border-b border-surface-800 px-3 py-1">
    <p class="min-w-0 truncate text-[10px] font-bold uppercase tracking-wide text-vigil-400">
      Parte 2 · 9:16
      <span class="ml-1 font-normal normal-case tracking-normal text-surface-400">
        marcos verdes
        {#if sel}
          <span class="font-mono">
            · {formatDuration(start)}–{formatDuration(end)} ({formatDuration(duration)})
          </span>
        {/if}
      </span>
    </p>
    <button
      type="button"
      class="shrink-0 rounded border border-surface-700 px-2 py-0.5 text-[10px] text-surface-300 hover:border-vigil-500 hover:text-white"
      data-testid="cta-back-to-moments"
      onclick={() => vnextSession.backToMomentReview()}
    >← Momentos</button>
  </div>

  {#if !sel || !media}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center">
      <p class="text-sm text-surface-400">No hay shorts seleccionados. Volvé a la Parte 1.</p>
      <button
        type="button"
        class="text-xs text-vigil-400 underline"
        onclick={() => vnextSession.goProjects()}
      >← Inicio</button>
    </div>
  {:else}
    {#if selectedList.length > 1}
      <div
        class="flex shrink-0 gap-1.5 overflow-x-auto border-b border-surface-800 px-3 py-2"
        data-testid="selected-shorts-bar"
      >
        {#each selectedList as c, i (c.id)}
          <button
            type="button"
            class="shrink-0 rounded-lg border px-2.5 py-1 text-[11px] font-semibold transition
              {sel.id === c.id
              ? 'border-vigil-500 bg-vigil-950/50 text-white'
              : 'border-surface-700 bg-surface-900 text-surface-400 hover:border-surface-500'}"
            onclick={() => switchShort(c.id)}
          >
            #{i + 1}
            {c.title?.slice(0, 18) || "Short"}
            {#if c.status === "exported"}
              <span class="text-emerald-400">✓</span>
            {/if}
          </button>
        {/each}
        <span class="ml-auto self-center text-[10px] text-surface-500">
          {pendingCount} pendiente{pendingCount === 1 ? "" : "s"}
        </span>
      </div>
    {/if}

    <div
      class="grid min-h-0 flex-1 grid-cols-1 gap-1.5 p-1.5 lg:grid-cols-[minmax(0,1.45fr)_minmax(10rem,0.7fr)]"
    >
      <section class="flex min-h-0 min-w-0 flex-col gap-1 overflow-hidden rounded-lg border border-surface-800 bg-black/40 p-1.5">
        <p class="shrink-0 text-[9px] font-bold uppercase tracking-wide text-vigil-400">
          Original · marcos = contenido vertical
        </p>
        <div class="min-h-0 flex-1 overflow-hidden">
          <MultiCropSource
            sourcePath={media}
            {start}
            {end}
            {regions}
            activeId={activeCropId}
            onRegionsChange={(r) => (regions = r)}
            onActiveChange={(id) => (activeCropId = id)}
          />
        </div>
      </section>

      <section class="flex min-h-0 min-w-0 flex-col items-center gap-1.5 overflow-hidden rounded-lg border border-surface-800 bg-black/50 p-1.5">
        <p class="shrink-0 text-[9px] font-bold uppercase tracking-wide text-vigil-400">
          Preview Short 9:16
        </p>
        <div class="flex min-h-0 w-full flex-1 flex-col items-center justify-center overflow-hidden">
          <SplitShortPreview
            sourcePath={media}
            {start}
            {end}
            {regions}
            autoplay={true}
          />
        </div>
        {#if vnextSession.statusMessage}
          <p class="w-full shrink-0 truncate text-center text-[10px] text-vigil-300">
            {vnextSession.statusMessage}
          </p>
        {/if}
        <button
          type="button"
          class="w-full shrink-0 rounded-lg bg-vigil-600 py-2.5 text-xs font-bold text-white hover:bg-vigil-500 disabled:opacity-40"
          data-testid="cta-create-short"
          disabled={!!vnextSession.pendingAction || !!vnextSession.activeRenderJob || sel.status === "exported"}
          onclick={() => void createShort()}
        >
          {#if vnextSession.pendingAction === "render" || vnextSession.activeRenderJob}
            Creando Short…
          {:else if sel.status === "exported"}
            Ya exportado
          {:else}
            Generar Short 9:16
          {/if}
        </button>
      </section>
    </div>
  {/if}
</div>
