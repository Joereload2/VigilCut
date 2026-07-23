<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/utils/tauri";

  type Dashboard = {
    inventory: {
      totalAssets: number;
      activeAssets: number;
      missingAssets: number;
      pendingCandidates: number;
      managedBytes: number;
    };
    coverage: {
      totalConcepts: number;
      coveredConcepts: number;
      uncoveredConcepts: number;
    };
    activity: { queued: number; running: number; cancelling: number; failedToday: number };
    provider: {
      name: string;
      configured: boolean;
      reachable: boolean;
      supportsImage: boolean;
      freeVerified: boolean;
      costKind: string;
      error?: string | null;
    };
    limits: {
      localDailyLimit: number;
      localUsedToday: number;
      localRemainingToday: number;
      providerRemaining?: number | null;
    };
    canWork: boolean;
    blockedReason?: string | null;
  };

  type RequestRow = {
    id: string;
    title: string;
    targetCount: number;
    desiredFormat: string;
    status: string;
    usefulAssets: number;
    queued: number;
    running: number;
    awaitingReview: number;
    failed: number;
    deficit: number;
    updatedAt: string;
  };

  type Preview = {
    request: RequestRow;
    canConfirm: boolean;
    maxEnqueueable: number;
    blockedReason?: string | null;
    provider: string;
  };

  type ConceptRow = {
    conceptId: string;
    title: string;
    usefulAssets: number;
    pendingCandidates: number;
    activeRequests: number;
    requestedTarget: number;
    state: "covered" | "partial" | "in_review" | "uncovered";
  };
  let { onReview = () => {} }: { onReview?: () => void } = $props();

  let dashboard = $state<Dashboard | null>(null);
  let requests = $state<RequestRow[]>([]);
  let concepts = $state<ConceptRow[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let probing = $state(false);
  let error = $state("");
  let notice = $state("");
  let advanced = $state(false);
  let preview = $state<Preview | null>(null);

  let instruction = $state("");
  let targetCount = $state(3);
  let desiredFormat = $state<"16:9" | "9:16" | "1:1" | "4:5">("16:9");
  let positive = $state("");
  let negative = $state("");
  let exclusions = $state("logos, marcas de agua, texto ilegible");
  let priority = $state(50);

  const activityCount = $derived(
    (dashboard?.activity.queued ?? 0) +
      (dashboard?.activity.running ?? 0) +
      (dashboard?.activity.cancelling ?? 0),
  );
  const providerLabel = $derived.by(() => {
    const p = dashboard?.provider;
    if (!p) return "Comprobando";
    if (p.name === "mock") return "Simulador local";
    if (!p.configured) return "OmniRoute sin configurar";
    if (!p.reachable) return "OmniRoute no disponible";
    if (!p.supportsImage) return "Imágenes no verificadas";
    if (p.freeVerified) return "OmniRoute listo · gratuito verificado";
    return "OmniRoute configurado · coste no verificado";
  });

  function splitValues(value: string): string[] {
    return value
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
  }

  function formatBytes(bytes: number): string {
    if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  async function refresh() {
    loading = true;
    error = "";
    try {
      const [nextDashboard, nextRequests, nextConcepts] = await Promise.all([
        api.visualLibraryDashboard(),
        api.visualLibraryListRequests(30),
        api.visualLibraryConceptCoverage(100),
      ]);
      dashboard = nextDashboard as Dashboard;
      requests = Array.isArray(nextRequests) ? (nextRequests as RequestRow[]) : [];
      concepts = Array.isArray(nextConcepts) ? (nextConcepts as ConceptRow[]) : [];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function prepare() {
    if (instruction.trim().length < 3) {
      error = "Describe el concepto que quieres cubrir.";
      return;
    }
    busy = true;
    error = "";
    notice = "";
    try {
      preview = (await api.visualLibraryCreateRequest({
        title: instruction.trim(),
        targetCount,
        desiredFormat,
        positiveContexts: splitValues(positive),
        negativeContexts: splitValues(negative),
        hardExclusions: splitValues(exclusions),
        priority,
      })) as Preview;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function confirmRequest(requestId: string) {
    busy = true;
    error = "";
    notice = "";
    try {
      const result = (await api.visualLibraryConfirmRequest(requestId)) as Preview;
      preview = null;
      instruction = "";
      notice =
        result.request.deficit === 0
          ? "El concepto ya estaba cubierto."
          : `Solicitud iniciada con ${result.request.queued + result.request.running} trabajo(s).`;
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function cancelRequest(requestId: string) {
    busy = true;
    error = "";
    try {
      await api.visualLibraryCancelRequest(requestId);
      notice = "Cancelación solicitada.";
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function probeProvider() {
    probing = true;
    error = "";
    try {
      await api.visualProbeImageProvider();
      await refresh();
    } catch (e) {
      error = String(e);
    } finally {
      probing = false;
    }
  }

  onMount(() => {
    void refresh();
    const timer = window.setInterval(() => {
      if (activityCount > 0) void refresh();
    }, 3000);
    return () => window.clearInterval(timer);
  });
</script>

<section class="space-y-3" aria-labelledby="library-control-title">
  <div class="flex flex-wrap items-start justify-between gap-2">
    <div>
      <h2 id="library-control-title" class="text-sm font-semibold text-white">
        Centro de control
      </h2>
      <p class="text-[11px] text-surface-400">
        Amplía la biblioteca por conceptos, sin abrir un video.
      </p>
    </div>
    <button
      type="button"
      class="btn-ghost text-[10px]"
      disabled={loading}
      onclick={() => void refresh()}
      aria-label="Actualizar estado de la biblioteca"
    >
      {loading ? "Actualizando…" : "Actualizar"}
    </button>
  </div>

  {#if dashboard}
    <div class="grid grid-cols-2 gap-2 lg:grid-cols-4">
      <div class="rounded-xl border border-surface-800 bg-surface-950/50 p-2">
        <p class="text-[10px] text-surface-500">Imágenes alojadas</p>
        <p class="mt-0.5 text-lg font-semibold text-white">{dashboard.inventory.activeAssets}</p>
        <p class="text-[9px] text-surface-500">
          {formatBytes(dashboard.inventory.managedBytes)}
          {#if dashboard.inventory.missingAssets > 0}
            · {dashboard.inventory.missingAssets} ausentes
          {/if}
        </p>
      </div>
      <div class="rounded-xl border border-surface-800 bg-surface-950/50 p-2">
        <p class="text-[10px] text-surface-500">Conceptos cubiertos</p>
        <p class="mt-0.5 text-lg font-semibold text-white">
          {dashboard.coverage.coveredConcepts}/{dashboard.coverage.totalConcepts}
        </p>
        <p class="text-[9px] text-surface-500">
          {dashboard.coverage.uncoveredConcepts} pendientes
        </p>
      </div>
      <div class="rounded-xl border border-surface-800 bg-surface-950/50 p-2">
        <p class="text-[10px] text-surface-500">Actividad</p>
        <p class="mt-0.5 text-lg font-semibold {activityCount ? 'text-sky-300' : 'text-white'}">
          {activityCount ? `${activityCount} en curso` : "En reposo"}
        </p>
        <p class="text-[9px] text-surface-500">
          {dashboard.inventory.pendingCandidates} por revisar ·
          {dashboard.activity.failedToday} fallidas hoy
        </p>
      </div>
      <div class="rounded-xl border border-surface-800 bg-surface-950/50 p-2">
        <p class="text-[10px] text-surface-500">Límite local de hoy</p>
        <p class="mt-0.5 text-lg font-semibold text-white">
          {dashboard.limits.localRemainingToday} disponibles
        </p>
        <p class="text-[9px] text-surface-500">
          {dashboard.limits.localUsedToday}/{dashboard.limits.localDailyLimit} usados
        </p>
      </div>
    </div>

    <div
      class="flex flex-wrap items-center justify-between gap-2 rounded-lg border px-2.5 py-2
        {dashboard.canWork
          ? 'border-emerald-800/60 bg-emerald-950/30'
          : 'border-amber-800/60 bg-amber-950/30'}"
    >
      <div class="min-w-0">
        <p class="text-[11px] font-medium text-surface-100">{providerLabel}</p>
        <p class="text-[9px] text-surface-400">
          {dashboard.blockedReason ??
            (dashboard.provider.name === "mock"
              ? "Las imágenes generadas serán fixtures de prueba, no generación IA real."
              : "Proveedor autorizado para nuevas solicitudes.")}
        </p>
      </div>
      <button
        type="button"
        class="btn-secondary shrink-0 text-[10px]"
        disabled={probing}
        onclick={() => void probeProvider()}
      >
        {probing ? "Comprobando…" : "Comprobar OmniRoute"}
      </button>
    </div>
  {/if}

  <form
    class="rounded-xl border border-violet-800/50 bg-violet-950/20 p-3"
    onsubmit={(event) => {
      event.preventDefault();
      void prepare();
    }}
  >
    <label for="library-instruction" class="block text-[11px] font-semibold text-white">
      ¿Qué imágenes debe buscar o crear?
    </label>
    <div class="mt-1.5 flex flex-col gap-2 sm:flex-row">
      <input
        id="library-instruction"
        class="min-w-0 flex-1 rounded-lg border border-surface-700 bg-surface-950 px-3 py-2 text-xs text-white placeholder:text-surface-600"
        bind:value={instruction}
        placeholder="Ej.: personas comparando precios en un supermercado"
        maxlength="180"
      />
      <label class="flex items-center gap-1 text-[10px] text-surface-400">
        Cantidad
        <input
          type="number"
          class="w-14 rounded border border-surface-700 bg-surface-950 px-1.5 py-2 text-center text-xs text-white"
          min="1"
          max="10"
          bind:value={targetCount}
        />
      </label>
      <button type="submit" class="btn-primary px-4 text-xs" disabled={busy}>
        {busy ? "Preparando…" : "Revisar solicitud"}
      </button>
    </div>
    <button
      type="button"
      class="mt-2 text-[10px] text-violet-300 underline"
      aria-expanded={advanced}
      onclick={() => (advanced = !advanced)}
    >
      {advanced ? "Ocultar detalles" : "Añadir contexto y exclusiones"}
    </button>
    {#if advanced}
      <div class="mt-2 grid gap-2 md:grid-cols-2">
        <label class="text-[10px] text-surface-400">
          Contexto deseado
          <input
            class="mt-1 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1.5 text-[11px] text-white"
            bind:value={positive}
            placeholder="ciudad, luz natural"
          />
        </label>
        <label class="text-[10px] text-surface-400">
          Evitar
          <input
            class="mt-1 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1.5 text-[11px] text-white"
            bind:value={negative}
            placeholder="lujo, celebridades"
          />
        </label>
        <label class="text-[10px] text-surface-400 md:col-span-2">
          Exclusiones obligatorias
          <input
            class="mt-1 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1.5 text-[11px] text-white"
            bind:value={exclusions}
          />
        </label>
        <label class="text-[10px] text-surface-400">
          Formato
          <select
            class="mt-1 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1.5 text-[11px] text-white"
            bind:value={desiredFormat}
          >
            <option value="16:9">Horizontal 16:9</option>
            <option value="9:16">Vertical 9:16</option>
            <option value="1:1">Cuadrada 1:1</option>
            <option value="4:5">Retrato 4:5</option>
          </select>
        </label>
        <label class="text-[10px] text-surface-400">
          Prioridad: {priority}
          <input class="mt-2 w-full" type="range" min="0" max="100" bind:value={priority} />
        </label>
      </div>
    {/if}
  </form>

  {#if preview}
    <div class="rounded-xl border border-sky-700/60 bg-sky-950/30 p-3" role="status">
      <div class="flex flex-wrap items-start justify-between gap-2">
        <div>
          <p class="text-xs font-semibold text-white">{preview.request.title}</p>
          <p class="mt-1 text-[10px] text-surface-300">
            Ya hay {preview.request.usefulAssets} útil(es); faltan {preview.request.deficit}.
            Se pueden iniciar {preview.maxEnqueueable} ahora mediante {preview.provider}.
          </p>
          {#if preview.blockedReason}
            <p class="mt-1 text-[10px] text-amber-300">{preview.blockedReason}</p>
          {/if}
        </div>
        <div class="flex gap-1">
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (preview = null)}>
            Ahora no
          </button>
          <button
            type="button"
            class="btn-primary text-[10px]"
            disabled={!preview.canConfirm || busy || preview.request.deficit === 0}
            onclick={() => void confirmRequest(preview!.request.id)}
          >
            Confirmar e iniciar
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if error}
    <p class="rounded-lg border border-red-800/60 bg-red-950/30 px-2 py-1.5 text-[10px] text-red-200" role="alert">
      {error}
    </p>
  {:else if notice}
    <p class="rounded-lg border border-emerald-800/60 bg-emerald-950/30 px-2 py-1.5 text-[10px] text-emerald-200" role="status">
      {notice}
    </p>
  {/if}

  {#if requests.length > 0}
    <div>
      <div class="mb-1.5 flex items-center justify-between">
        <h3 class="text-[11px] font-semibold text-surface-200">Solicitudes recientes</h3>
        {#if (dashboard?.inventory.pendingCandidates ?? 0) > 0}
          <button type="button" class="text-[10px] text-amber-300 underline" onclick={onReview}>
            Revisar candidatos
          </button>
        {/if}
      </div>
      <div class="space-y-1.5">
        {#each requests as request (request.id)}
          <article class="flex flex-wrap items-center gap-2 rounded-lg border border-surface-800 bg-surface-950/40 px-2.5 py-2">
            <div class="min-w-[180px] flex-1">
              <p class="truncate text-[11px] font-medium text-surface-100">{request.title}</p>
              <p class="text-[9px] text-surface-500">
                {request.usefulAssets}/{request.targetCount} cubiertas · {request.desiredFormat}
              </p>
            </div>
            <div class="text-right text-[9px] text-surface-400">
              {#if request.running > 0}
                <span class="text-sky-300">{request.running} generando</span>
              {:else if request.queued > 0}
                <span class="text-sky-300">{request.queued} en cola</span>
              {:else if request.awaitingReview > 0}
                <span class="text-amber-300">{request.awaitingReview} por revisar</span>
              {:else if request.deficit === 0}
                <span class="text-emerald-300">Cubierto</span>
              {:else if request.failed > 0}
                <span class="text-red-300">{request.failed} fallida(s)</span>
              {:else}
                <span>{request.status}</span>
              {/if}
            </div>
            {#if request.queued + request.running > 0}
              <button
                type="button"
                class="btn-ghost text-[9px]"
                disabled={busy}
                onclick={() => void cancelRequest(request.id)}
              >
                Cancelar
              </button>
            {/if}
          </article>
        {/each}
      </div>
    </div>
  {/if}
  <div>
    <h3 class="mb-1.5 text-[11px] font-semibold text-surface-200">Cobertura por concepto</h3>
    {#if concepts.length === 0}
      <p class="rounded-lg border border-dashed border-surface-700 p-2 text-[10px] text-surface-500">
        Aún no hay conceptos. Crea la primera solicitud para comenzar.
      </p>
    {:else}
      <div class="flex flex-wrap gap-1.5">
        {#each concepts as concept (concept.conceptId)}
          <span
            class="rounded-full border px-2 py-1 text-[9px]
              {concept.state === 'covered'
                ? 'border-emerald-800 bg-emerald-950/40 text-emerald-200'
                : concept.state === 'in_review'
                  ? 'border-amber-800 bg-amber-950/40 text-amber-200'
                  : concept.state === 'partial'
                    ? 'border-sky-800 bg-sky-950/40 text-sky-200'
                    : 'border-surface-700 bg-surface-900 text-surface-400'}"
            title={`${concept.usefulAssets} imágenes útiles · ${concept.pendingCandidates} por revisar`}
          >
            {concept.title} · {concept.usefulAssets}/{Math.max(concept.requestedTarget, 1)}
          </span>
        {/each}
      </div>
    {/if}
  </div></section>
