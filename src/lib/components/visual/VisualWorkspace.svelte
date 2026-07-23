<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";
  import DailySettings from "./DailySettings.svelte";
  import type { CandidateView, NeedSupervision, SupervisionSnapshot } from "./imageGenTypes";
  import LibraryView from "./LibraryView.svelte";
  import type {
    AssetUsageRow,
    LicenseStatus,
    MediaAsset,
    VisualsViewId,
  } from "./library/libraryTypes";
  import ReviewInbox from "./ReviewInbox.svelte";
  import VideoVisualsView from "./VideoVisualsView.svelte";
  import VisualPicker from "./VisualPicker.svelte";

  let {
    projectKey = $bindable<string | null>(null),
    compact = false,
    onMessage = (_m: string) => {},
    onError = (_e: string) => {},
    onPlanUpdated = (_p: unknown) => {},
  }: {
    projectKey?: string | null;
    /** true = panel derecho dentro de VisualPanel */
    compact?: boolean;
    onMessage?: (m: string) => void;
    onError?: (e: string) => void;
    onPlanUpdated?: (p: unknown) => void;
  } = $props();

  let view = $state<VisualsViewId>("library");
  let searchQ = $state("");
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let assets = $state<MediaAsset[]>([]);
  let assetsLoading = $state(false);
  let selectedAssetId = $state<string | null>(null);
  let usage = $state<AssetUsageRow[]>([]);
  let usageLoading = $state(false);
  let snap = $state<SupervisionSnapshot | null>(null);
  let busy = $state(false);
  let busyId = $state<string | null>(null);
  let menuOpen = $state(false);
  let dailyOpen = $state(false);
  let dailyEnabled = $state(false);
  let weekMsg = $state("");
  let importMenu = $state(false);
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  let pickerOpen = $state(false);
  let pickerNeed = $state<NeedSupervision | null>(null);
  let pickerMatches = $state<MediaAsset[]>([]);
  let pickerLoading = $state(false);

  const hasVideo = $derived(!!projectStore.mediaPath);
  const pending = $derived(snap?.pendingReview ?? []);
  const needs = $derived(snap?.needs ?? []);
  const coverage = $derived(snap?.coverage ?? null);
  const sceneLabel = $derived(
    pickerNeed
      ? null
      : needs.find((n) => n.uiState === "uncovered" || n.need.coverage === "uncovered")
        ? null
        : null,
  );

  let didInitView = $state(false);
  $effect(() => {
    if (!didInitView) {
      didInitView = true;
      view = hasVideo ? "video" : "library";
    } else if (!hasVideo && view === "video") {
      view = "library";
    }
  });

  $effect(() => {
    void loadAssets(null);
    void loadDaily();
    void refreshSnap();
    return () => {
      if (pollTimer) clearInterval(pollTimer);
      if (searchTimer) clearTimeout(searchTimer);
    };
  });

  async function loadAssets(q: string | null) {
    assetsLoading = true;
    try {
      const list = (await api.visualListAssets(q, 250)) as MediaAsset[];
      assets = Array.isArray(list) ? list : [];
      if (selectedAssetId && !assets.some((a) => a.id === selectedAssetId)) {
        selectedAssetId = assets[0]?.id ?? null;
      } else if (!selectedAssetId && assets[0]) {
        selectedAssetId = assets[0].id;
      }
    } catch (e) {
      onError(String(e));
    } finally {
      assetsLoading = false;
    }
  }

  function onSearchInput(v: string) {
    searchQ = v;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void loadAssets(v.trim() || null);
      if (view !== "library") view = "library";
    }, 300);
  }

  async function loadDaily() {
    try {
      const s = (await api.visualDailyFeedSettings()) as { enabled?: boolean };
      dailyEnabled = !!s.enabled;
      const w = (await api.visualDailyWeekSummary()) as { message?: string };
      weekMsg = w.message ?? "";
    } catch {
      /* ignore */
    }
  }

  async function refreshSnap() {
    try {
      if (projectKey) {
        snap = (await api.visualSupervision(projectKey)) as SupervisionSnapshot;
      } else {
        snap = (await api.visualSupervisionGlobal()) as SupervisionSnapshot;
      }
      if (snap?.dailyFeed) dailyEnabled = !!snap.dailyFeed.enabled;
    } catch {
      /* ignore */
    }
  }

  function startPoll() {
    if (pollTimer) return;
    pollTimer = setInterval(() => {
      void refreshSnap().then(() => {
        const still = needs.some((n) =>
          ["queued", "processing", "cancelling"].includes(n.uiState),
        );
        if (!still && pollTimer) {
          clearInterval(pollTimer);
          pollTimer = null;
        }
      });
    }, 2000);
  }

  async function loadUsage(id: string) {
    usageLoading = true;
    try {
      const rows = (await api.visualListUsage(id, 20)) as AssetUsageRow[];
      usage = Array.isArray(rows) ? rows : [];
    } catch {
      usage = [];
    } finally {
      usageLoading = false;
    }
  }

  $effect(() => {
    if (selectedAssetId) void loadUsage(selectedAssetId);
  });

  async function detect() {
    if (!projectStore.mediaPath) {
      onError("Abre un video primero");
      return;
    }
    busy = true;
    try {
      const res = (await api.visualDetectNeeds({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
      })) as { projectKey?: string };
      projectKey = res.projectKey ?? projectKey;
      await refreshSnap();
      view = "video";
      onMessage("Momentos visuales actualizados");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function openPicker(n: NeedSupervision) {
    if (n.uiState === "needs_human_review" || n.uiState === "reviewing") {
      view = "review";
      return;
    }
    pickerNeed = n;
    pickerOpen = true;
    pickerLoading = true;
    pickerMatches = [];
    try {
      const res = (await api.visualSearchLibraryForNeed(n.need.id)) as {
        candidates?: {
          assetId?: string;
          asset_id?: string;
          assetTitle?: string;
          thumbnailPath?: string | null;
          score?: number;
        }[];
      };
      const ranked = Array.isArray(res.candidates) ? res.candidates : [];
      const byId = new Map(assets.map((a) => [a.id, a]));
      const fromRank: MediaAsset[] = [];
      for (const c of ranked) {
        const id = c.assetId || c.asset_id;
        if (!id) continue;
        const full = byId.get(id);
        if (full) {
          fromRank.push(full);
        } else {
          fromRank.push({
            id,
            kind: "image",
            managedPath: c.thumbnailPath || "",
            thumbnailPath: c.thumbnailPath,
            sha256: "",
            title: c.assetTitle || id.slice(0, 8),
            tags: [],
            concepts: [],
            width: 0,
            height: 0,
            orientation: "landscape",
            mimeType: "image/*",
            fileSize: 0,
            licenseStatus: "unknown",
            timesUsed: 0,
            allowSameVideoRepeat: false,
            minimumVideosBeforeReuse: 0,
            status: "active",
            createdAt: "",
            updatedAt: "",
          } as MediaAsset);
        }
      }
      if (fromRank.length > 0) {
        pickerMatches = fromRank.slice(0, 8);
      } else {
        const terms = (n.need.terms ?? [n.need.label]).map((t) => t.toLowerCase());
        pickerMatches = assets
          .filter((a) => {
            const blob = `${a.title} ${(a.concepts ?? []).join(" ")} ${(a.tags ?? []).join(" ")}`.toLowerCase();
            return terms.some((t) => t.length > 2 && blob.includes(t));
          })
          .slice(0, 8);
      }
    } catch {
      pickerMatches = [];
    } finally {
      pickerLoading = false;
    }
  }

  async function useAssetOnNeed(assetId: string) {
    if (!pickerNeed || !projectStore.mediaPath) return;
    busy = true;
    try {
      await api.visualAssignNeedAsset(pickerNeed.need.id, assetId);
      const s = pickerNeed.need.outputStart ?? 0;
      const e = pickerNeed.need.outputEnd ?? s + 4;
      const res = (await api.visualCreateManualPlacement({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        assetId,
        outputStart: s,
        outputEnd: e,
        displayMode: "completa",
        sourceDuration: projectStore.duration,
        label: pickerNeed.need.label,
      })) as { plan?: unknown; message?: string };
      if (res.plan) onPlanUpdated(res.plan);
      try {
        const applied = (await api.visualApplyNeedsToPlan({
          mediaPath: projectStore.mediaPath,
          analysisRunId: projectStore.analysisRun?.id ?? null,
          projectKey: projectKey ?? pickerNeed.need.projectKey,
        })) as { plan?: unknown };
        if (applied.plan) onPlanUpdated(applied.plan);
      } catch {
        /* placement already done */
      }
      onMessage(res.message ?? "Imagen en el video");
      pickerOpen = false;
      pickerNeed = null;
      await refreshSnap();
      await loadAssets(searchQ.trim() || null);
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  /** Import and place at current playhead (no need required). */
  async function placeAtPlayhead() {
    if (!projectStore.mediaPath || !api.isTauri()) {
      onError("Abre un video primero");
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Imagen", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (typeof p !== "string") return;
      busy = true;
      const t = projectStore.outputClock() || 0;
      const s = Math.max(0, t - 0.2);
      const e = s + 3.8;
      const res = (await api.visualCreateManualPlacement({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        imagePath: p,
        outputStart: s,
        outputEnd: e,
        displayMode: "completa",
        sourceDuration: projectStore.duration,
        label: p.split(/[/\\]/).pop() ?? "imagen",
      })) as { plan?: unknown; message?: string };
      if (res.plan) onPlanUpdated(res.plan);
      await loadAssets(null);
      onMessage(res.message ?? "Imagen colocada en el playhead");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function pickerImport() {
    if (!api.isTauri()) {
      onError("Disponible en la aplicación de escritorio");
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Imagen", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (typeof p !== "string") return;
      busy = true;
      const a = (await api.visualImportImage(p)) as MediaAsset;
      await loadAssets(searchQ.trim() || null);
      selectedAssetId = a.id;
      if (pickerNeed) {
        await useAssetOnNeed(a.id);
      } else {
        view = "library";
        onMessage("Imagen importada a la biblioteca");
      }
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
      importMenu = false;
    }
  }

  async function importFolder() {
    if (!api.isTauri()) {
      onError("Disponible en la aplicación de escritorio");
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({ directory: true });
      if (typeof p !== "string") return;
      busy = true;
      const r = (await api.visualImportFolder(p, [], [], false)) as {
        imported?: number;
        duplicates?: number;
        failed?: number;
        scanned?: number;
      };
      await loadAssets(searchQ.trim() || null);
      onMessage(
        `Carpeta: ${r.imported ?? 0} nuevas, ${r.duplicates ?? 0} duplicadas` +
          (r.failed ? `, ${r.failed} errores` : ""),
      );
      view = "library";
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
      importMenu = false;
    }
  }

  async function pickerGenerate() {
    if (!pickerNeed) return;
    busy = true;
    try {
      await api.visualGenerateNeed(pickerNeed.need.id);
      onMessage("En cola — se generará en segundo plano");
      pickerOpen = false;
      startPoll();
      await refreshSnap();
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function pickerSkip() {
    if (!pickerNeed) return;
    try {
      await api.visualSkipNeed(pickerNeed.need.id);
      onMessage("Escena sin imagen");
      pickerOpen = false;
      await refreshSnap();
    } catch (e) {
      onError(String(e));
    }
  }

  async function cancelJob(jobId: string) {
    try {
      await api.visualCancelJob(jobId);
      onMessage("Cancelación solicitada");
      startPoll();
      await refreshSnap();
    } catch (e) {
      onError(String(e));
    }
  }

  async function approve(c: CandidateView, place: boolean) {
    busyId = c.id;
    try {
      const res = (await api.visualApproveAndUse({
        candidateId: c.id,
        mediaPath: place ? projectStore.mediaPath : null,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        place,
      })) as { message?: string; placementAdded?: boolean };
      onMessage(res.message ?? "Aprobada");
      if (res.placementAdded) {
        try {
          const sess = (await api.visualGetSession()) as { plan?: unknown };
          if (sess.plan) onPlanUpdated(sess.plan);
        } catch {
          /* ignore */
        }
      }
      await refreshSnap();
      await loadAssets(searchQ.trim() || null);
    } catch (e) {
      onError(String(e));
    } finally {
      busyId = null;
    }
  }

  async function reject(c: CandidateView, reason: string) {
    busyId = c.id;
    try {
      await api.visualRejectCandidate(c.id, reason);
      onMessage("Imagen rechazada");
      await refreshSnap();
    } catch (e) {
      onError(String(e));
    } finally {
      busyId = null;
    }
  }

  async function regenerate(needId: string) {
    try {
      await api.visualRegenerateNeed(needId);
      onMessage("Regeneración en cola");
      startPoll();
      await refreshSnap();
    } catch (e) {
      onError(String(e));
    }
  }

  async function saveAsset(
    id: string,
    patch: { title: string; tags: string[]; concepts: string[]; license: LicenseStatus },
  ) {
    busy = true;
    try {
      const a = (await api.visualUpdateAsset({
        id,
        title: patch.title,
        tags: patch.tags,
        concepts: patch.concepts,
        license: patch.license,
      })) as MediaAsset;
      assets = assets.map((x) => (x.id === id ? a : x));
      onMessage("Cambios guardados");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function setStatus(id: string, status: string) {
    busy = true;
    try {
      const a = (await api.visualUpdateAsset({ id, status })) as MediaAsset;
      assets = assets.map((x) => (x.id === id ? a : x));
      onMessage(status === "archived" ? "Asset archivado" : "Actualizado");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function scanMissing() {
    try {
      const n = await api.visualScanMissing();
      onMessage(`Se marcaron ${n} archivos ausentes`);
      await loadAssets(searchQ.trim() || null);
    } catch (e) {
      onError(String(e));
    }
  }

  async function toggleDaily(v: boolean) {
    try {
      await api.visualDailyFeedSetEnabled(v);
      dailyEnabled = v;
      onMessage(v ? "Biblioteca automática activada" : "Biblioteca automática desactivada");
      if (v) {
        const r = (await api.visualDailyFeedCycle()) as { ok?: boolean; reason?: string };
        if (r.ok) startPoll();
        else onMessage(`Daily: ${r.reason ?? "ok"}`);
      }
      await loadDaily();
      await refreshSnap();
    } catch (e) {
      onError(String(e));
    }
  }

  function primaryNeed(n: NeedSupervision) {
    void openPicker(n);
  }
</script>

<div
  class="flex h-full min-h-0 min-w-0 flex-col overflow-hidden {compact
    ? ''
    : 'rounded-xl border border-surface-800 bg-surface-900/40'}"
  class:p-2={!compact}
>
  <!-- Shell header -->
  <header class="shrink-0 space-y-1.5 border-b border-surface-800 pb-2">
    {#if !compact}
      <div>
        <h1 class="text-sm font-semibold text-surface-50">Visuales</h1>
        <p class="text-[10px] text-surface-500">
          Encuentra, revisa y usa imágenes sin salir de tu proyecto.
        </p>
      </div>
    {:else}
      <div class="text-[11px] font-semibold text-surface-200">Visuales</div>
    {/if}

    <div class="flex gap-1">
      <input
        class="min-w-0 flex-1 rounded-lg border border-surface-700 bg-surface-950 px-2 py-1.5 text-[11px]"
        placeholder="Buscar imágenes…"
        value={searchQ}
        oninput={(e) => onSearchInput((e.currentTarget as HTMLInputElement).value)}
      />
      <div class="relative">
        <button
          type="button"
          class="btn-primary text-[10px]"
          disabled={busy || !api.isTauri()}
          title={!api.isTauri() ? "Disponible en la aplicación de escritorio" : "Importar"}
          onclick={() => (importMenu = !importMenu)}
        >
          Importar
        </button>
        {#if importMenu}
          <div
            class="absolute right-0 z-20 mt-1 w-40 rounded-lg border border-surface-700 bg-surface-900 py-1 shadow-xl"
          >
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-surface-800"
              onclick={() => void pickerImport()}>Importar imagen</button
            >
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-surface-800"
              onclick={() => void importFolder()}>Importar carpeta</button
            >
          </div>
        {/if}
      </div>
      <div class="relative">
        <button
          type="button"
          class="btn-ghost px-2 text-[12px]"
          aria-label="Más opciones"
          onclick={() => (menuOpen = !menuOpen)}>⋯</button
        >
        {#if menuOpen}
          <div
            class="absolute right-0 z-20 mt-1 w-52 rounded-lg border border-surface-700 bg-surface-900 py-1 shadow-xl"
          >
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-surface-800"
              onclick={() => {
                menuOpen = false;
                dailyOpen = true;
              }}>Biblioteca automática…</button
            >
            {#if hasVideo}
              <button
                type="button"
                class="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-surface-800"
                onclick={() => {
                  menuOpen = false;
                  void detect();
                }}>Detectar momentos</button
              >
            {/if}
            <button
              type="button"
              class="block w-full px-3 py-1.5 text-left text-[11px] hover:bg-surface-800"
              onclick={() => {
                menuOpen = false;
                void scanMissing();
              }}>Buscar archivos ausentes</button
            >
          </div>
        {/if}
      </div>
    </div>

    <div class="grid grid-cols-3 gap-1" role="tablist" aria-label="Vistas de Visuales">
      <button
        type="button"
        role="tab"
        aria-selected={view === "video"}
        disabled={!hasVideo}
        title={!hasVideo ? "Abre un video para ver qué imágenes necesita" : undefined}
        class="rounded-lg px-1.5 py-1.5 text-[10px] font-semibold transition
          {view === 'video' ? 'bg-sky-600 text-white' : 'bg-surface-800 text-surface-300'}
          {!hasVideo ? 'cursor-not-allowed opacity-40' : 'hover:bg-surface-700'}"
        onclick={() => hasVideo && (view = "video")}
      >
        Este video
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={view === "library"}
        class="rounded-lg px-1.5 py-1.5 text-[10px] font-semibold transition
          {view === 'library' ? 'bg-violet-600 text-white' : 'bg-surface-800 text-surface-300 hover:bg-surface-700'}"
        onclick={() => (view = "library")}
      >
        Biblioteca
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={view === "review"}
        class="rounded-lg px-1.5 py-1.5 text-[10px] font-semibold transition
          {view === 'review' ? 'bg-amber-600 text-white' : 'bg-surface-800 text-surface-300 hover:bg-surface-700'}"
        onclick={() => (view = "review")}
      >
        Por revisar
        {#if pending.length > 0}
          <span class="ml-0.5 rounded-full bg-black/30 px-1 text-[9px]">{pending.length}</span>
        {/if}
      </button>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-hidden pt-2" aria-live="polite">
    {#if dailyOpen}
      <div class="mb-2 rounded-lg border border-surface-800 p-2">
        <DailySettings
          enabled={dailyEnabled}
          {weekMsg}
          {busy}
          onToggle={(v) => void toggleDaily(v)}
          onRunNow={() => void toggleDaily(true)}
        />
        <button type="button" class="btn-ghost mt-2 text-[10px]" onclick={() => (dailyOpen = false)}
          >Cerrar</button
        >
      </div>
    {/if}

    {#if view === "video"}
      {#if !hasVideo}
        <p class="p-2 text-[11px] text-surface-400">
          Abre un video para ver qué imágenes necesita.
        </p>
      {:else}
        <VideoVisualsView
          {needs}
          {coverage}
          {busy}
          onDetect={() => void detect()}
          onPrimary={primaryNeed}
          onCancel={(id) => void cancelJob(id)}
          onPlacePlayhead={() => void placeAtPlayhead()}
        />
      {/if}
    {:else if view === "library"}
      <LibraryView
        {assets}
        loading={assetsLoading}
        selectedId={selectedAssetId}
        {usage}
        {usageLoading}
        {busy}
        sceneLabel={null}
        onSelect={(id) => (selectedAssetId = id)}
        onSave={saveAsset}
        onArchive={(id) => setStatus(id, "archived")}
        onRestore={(id) => setStatus(id, "active")}
        onBlock={(id) => {
          if (confirm("¿Bloquear este asset? No se sugerirá en matching.")) {
            void setStatus(id, "blocked");
          }
          return Promise.resolve();
        }}
      />
    {:else}
      <ReviewInbox
        candidates={pending}
        {busyId}
        onApprove={(c, place) => void approve(c, place)}
        onReject={(c, reason) => void reject(c, reason)}
        onRegenerate={(nid) => void regenerate(nid)}
      />
    {/if}
  </div>
</div>

<VisualPicker
  open={pickerOpen}
  need={pickerNeed}
  matches={pickerMatches}
  matchesLoading={pickerLoading}
  {busy}
  onClose={() => {
    pickerOpen = false;
    pickerNeed = null;
  }}
  onUseAsset={(id) => void useAssetOnNeed(id)}
  onImport={() => void pickerImport()}
  onGenerate={() => void pickerGenerate()}
  onSkip={() => void pickerSkip()}
/>
