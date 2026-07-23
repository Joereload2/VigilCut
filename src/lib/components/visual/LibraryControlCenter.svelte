<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import * as api from "$lib/utils/tauri";

  type Dashboard = {
    inventory: { activeAssets: number; pendingCandidates: number; managedBytes: number };
    activity: { queued: number; running: number; cancelling: number; failedToday: number };
    provider: { name: string; configured: boolean; reachable: boolean; supportsImage: boolean; freeVerified: boolean; costKind: string };
    limits: { localDailyLimit: number; localUsedToday: number; localRemainingToday: number };
    canWork: boolean; blockedReason?: string | null;
  };
  type RequestRow = {
    id: string; origin: string; theme: string; title: string; description: string;
    prompt: string; negativePrompt: string; desiredFormat: string; width: number; height: number;
    style: string; status: string; queued: number; running: number; awaitingReview: number;
    failed: number; updatedAt: string;
  };
  type Match = { assetId: string; title: string; thumbnailPath?: string | null; score: number; reasons: string[] };
  type Preview = { request: RequestRow; matches: Match[]; canConfirm: boolean; maxEnqueueable: number; blockedReason?: string | null; provider: string };
  let { onReview = () => {}, onUseExisting = (_id: string) => {} }: { onReview?: () => void; onUseExisting?: (id: string) => void } = $props();

  let dashboard = $state<Dashboard | null>(null);
  let requests = $state<RequestRow[]>([]);
  let loading = $state(true);
  let busy = $state(false);
  let error = $state("");
  let notice = $state("");
  let formOpen = $state(false);
  let advanced = $state(false);
  let preview = $state<Preview | null>(null);
  let theme = $state("");
  let concept = $state("");
  let description = $state("");
  let prompt = $state("");
  let negativePrompt = $state("");
  let intent = $state("");
  let format = $state<"16:9" | "9:16" | "1:1" | "4:5">("16:9");
  let style = $state<"photorealistic" | "illustration" | "infographic" | "cinematic" | "other">("photorealistic");

  const activityCount = $derived((dashboard?.activity.queued ?? 0) + (dashboard?.activity.running ?? 0) + (dashboard?.activity.cancelling ?? 0));
  const providerLabel = $derived.by(() => {
    const p = dashboard?.provider;
    if (!p) return "Comprobando proveedor";
    if (p.name === "mock") return "Simulación local (mock) — no es IA real";
    if (!p.configured) return "OmniRoute no configurado";
    if (!p.reachable) return "OmniRoute no disponible";
    if (!p.supportsImage) return "Generación de imágenes no verificada";
    return p.freeVerified ? "OmniRoute · gratuidad verificada" : "OmniRoute · coste no verificado";
  });

  function imageUrl(path?: string | null) {
    if (!path) return null;
    try { return convertFileSrc(path); } catch { return null; }
  }

  function resetForm() {
    intent = ""; theme = ""; concept = ""; description = ""; prompt = ""; negativePrompt = "";
    format = "16:9"; style = "photorealistic"; preview = null; advanced = false;
  }

  async function refresh() {
    loading = true; error = "";
    try {
      const [nextDashboard, nextRequests] = await Promise.all([
        api.visualLibraryDashboard(), api.visualLibraryListRequests(30),
      ]);
      dashboard = nextDashboard as Dashboard;
      requests = Array.isArray(nextRequests) ? nextRequests as RequestRow[] : [];
    } catch (e) { error = String(e); } finally { loading = false; }
  }

  async function searchFirst() {
    if (intent.trim().length < 2) {
      error = "Escribe qué necesitas, por ejemplo: dinero, economía o personas trabajando."; return;
    }
    concept = concept.trim() || intent.trim();
    description = description.trim() || `Imagen relacionada con ${intent.trim()}`;
    busy = true; error = ""; notice = "";
    try {
      preview = await api.visualLibraryCreateRequest({
        origin: "manual", theme: theme.trim(), title: concept.trim(), description: description.trim(),
        prompt: prompt.trim(), negativePrompt: negativePrompt.trim(), targetCount: 1,
        desiredFormat: format, width: 0, height: 0, style,
        positiveContexts: [], negativeContexts: [], hardExclusions: [], priority: 70,
      }) as Preview;
      prompt = preview.request.prompt;
      negativePrompt = preview.request.negativePrompt;
      await refresh();
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  async function useExisting(assetId: string) {
    if (!preview) return;
    busy = true; error = "";
    try {
      await api.visualLibraryUseExisting(preview.request.id, assetId);
      notice = "Imagen existente seleccionada. No se creó ninguna generación.";
      onUseExisting(assetId); formOpen = false; resetForm(); await refresh();
    } catch (e) { error = String(e); } finally { busy = false; }
  }
  async function generate() {
    if (!preview) return;
    busy = true; error = "";
    try {
      if (prompt.trim() !== preview.request.prompt || negativePrompt.trim() !== preview.request.negativePrompt) {
        await api.visualLibraryRegenerateRequest(preview.request.id, prompt, negativePrompt);
      } else {
        await api.visualLibraryConfirmRequest(preview.request.id);
      }
      notice = dashboard?.provider.name === "mock"
        ? "Simulación en cola. El resultado deberá aprobarse antes de entrar a la Biblioteca."
        : "Generación en cola. El resultado aparecerá en Por revisar.";
      formOpen = false; resetForm(); await refresh();
    } catch (e) { error = String(e); } finally { busy = false; }
  }

  async function cancelRequest(requestId: string) {
    busy = true; error = "";
    try { await api.visualLibraryCancelRequest(requestId); notice = "Cancelación solicitada."; await refresh(); }
    catch (e) { error = String(e); } finally { busy = false; }
  }

  onMount(() => {
    void refresh();
    const timer = window.setInterval(() => { if (activityCount > 0) void refresh(); }, 2500);
    return () => window.clearInterval(timer);
  });
</script>

<section class="space-y-3" aria-labelledby="library-control-title">
  <div class="flex flex-wrap items-center justify-between gap-2">
    <div>
      <h2 id="library-control-title" class="text-sm font-semibold text-white">Crear y revisar imágenes</h2>
      <p class="text-[11px] text-surface-400">Busca primero en tu Biblioteca y genera solo si necesitas una variante nueva.</p>
    </div>
    <button type="button" class="btn-primary px-4 py-2 text-xs" onclick={() => { resetForm(); formOpen = true; }}>
      + Nueva imagen
    </button>
  </div>

  {#if dashboard}
    <div class="grid gap-2 sm:grid-cols-3">
      <div class="rounded-xl border border-surface-800 bg-surface-950/50 p-2.5"><p class="text-[10px] text-surface-500">Biblioteca</p><p class="text-lg font-semibold text-white">{dashboard.inventory.activeAssets} imágenes</p></div>
      <button type="button" class="rounded-xl border border-amber-900/60 bg-amber-950/20 p-2.5 text-left" onclick={onReview}><p class="text-[10px] text-amber-300">Por revisar</p><p class="text-lg font-semibold text-white">{dashboard.inventory.pendingCandidates}</p></button>
      <div class="rounded-xl border border-surface-800 bg-surface-950/50 p-2.5"><p class="text-[10px] text-surface-500">Actividad</p><p class="text-sm font-semibold text-white">{activityCount ? `${activityCount} en curso` : "En reposo"}</p><p class="text-[9px] text-surface-500">{dashboard.limits.localRemainingToday} generaciones disponibles hoy</p></div>
    </div>
    <div class="rounded-lg border {dashboard.canWork ? 'border-emerald-900/60 bg-emerald-950/20' : 'border-amber-900/60 bg-amber-950/20'} px-3 py-2">
      <p class="text-[11px] font-medium text-surface-100">{providerLabel}</p>
      {#if dashboard.blockedReason}<p class="text-[10px] text-amber-300">{dashboard.blockedReason}</p>{/if}
    </div>
  {/if}

  {#if formOpen}
    <div class="rounded-2xl border border-violet-700/60 bg-surface-950/95 p-4 shadow-xl" role="dialog" aria-modal="true" aria-labelledby="new-image-title" tabindex="-1">
      <div class="flex items-start justify-between gap-3"><div><h3 id="new-image-title" class="text-base font-semibold text-white">Nueva imagen</h3><p class="text-[11px] text-surface-400">Una imagen · revisión humana obligatoria</p></div><button type="button" class="btn-ghost" onclick={() => { formOpen = false; resetForm(); }}>Cerrar</button></div>
      <form class="mt-4" onsubmit={(e) => { e.preventDefault(); void searchFirst(); }}>
        <label class="block text-xs font-medium text-surface-200">¿Qué imagen necesitas?
          <input class="mt-2 w-full rounded-xl border border-surface-700 bg-surface-900 px-4 py-3 text-base" bind:value={intent} placeholder="dinero" required />
        </label>
        <p class="mt-2 text-[11px] text-surface-500">Escribe una idea. VigilCut buscará conceptos relacionados antes de generar.</p>
        <button type="button" class="mt-3 text-left text-[11px] text-violet-300 underline" onclick={() => (advanced = !advanced)}>{advanced ? "Ocultar detalles" : "Añadir detalles opcionales"}</button>
        {#if advanced}
          <div class="mt-3 grid gap-3 rounded-xl border border-surface-800 bg-surface-900/60 p-3 sm:grid-cols-2">
            <label class="text-[11px] text-surface-300">Tema<input class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-950 px-3 py-2" bind:value={theme} placeholder="Se infiere automáticamente" /></label>
            <label class="text-[11px] text-surface-300">Título o concepto<input class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-950 px-3 py-2" bind:value={concept} placeholder="Opcional" /></label>
            <label class="text-[11px] text-surface-300 sm:col-span-2">Descripción<textarea class="mt-1 min-h-16 w-full rounded-lg border border-surface-700 bg-surface-950 px-3 py-2" bind:value={description} placeholder="Opcional: se genera a partir de tu idea"></textarea></label>
            <label class="text-[11px] text-surface-300 sm:col-span-2">No debe contener<textarea class="mt-1 min-h-14 w-full rounded-lg border border-surface-700 bg-surface-950 px-3 py-2" bind:value={negativePrompt} placeholder="logos, texto ilegible..."></textarea></label>
            <label class="text-[11px] text-surface-300">Formato<select class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-950 px-3 py-2" bind:value={format}><option value="16:9">Horizontal 16:9</option><option value="9:16">Vertical 9:16</option><option value="1:1">Cuadrado 1:1</option><option value="4:5">Retrato 4:5</option></select></label>
            <label class="text-[11px] text-surface-300">Estilo<select class="mt-1 w-full rounded-lg border border-surface-700 bg-surface-950 px-3 py-2" bind:value={style}><option value="photorealistic">Fotografía realista</option><option value="illustration">Ilustración</option><option value="infographic">Infografía sin texto</option><option value="cinematic">Cinematográfico</option><option value="other">Otro</option></select></label>
          </div>
        {/if}
        <div class="mt-4 flex justify-end gap-2"><button type="button" class="btn-ghost" onclick={() => { formOpen = false; resetForm(); }}>Cancelar</button><button type="submit" class="btn-primary px-5" disabled={busy}>{busy ? "Buscando…" : "Buscar imágenes"}</button></div>
      </form>

      {#if preview}
        <div class="mt-4 border-t border-surface-800 pt-3">
          <h4 class="text-sm font-semibold text-white">Coincidencias encontradas</h4>
          {#if preview.matches.length === 0}<p class="mt-2 rounded-lg border border-dashed border-surface-700 p-3 text-[11px] text-surface-400">No encontramos una imagen adecuada. Puedes continuar con la generación.</p>{:else}<div class="mt-2 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">{#each preview.matches as match (match.assetId)}{@const src = imageUrl(match.thumbnailPath)}<article class="overflow-hidden rounded-xl border border-surface-800 bg-surface-900">{#if src}<img src={src} alt={match.title} class="aspect-video w-full object-cover" />{/if}<div class="p-2"><p class="truncate text-[11px] font-medium text-white">{match.title}</p><p class="text-[9px] text-surface-400">Coincidencia {Math.round(match.score * 100)}%</p><button type="button" class="btn-secondary mt-2 text-[10px]" onclick={() => void useExisting(match.assetId)}>Usar esta imagen</button></div></article>{/each}</div>{/if}
          <div class="mt-3 rounded-lg border border-surface-800 bg-surface-900/70 p-3"><label class="text-[10px] text-surface-400">Prompt positivo<textarea class="mt-1 min-h-20 w-full rounded border border-surface-700 bg-surface-950 p-2 text-[11px]" bind:value={prompt}></textarea></label><label class="mt-2 block text-[10px] text-surface-400">Prompt negativo<textarea class="mt-1 min-h-14 w-full rounded border border-surface-700 bg-surface-950 p-2 text-[11px]" bind:value={negativePrompt}></textarea></label><div class="mt-2 flex justify-end"><button type="button" class="btn-primary" disabled={busy || !preview.canConfirm} onclick={() => void generate()}>{dashboard?.provider.name === "mock" ? "Continuar con simulación" : "Continuar y generar"}</button></div>{#if preview.blockedReason}<p class="mt-2 text-[10px] text-amber-300">{preview.blockedReason}</p>{/if}</div>
        </div>
      {/if}
    </div>
  {/if}

  {#if error}<p class="rounded-lg border border-red-800 bg-red-950/30 p-2 text-[11px] text-red-200" role="alert">{error}</p>{:else if notice}<p class="rounded-lg border border-emerald-800 bg-emerald-950/30 p-2 text-[11px] text-emerald-200" role="status">{notice}</p>{/if}

  {#if requests.length > 0}<div><div class="mb-1 flex items-center justify-between"><h3 class="text-[11px] font-semibold text-surface-200">Solicitudes recientes</h3>{#if (dashboard?.inventory.pendingCandidates ?? 0) > 0}<button type="button" class="text-[10px] text-amber-300 underline" onclick={onReview}>Abrir Por revisar</button>{/if}</div><div class="space-y-1.5">{#each requests as request (request.id)}<article class="flex flex-wrap items-center gap-2 rounded-lg border border-surface-800 bg-surface-950/40 p-2.5"><div class="min-w-44 flex-1"><p class="text-[11px] font-medium text-white">{request.title}</p><p class="text-[9px] text-surface-500">{request.theme || "Sin tema"} · {request.desiredFormat} · {request.width}×{request.height}</p></div><p class="text-[10px] text-surface-300">{request.running ? "Generando" : request.queued ? "En cola" : request.awaitingReview ? "Por revisar" : request.failed ? "Fallida" : request.status}</p>{#if request.queued + request.running > 0}<button type="button" class="btn-ghost text-[9px]" disabled={busy} onclick={() => void cancelRequest(request.id)}>Cancelar</button>{/if}</article>{/each}</div></div>{:else if !loading}<div class="rounded-xl border border-dashed border-surface-700 p-5 text-center"><p class="text-sm text-surface-300">Aún no hay solicitudes.</p><p class="mt-1 text-[11px] text-surface-500">Crea una imagen sin abrir ningún video.</p></div>{/if}
</section>
