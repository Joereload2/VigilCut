<script lang="ts">
  /**
   * Ajuste: marcos verdes = secciones del Short 9:16 (1–3, moldeables).
   */
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import MultiCropSource from "$lib/vnext/components/MultiCropSource.svelte";
  import SplitShortPreview from "$lib/vnext/components/SplitShortPreview.svelte";
  import MiniTimeline from "$lib/vnext/components/MiniTimeline.svelte";
  import FrameGrid from "$lib/vnext/components/FrameGrid.svelte";
  import StationDock from "$lib/vnext/components/StationDock.svelte";
  import { buildStationGuide } from "$lib/vnext/presentation/stations";
  import { type CropRegion, defaultSection, clampSection } from "$lib/vnext/types/crop";

  const sel = $derived(vnextSession.selectedCandidate);
  const media = $derived(vnextSession.project?.sourceMediaPath ?? null);
  const suggested = $derived(
    vnextSession.candidates.filter((c) => c.status !== "rejected"),
  );

  let start = $state(0);
  let end = $state(10);
  let subtitleText = $state("");
  let regions = $state<CropRegion[]>([defaultSection(0)]);
  let activeCropId = $state<string | null>(null);
  let seededFor = $state<string | null>(null);

  const primary = $derived(regions[0] ?? defaultSection(0));

  const guide = $derived(
    buildStationGuide({
      stage: "adjusting",
      isHub: false,
      hasSource: true,
      candidateCount: suggested.length,
      hasSelection: !!sel,
      jobs: vnextSession.flowJobs,
    }),
  );

  $effect(() => {
    const c = sel;
    if (!c) return;
    start = c.start;
    end = c.end;
    subtitleText = c.transcript?.slice(0, 180) ?? "";
    // Only reseed frames when switching suggested short (keep multi-section edits)
    if (seededFor !== c.id) {
      seededFor = c.id;
      const one = clampSection({
        id: `sec-0-${c.id}`,
        centerX: c.framing?.centerX ?? 0.5,
        centerY: c.framing?.centerY ?? 0.5,
        widthFrac: 0.3,
        heightFrac: 0.95,
      });
      regions = [one];
      activeCropId = one.id;
    }
  });

  async function persistSpan() {
    await vnextSession.saveSpan(start, end);
  }

  async function persistPrimaryFraming() {
    if (!sel) return;
    const p = primary;
    await vnextSession.saveFraming({
      ...sel.framing,
      mode: "manual",
      centerX: p.centerX,
      centerY: p.centerY,
      zoom: Math.max(1, 0.4 / Math.max(0.12, p.widthFrac)),
    });
  }

  function onTimeline(s: number, e: number) {
    start = s;
    end = e;
  }

  function selectSuggested(id: string) {
    vnextSession.selectedCandidateId = id;
  }

  async function createShort() {
    await persistSpan();
    await persistPrimaryFraming();
    const dur = Math.max(0.5, end - start);
    await vnextSession.startCreateShort(
      [{ startS: 0, endS: dur, text: subtitleText || " " }],
      true,
    );
  }
</script>

<div class="flex h-full min-h-0 flex-col" data-testid="page-edit">
  <div
    class="grid min-h-0 flex-1 grid-cols-1 gap-2 overflow-hidden p-2 lg:grid-cols-[minmax(0,1.4fr)_minmax(9rem,0.55fr)_minmax(11rem,0.85fr)] lg:p-2.5"
    data-testid="adjust-canvas"
  >
    <section class="flex min-h-0 min-w-0 flex-col gap-1.5 rounded-2xl border border-surface-800 bg-surface-950/40 p-2">
      {#if sel}
        <div class="min-h-0 flex-1">
          <MultiCropSource
            sourcePath={media || sel.sourceMediaPath}
            {start}
            {end}
            {regions}
            activeId={activeCropId}
            onRegionsChange={(r) => (regions = r)}
            onActiveChange={(id) => (activeCropId = id)}
          />
        </div>
        <div class="shrink-0 rounded-xl border border-surface-800 bg-surface-900/70 px-2 py-1.5">
          <p class="mb-0.5 text-[10px] font-semibold uppercase text-surface-500">Cortar (inicio / final)</p>
          <MiniTimeline {start} {end} editable={true} onChange={onTimeline} />
        </div>
        <label class="shrink-0 text-[10px] text-surface-400">
          Subtítulos
          <textarea
            class="mt-0.5 min-h-[2.4rem] w-full rounded-lg border border-surface-700 bg-surface-950 p-1.5 text-xs text-surface-100"
            bind:value={subtitleText}
          ></textarea>
        </label>
      {:else}
        <p class="m-auto text-sm text-surface-500">Elegí un Short de la bandeja derecha</p>
      {/if}
    </section>

    <section class="flex min-h-0 min-w-0 flex-col items-center justify-start gap-1 overflow-y-auto rounded-2xl border border-surface-800 bg-black/40 p-1.5">
      {#if sel}
        <SplitShortPreview
          sourcePath={media || sel.sourceMediaPath}
          {start}
          {end}
          {regions}
          autoplay={true}
        />
      {/if}
    </section>

    <aside
      class="flex min-h-0 min-w-0 flex-col overflow-hidden rounded-2xl border border-sky-900/40 bg-sky-950/15"
      data-testid="suggested-shorts"
    >
      <div class="border-b border-surface-700/50 px-2.5 py-1.5">
        <h2 class="text-xs font-semibold text-white">Shorts sugeridos</h2>
        <p class="text-[10px] text-surface-500">Elegí uno para modificar</p>
      </div>
      <div class="min-h-0 flex-1 overflow-hidden">
        <FrameGrid
          items={suggested}
          selectedId={sel?.id ?? null}
          mediaPath={media}
          onSelect={selectSuggested}
        />
      </div>
      <div class="shrink-0 border-t border-surface-800 p-2">
        <p class="text-[9px] leading-snug text-surface-500">
          {regions.length} sección{regions.length === 1 ? "" : "es"} en el 9:16 (máx. 3)
        </p>
      </div>
    </aside>
  </div>

  <StationDock
    {guide}
    label="Crear Short"
    pending={vnextSession.pendingAction === "render" || !!vnextSession.activeRenderJob}
    pendingLabel="Creando Short…"
    disabled={!sel || !!vnextSession.pendingAction || !!vnextSession.activeRenderJob}
    hint="Siguiente: generar el vertical 9:16"
    onPrimary={() => void createShort()}
  />
</div>
