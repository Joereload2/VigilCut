<script lang="ts">
  /**
   * Revisión obligatoria en 3 capas:
   * 1) Momento interesante (scoring / razones)
   * 2) Cara / sujeto (marco 9:16)
   * 3) Imágenes de biblioteca (B-roll sugerido por el texto)
   */
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import SourcePreview from "$lib/vnext/components/SourcePreview.svelte";
  import ShortPreview from "$lib/vnext/components/ShortPreview.svelte";
  import MiniTimeline from "$lib/vnext/components/MiniTimeline.svelte";
  import StationDock from "$lib/vnext/components/StationDock.svelte";
  import { formatDuration, scoreToHuman } from "$lib/vnext/presentation/jobLabels";
  import { buildStationGuide } from "$lib/vnext/presentation/stations";
  import {
    buildInterestCard,
    canApproveShort,
    type ReviewGate,
  } from "$lib/vnext/presentation/interest";
  import * as api from "$lib/utils/tauri";
  import { pathToPlayableUrl } from "$lib/vnext/media/fileUrl";

  type LibAsset = {
    id: string;
    title?: string | null;
    fileName?: string | null;
    path?: string | null;
    previewPath?: string | null;
    tags?: string[];
  };

  const list = $derived(
    [...vnextSession.candidates]
      .filter((c) => c.status !== "rejected")
      .sort((a, b) => b.score - a.score),
  );
  const sel = $derived(
    list.find((c) => c.id === vnextSession.selectedCandidateId) ?? list[0] ?? null,
  );
  const media = $derived(vnextSession.project?.sourceMediaPath ?? null);
  const allDiscarded = $derived(
    vnextSession.candidates.length > 0 && list.length === 0,
  );
  const interest = $derived(sel ? buildInterestCard(sel) : null);

  let start = $state(0);
  let end = $state(10);
  let centerX = $state(0.5);
  let centerY = $state(0.45);

  /** Gates: operator must pass all three. */
  let faceConfirmed = $state(false);
  let imagesMode = $state<"pending" | "picked" | "skip">("pending");
  let imageAssets = $state<LibAsset[]>([]);
  let selectedImageIds = $state<string[]>([]);
  let imagesLoading = $state(false);
  let imagesError = $state<string | null>(null);
  let lastQuery = $state("");

  const gate = $derived<ReviewGate>({
    momentOk: !!sel,
    faceOk: faceConfirmed,
    imagesOk: imagesMode === "picked" || imagesMode === "skip",
  });

  const guide = $derived(
    buildStationGuide({
      stage: "candidates_ready",
      isHub: false,
      hasSource: true,
      candidateCount: list.length,
      hasSelection: !!sel,
      jobs: vnextSession.flowJobs,
      emptyCandidates: allDiscarded || list.length === 0,
    }),
  );

  $effect(() => {
    const c = sel;
    if (!c) return;
    start = c.start;
    end = c.end;
    centerX = c.framing?.centerX ?? 0.5;
    centerY = c.framing?.centerY ?? 0.45;
    // Changing moment resets face + image gates
    faceConfirmed = false;
    imagesMode = "pending";
    selectedImageIds = [];
    void loadImagesForMoment();
  });

  $effect(() => {
    if (list.length === 0) return;
    if (!list.some((c) => c.id === vnextSession.selectedCandidateId)) {
      vnextSession.selectedCandidateId = list[0].id;
    }
  });

  function select(id: string) {
    vnextSession.selectedCandidateId = id;
  }

  async function loadImagesForMoment() {
    if (!sel || !interest) return;
    imagesLoading = true;
    imagesError = null;
    const q = interest.imageQueryTerms.slice(0, 4).join(" ");
    lastQuery = q || sel.title || "concept";
    try {
      const raw = (await api.visualListAssets(lastQuery || null, 16)) as LibAsset[];
      imageAssets = Array.isArray(raw) ? raw : [];
      // Fallback: list without query if empty
      if (imageAssets.length === 0) {
        const all = (await api.visualListAssets(null, 16)) as LibAsset[];
        imageAssets = Array.isArray(all) ? all : [];
      }
    } catch (e) {
      imagesError = String(e);
      imageAssets = [];
    } finally {
      imagesLoading = false;
    }
  }

  function toggleImage(id: string) {
    if (selectedImageIds.includes(id)) {
      selectedImageIds = selectedImageIds.filter((x) => x !== id);
    } else {
      selectedImageIds = [...selectedImageIds, id].slice(0, 4);
    }
    imagesMode = selectedImageIds.length > 0 ? "picked" : "pending";
  }

  function skipImages() {
    selectedImageIds = [];
    imagesMode = "skip";
  }

  function confirmFace() {
    faceConfirmed = true;
  }

  function facePreset(x: number, y: number) {
    centerX = x;
    centerY = y;
    faceConfirmed = false;
  }

  async function onCenter(x: number, y: number) {
    centerX = x;
    centerY = y;
    faceConfirmed = false;
  }

  async function useClip() {
    if (!sel || !canApproveShort(gate)) return;
    await vnextSession.saveSpan(start, end);
    await vnextSession.saveFraming({
      ...sel.framing,
      mode: "manual",
      centerX,
      centerY,
    });
    // Note: selected images are operator intent for B-roll; full bake is later phase.
    // Persist path: keep selection in status message so it's not silent.
    if (selectedImageIds.length > 0) {
      vnextSession.statusMessage = `B-roll elegido: ${selectedImageIds.length} imagen(es)`;
    }
    await vnextSession.decideUseCandidate(sel.id);
  }
</script>

<div class="flex h-full min-h-0 flex-col" data-testid="page-candidates">
  {#if allDiscarded}
    <div class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
      <h2 class="text-lg font-semibold text-white">Descartaste todos los momentos</h2>
      <p class="max-w-md text-sm text-surface-400">
        Sin un momento interesante no hay Short. Reanalizá o usá otro video.
      </p>
    </div>
    <StationDock
      label="Analizar de nuevo"
      pending={!!vnextSession.pendingAction}
      secondaryLabel="Proyectos"
      hint="Siguiente: reanalizá el video o volvé a proyectos"
      onPrimary={() => void vnextSession.reanalyzeCurrent()}
      onSecondary={() => vnextSession.goProjects()}
    />
  {:else}
    <!-- Three mandatory layers -->
    <div
      class="grid min-h-0 flex-1 grid-cols-1 gap-2 overflow-hidden p-2 lg:grid-cols-[minmax(12rem,0.85fr)_minmax(0,1.2fr)_minmax(11rem,0.9fr)]"
      data-testid="review-funnel"
    >
      <!-- 1. MOMENTO -->
      <section
        class="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-surface-800 bg-surface-950/50"
        data-testid="layer-moment"
      >
        <header class="border-b border-surface-800 px-3 py-2">
          <p class="text-[10px] font-bold uppercase tracking-wide text-vigil-400">1 · Momento</p>
          <p class="text-xs text-surface-300">Qué dice el video que vale un Short</p>
        </header>
        <ul class="min-h-0 flex-1 space-y-1.5 overflow-y-auto p-2" data-testid="moment-list">
          {#each list as c, i (c.id)}
            {@const card = buildInterestCard(c)}
            <li>
              <button
                type="button"
                class="w-full rounded-xl border px-2.5 py-2 text-left transition
                  {sel?.id === c.id
                  ? 'border-vigil-500 bg-vigil-950/40'
                  : 'border-surface-800 bg-surface-900/40 hover:border-surface-600'}"
                onclick={() => select(c.id)}
              >
                <div class="flex items-start justify-between gap-1">
                  <span class="text-[11px] font-semibold text-white"
                    >#{i + 1} {card.momentTitle}</span
                  >
                  <span class="shrink-0 rounded bg-surface-800 px-1.5 py-0.5 font-mono text-[9px] text-vigil-300"
                    >{Math.round(c.score)}</span
                  >
                </div>
                <p class="mt-0.5 text-[10px] text-surface-400">
                  {scoreToHuman(c.score)} · {formatDuration(c.duration)}
                </p>
                <p class="mt-1 line-clamp-2 text-[10px] text-surface-500">
                  {card.whyInteresting[0]}
                </p>
              </button>
            </li>
          {/each}
        </ul>
        {#if interest}
          <div class="border-t border-surface-800 px-3 py-2">
            <p class="text-[9px] font-bold uppercase text-surface-500">Por qué es interesante</p>
            <ul class="mt-1 space-y-0.5">
              {#each interest.whyInteresting as w}
                <li class="text-[10px] text-surface-200">• {w}</li>
              {/each}
            </ul>
            {#if interest.risks.length}
              <p class="mt-1.5 text-[9px] text-amber-500/90">Riesgos: {interest.risks.join(" · ")}</p>
            {/if}
          </div>
        {/if}
      </section>

      <!-- 2. CARA / SUJETO -->
      <section
        class="flex min-h-0 flex-col gap-1.5 overflow-hidden rounded-2xl border border-surface-800 bg-black/40 p-2"
        data-testid="layer-face"
      >
        <header class="flex items-center justify-between gap-2 px-1">
          <div>
            <p class="text-[10px] font-bold uppercase tracking-wide text-vigil-400">2 · Cara / sujeto</p>
            <p class="text-[10px] text-surface-400">
              {interest?.faceHint ?? "Elegí el rostro o sujeto del 9:16"}
            </p>
          </div>
          <span
            class="rounded-full px-2 py-0.5 text-[9px] font-semibold
              {faceConfirmed ? 'bg-keep/20 text-keep' : 'bg-surface-800 text-surface-400'}"
          >{faceConfirmed ? "Cara OK" : "Pendiente"}</span>
        </header>
        {#if sel}
          <div class="min-h-0 flex-1">
            <SourcePreview
              sourcePath={media || sel.sourceMediaPath}
              {start}
              {end}
              {centerX}
              {centerY}
              autoplay={false}
              interactive={true}
              onCenterChange={(x, y) => void onCenter(x, y)}
            />
          </div>
          <div class="flex flex-wrap gap-1.5">
            <button type="button" class="rounded-lg bg-vigil-900/50 px-2 py-1 text-[10px] text-vigil-200 ring-1 ring-vigil-700/40" onclick={() => facePreset(0.35, 0.4)}>Cara izq.</button>
            <button type="button" class="rounded-lg bg-vigil-900/50 px-2 py-1 text-[10px] text-vigil-200 ring-1 ring-vigil-700/40" onclick={() => facePreset(0.5, 0.4)}>Centro</button>
            <button type="button" class="rounded-lg bg-vigil-900/50 px-2 py-1 text-[10px] text-vigil-200 ring-1 ring-vigil-700/40" onclick={() => facePreset(0.65, 0.4)}>Cara der.</button>
            <button
              type="button"
              class="rounded-lg bg-vigil-600 px-2.5 py-1 text-[10px] font-semibold text-white disabled:opacity-40"
              onclick={confirmFace}
            >Confirmar cara</button>
          </div>
          <div class="flex min-h-0 gap-2">
            <div class="min-w-0 flex-1">
              <MiniTimeline
                {start}
                {end}
                editable={true}
                onChange={(s, e) => {
                  start = s;
                  end = e;
                }}
              />
            </div>
            <div class="shrink-0">
              <ShortPreview
                sourcePath={media || sel.sourceMediaPath}
                {start}
                {end}
                {centerX}
                {centerY}
                autoplay={true}
                size="sm"
                framed={true}
                label="9:16"
              />
            </div>
          </div>
        {/if}
      </section>

      <!-- 3. IMÁGENES -->
      <section
        class="flex min-h-0 flex-col overflow-hidden rounded-2xl border border-sky-900/40 bg-sky-950/15"
        data-testid="layer-images"
      >
        <header class="border-b border-surface-800 px-3 py-2">
          <p class="text-[10px] font-bold uppercase tracking-wide text-sky-400">3 · Imágenes</p>
          <p class="text-[10px] text-surface-400">
            B-roll / biblioteca según el texto del momento
          </p>
          {#if lastQuery}
            <p class="mt-0.5 truncate font-mono text-[9px] text-surface-600">buscó: {lastQuery}</p>
          {/if}
        </header>
        <div class="min-h-0 flex-1 overflow-y-auto p-2">
          {#if imagesLoading}
            <p class="text-xs text-surface-500">Buscando en la biblioteca…</p>
          {:else if imagesError}
            <p class="text-[11px] text-amber-400">No se pudo consultar la biblioteca.</p>
            <p class="mt-1 text-[10px] text-surface-500">{imagesError}</p>
          {:else if imageAssets.length === 0}
            <div class="rounded-xl border border-dashed border-surface-700 p-3 text-center">
              <p class="text-xs text-surface-300">No hay imágenes coincidentes</p>
              <p class="mt-1 text-[10px] text-surface-500">
                Importá B-roll en Herramientas anteriores → Biblioteca, o seguí sin imágenes.
              </p>
            </div>
          {:else}
            <ul class="grid grid-cols-2 gap-1.5">
              {#each imageAssets as a (a.id)}
                {@const thumb = pathToPlayableUrl(a.previewPath || a.path || null)}
                {@const on = selectedImageIds.includes(a.id)}
                <li>
                  <button
                    type="button"
                    class="w-full overflow-hidden rounded-lg border text-left
                      {on ? 'border-sky-400 ring-1 ring-sky-500/40' : 'border-surface-700'}"
                    onclick={() => toggleImage(a.id)}
                  >
                    <div class="aspect-video bg-surface-900">
                      {#if thumb}
                        <img src={thumb} alt="" class="h-full w-full object-cover" />
                      {:else}
                        <div class="flex h-full items-center justify-center text-[9px] text-surface-600">img</div>
                      {/if}
                    </div>
                    <p class="truncate px-1 py-0.5 text-[9px] text-surface-300">
                      {a.title || a.fileName || a.id.slice(0, 8)}
                    </p>
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
        <div class="flex flex-wrap gap-1.5 border-t border-surface-800 p-2">
          <button
            type="button"
            class="rounded-lg border border-surface-600 px-2 py-1 text-[10px] text-surface-300 hover:bg-surface-900"
            onclick={() => void loadImagesForMoment()}
          >Buscar de nuevo</button>
          <button
            type="button"
            class="rounded-lg border border-surface-600 px-2 py-1 text-[10px] text-surface-300 hover:bg-surface-900
              {imagesMode === 'skip' ? 'border-amber-600 text-amber-300' : ''}"
            onclick={skipImages}
          >Sin B-roll (explícito)</button>
          <span class="ml-auto text-[9px] text-surface-500">
            {imagesMode === "picked"
              ? `${selectedImageIds.length} elegida(s)`
              : imagesMode === "skip"
                ? "Sin imágenes"
                : "Elegí o saltá"}
          </span>
        </div>
      </section>
    </div>

    <StationDock
      {guide}
      label="Usar este clip"
      pending={vnextSession.pendingAction === "approve"}
      pendingLabel="Guardando…"
      disabled={!canApproveShort(gate) || !!vnextSession.pendingAction}
      secondaryLabel={sel ? "Descartar momento" : null}
      hint={canApproveShort(gate)
        ? "Siguiente: Usar este clip y pasar a Ajuste"
        : `Falta: ${[
            !gate.momentOk ? "momento" : null,
            !gate.faceOk ? "confirmar cara" : null,
            !gate.imagesOk ? "imágenes o Sin B-roll" : null,
          ]
            .filter(Boolean)
            .join(" · ")}`}
      onPrimary={() => void useClip()}
      onSecondary={() => {
        if (sel) void vnextSession.decideDiscardCandidate(sel.id);
      }}
    />
  {/if}
</div>
