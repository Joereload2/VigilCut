<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";
  import {
    costLabel,
    formatTimeRange,
    stateColor,
    type CandidateView,
    type NeedSupervision,
    type SupervisionSnapshot,
  } from "./imageGenTypes";

  type FilterId = "all" | "uncovered" | "processing" | "review" | "approved" | "failed";

  let {
    projectKey = $bindable<string | null>(null),
    onPlanUpdated = (_plan: unknown) => {},
    onMessage = (_m: string) => {},
    onError = (_e: string) => {},
  }: {
    projectKey?: string | null;
    onPlanUpdated?: (plan: unknown) => void;
    onMessage?: (m: string) => void;
    onError?: (e: string) => void;
  } = $props();

  let snap = $state<SupervisionSnapshot | null>(null);
  let selectedId = $state<string | null>(null);
  let filter = $state<FilterId>("all");
  let busy = $state(false);
  let showDetails = $state(false);
  let dailyEnabled = $state(false);
  let weekMsg = $state("");
  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const selected = $derived(
    snap?.needs.find((n) => n.need.id === selectedId) ?? snap?.needs[0] ?? null,
  );

  const filtered = $derived.by(() => {
    const list = snap?.needs ?? [];
    switch (filter) {
      case "uncovered":
        return list.filter((n) =>
          ["uncovered", "skipped", "cancelled"].includes(n.uiState),
        );
      case "processing":
        return list.filter((n) => ["queued", "processing"].includes(n.uiState));
      case "review":
        return list.filter((n) => n.uiState === "needs_human_review");
      case "approved":
        return list.filter((n) => n.uiState === "approved");
      case "failed":
        return list.filter((n) => n.uiState === "failed");
      default:
        return list;
    }
  });

  const cov = $derived(snap?.coverage);

  function fileUrl(path?: string | null) {
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      try {
        return convertFileSrc(path.replace(/\\/g, "/"));
      } catch {
        return null;
      }
    }
  }

  async function refresh() {
    if (!projectKey) return;
    try {
      const s = (await api.visualSupervision(projectKey)) as SupervisionSnapshot;
      snap = s;
      dailyEnabled = !!s.dailyFeed?.enabled;
      if (!selectedId && s.needs[0]) selectedId = s.needs[0].need.id;
      if (selectedId && !s.needs.some((n) => n.need.id === selectedId)) {
        selectedId = s.needs[0]?.need.id ?? null;
      }
    } catch (e) {
      onError(String(e));
    }
  }

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
      await refresh();
      onMessage("Necesidades visuales actualizadas");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function generate(n: NeedSupervision) {
    busy = true;
    try {
      const res = (await api.visualGenerateNeed(n.need.id)) as {
        action?: string;
        message?: string;
        snapshot?: SupervisionSnapshot;
      };
      if (res.snapshot) snap = res.snapshot;
      else await refresh();
      onMessage(
        res.action === "reused"
          ? "Imagen reutilizada de la biblioteca"
          : "Generación iniciada",
      );
      // Poll while processing
      startPoll();
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function cancel(jobId: string) {
    busy = true;
    try {
      await api.visualCancelJob(jobId);
      await refresh();
      onMessage("Generación cancelada");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function approve(c: CandidateView, place: boolean) {
    busy = true;
    try {
      const res = (await api.visualApproveAndUse({
        candidateId: c.id,
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        place,
      })) as { message?: string; placementAdded?: boolean };
      // Second click is idempotent
      await refresh();
      onMessage(res.message ?? "Aprobada");
      if (res.placementAdded) {
        const sess = (await api.visualGetSession()) as { plan?: unknown };
        if (sess.plan) onPlanUpdated(sess.plan);
      }
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function reject(c: CandidateView) {
    busy = true;
    try {
      await api.visualRejectCandidate(c.id, "Rechazo humano");
      await refresh();
      onMessage("Imagen rechazada");
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function regenerate(needId: string) {
    busy = true;
    try {
      await api.visualRegenerateNeed(needId);
      await refresh();
      onMessage("Regenerando…");
      startPoll();
    } catch (e) {
      onError(String(e));
    } finally {
      busy = false;
    }
  }

  async function tickWorker() {
    try {
      await api.visualWorkerTick(2);
      await refresh();
    } catch {
      /* ignore */
    }
  }

  function startPoll() {
    if (pollTimer) return;
    pollTimer = setInterval(async () => {
      await tickWorker();
      const still = snap?.needs.some((n) =>
        ["queued", "processing"].includes(n.uiState),
      );
      if (!still && pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    }, 2000);
  }

  async function toggleDaily(v: boolean) {
    try {
      await api.visualDailyFeedSetEnabled(v);
      dailyEnabled = v;
      if (v) {
        const r = (await api.visualDailyFeedCycle()) as { ok?: boolean; reason?: string };
        onMessage(r.ok ? "Alimentación diaria: ciclo ejecutado" : `Daily: ${r.reason ?? "ok"}`);
      }
      const w = (await api.visualDailyWeekSummary()) as { message?: string };
      weekMsg = w.message ?? "";
    } catch (e) {
      onError(String(e));
    }
  }

  $effect(() => {
    if (projectKey) void refresh();
    return () => {
      if (pollTimer) clearInterval(pollTimer);
    };
  });
</script>

<div class="flex min-h-0 min-w-0 flex-col gap-2 overflow-y-auto text-[11px]">
  <!-- Coverage -->
  {#if cov && (cov.total ?? 0) > 0}
    <div class="rounded-lg border border-surface-800 bg-surface-950/70 p-2">
      <p class="font-medium text-surface-100">
        Cobertura visual: {(cov.reused ?? 0) + (cov.generated ?? 0)} de {cov.total}
      </p>
      <ul class="mt-1 flex flex-wrap gap-x-3 gap-y-0.5 text-surface-500">
        {#if (cov.reused ?? 0) > 0}<li>{cov.reused} reutilizadas</li>{/if}
        {#if (cov.generated ?? 0) > 0}<li>{cov.generated} generadas</li>{/if}
        {#if (cov.waiting ?? 0) > 0}<li>{cov.waiting} esperando</li>{/if}
        {#if (cov.needsReview ?? 0) > 0}<li>{cov.needsReview} para revisar</li>{/if}
        {#if (cov.uncovered ?? 0) > 0}<li>{cov.uncovered} sin imagen</li>{/if}
        {#if (cov.failed ?? 0) > 0}<li>{cov.failed} fallidas</li>{/if}
      </ul>
    </div>
  {/if}

  <!-- Toolbar -->
  <div class="flex flex-wrap gap-1">
    <button type="button" class="btn-secondary text-[10px]" disabled={busy} onclick={detect}>
      Detectar necesidades
    </button>
    <button type="button" class="btn-ghost text-[10px]" disabled={busy || !projectKey} onclick={() => refresh()}>
      Actualizar
    </button>
    <button type="button" class="btn-ghost text-[10px]" disabled={busy} onclick={tickWorker}>
      Procesar cola
    </button>
  </div>

  <!-- Filters -->
  <div class="flex flex-wrap gap-1">
    {#each [
      ["all", "Todas"],
      ["uncovered", "Faltantes"],
      ["processing", "Generando"],
      ["review", "Revisar"],
      ["approved", "OK"],
      ["failed", "Fallidas"],
    ] as [id, lab]}
      <button
        type="button"
        class="rounded px-1.5 py-0.5 text-[10px] {filter === id
          ? 'bg-brand-600/30 text-brand-200'
          : 'bg-surface-800 text-surface-400'}"
        onclick={() => (filter = id as FilterId)}>{lab}</button
      >
    {/each}
  </div>

  <!-- Need list -->
  <div class="flex max-h-36 flex-col gap-1 overflow-y-auto">
    {#each filtered as n (n.need.id)}
      <button
        type="button"
        class="flex w-full items-center gap-2 rounded-lg border px-2 py-1.5 text-left transition
          {selectedId === n.need.id ? 'border-brand-500/50 bg-surface-800/80' : 'border-surface-800 bg-surface-950/50 hover:border-surface-600'}"
        onclick={() => (selectedId = n.need.id)}
      >
        <div class="min-w-0 flex-1">
          <p class="truncate font-medium text-surface-200">{n.need.label}</p>
          <p class="text-[10px] text-surface-500">
            {formatTimeRange(n.need.outputStart, n.need.outputEnd)}
            · {n.need.desiredAspect ?? "16:9"}
          </p>
        </div>
        <span class="shrink-0 rounded border px-1.5 py-0.5 text-[9px] {stateColor(n.uiState)}">
          {n.uiLabel}
        </span>
      </button>
    {:else}
      <p class="text-surface-500">
        {projectKey
          ? "No hay necesidades con este filtro."
          : "Detecta necesidades del video para empezar."}
      </p>
    {/each}
  </div>

  <!-- Selected detail -->
  {#if selected}
    {@const n = selected.need}
    {@const c = selected.candidate}
    {@const j = selected.job}
    <div class="rounded-lg border border-surface-800 bg-surface-950/60 p-2 space-y-2">
      <div>
        <p class="text-[10px] uppercase tracking-wide text-surface-500">Imagen solicitada</p>
        <p class="font-medium text-surface-100">{n.label}</p>
      </div>
      {#if (n.terms?.length ?? 0) > 0 || (n.requiredContexts?.length ?? 0) > 0}
        <div>
          <p class="text-[10px] text-surface-500">Debe incluir / representa</p>
          <p class="text-surface-300">
            {[...(n.terms ?? []), ...(n.requiredContexts ?? [])].slice(0, 8).join(" · ")}
          </p>
        </div>
      {/if}
      {#if (n.hardExclusions?.length ?? 0) > 0 || (n.forbiddenContexts?.length ?? 0) > 0}
        <div>
          <p class="text-[10px] text-surface-500">No debe incluir</p>
          <p class="text-surface-400">
            {[...(n.hardExclusions ?? []), ...(n.forbiddenContexts ?? [])].join(" · ")}
          </p>
        </div>
      {/if}
      <p class="text-surface-500">
        Formato {n.desiredAspect ?? "16:9"} · escena {formatTimeRange(n.outputStart, n.outputEnd)}
      </p>

      <!-- Cost badge from job -->
      {#if j}
        <p class="text-[10px] text-surface-400">
          Coste: {costLabel(j.costKind, j.freeVerified)}
          {#if j.provider}· {j.provider}{/if}
          {#if j.model}· {j.model}{/if}
        </p>
        {#if j.stage && ["queued", "processing"].includes(selected.uiState)}
          <p class="text-sky-300/90">Etapa: {j.stage.replaceAll("_", " ")} · intento {j.attempt}/{j.maxAttempts}</p>
        {/if}
        {#if j.lastError}
          <p class="text-red-300/90 break-words">{j.lastError}</p>
        {/if}
      {/if}

      <!-- Actions -->
      <div class="flex flex-wrap gap-1">
        {#if selected.primaryAction === "generate" || selected.uiState === "uncovered" || selected.uiState === "cancelled" || selected.uiState === "skipped"}
          <button type="button" class="btn-primary text-[10px]" disabled={busy} onclick={() => generate(selected)}>
            Generar imagen
          </button>
        {/if}
        {#if selected.uiState === "failed"}
          <button type="button" class="btn-primary text-[10px]" disabled={busy} onclick={() => regenerate(n.id)}>
            Reintentar
          </button>
        {/if}
        {#if (selected.uiState === "queued" || selected.uiState === "processing") && j}
          <button type="button" class="btn-secondary text-[10px]" disabled={busy} onclick={() => cancel(j.id)}>
            Cancelar
          </button>
        {/if}
        {#if selected.uiState === "needs_human_review" || selected.uiState === "rejected"}
          <button type="button" class="btn-secondary text-[10px]" disabled={busy} onclick={() => regenerate(n.id)}>
            Regenerar
          </button>
        {/if}
        <button
          type="button"
          class="btn-ghost text-[10px]"
          disabled={busy}
          onclick={async () => {
            try {
              await api.visualSkipNeed(n.id);
              await refresh();
              onMessage("Continuamos sin imagen para esta escena");
            } catch (e) {
              onError(String(e));
            }
          }}>Continuar sin imagen</button
        >
      </div>

      <!-- Preview -->
      {#if c}
        <div class="space-y-1.5 border-t border-surface-800 pt-2">
          <p class="text-[10px] uppercase text-surface-500">Vista previa</p>
          {#if c.fileExists && c.localPath}
            {@const u = fileUrl(c.localPath)}
            {@const ar = (c.width && c.height && c.height > c.width) ? "9/16" : "16/9"}
            <div
              class="mx-auto w-full max-w-[220px] overflow-hidden rounded-lg border border-surface-700 bg-black/40"
              style="aspect-ratio: {ar}"
            >
              {#if u}
                <img src={u} alt="Candidato" class="h-full w-full object-contain" />
              {:else}
                <p class="p-4 text-center text-surface-500">No se pudo cargar la vista</p>
              {/if}
            </div>
          {:else}
            <p class="text-amber-200/80">Archivo no disponible (eliminado o ruta inválida).</p>
          {/if}
          <div class="grid grid-cols-2 gap-x-2 text-[10px] text-surface-400">
            <span>{c.width ?? "?"}×{c.height ?? "?"}</span>
            <span>{c.mimeType ?? "—"}</span>
            <span>{costLabel(c.costKind, c.freeVerified)}</span>
            <span>QA: {c.qaDecision ?? c.status}</span>
            {#if c.semanticScore != null}
              <span>Semántica: {(c.semanticScore * 100).toFixed(0)}%</span>
            {/if}
            {#if c.technicalScore != null}
              <span>Técnica: {(c.technicalScore * 100).toFixed(0)}%</span>
            {/if}
          </div>
          {#if c.qaReason}
            <p class="text-surface-400">{c.qaReason}</p>
          {/if}
          <div class="flex flex-wrap gap-1">
            {#if c.status === "needs_human_review" || c.status === "automated_review" || c.status === "generated"}
              <button type="button" class="btn-primary text-[10px]" disabled={busy} onclick={() => approve(c, true)}>
                Aprobar y usar
              </button>
              <button type="button" class="btn-secondary text-[10px]" disabled={busy} onclick={() => approve(c, false)}>
                Solo biblioteca
              </button>
              <button type="button" class="btn-ghost text-[10px]" disabled={busy} onclick={() => reject(c)}>
                Rechazar
              </button>
              <button type="button" class="btn-ghost text-[10px]" disabled={busy} onclick={() => regenerate(n.id)}>
                Regenerar
              </button>
            {:else if c.status === "approved"}
              <span class="text-emerald-400">En biblioteca ✓ {c.approvedAssetId ? "· asset listo" : ""}</span>
            {/if}
          </div>
        </div>
      {/if}

      <!-- Details -->
      <button type="button" class="text-[10px] text-brand-300 underline" onclick={() => (showDetails = !showDetails)}>
        {showDetails ? "Ocultar detalles" : "Ver detalles"}
      </button>
      {#if showDetails && (j || c)}
        <div class="space-y-1 rounded bg-surface-900/80 p-2 font-mono text-[9px] text-surface-400 break-all">
          {#if j}
            <p>job: {j.id}</p>
            <p>prompt: {j.prompt}</p>
            <p>negative: {j.negativePrompt}</p>
            <p>strategy: {j.promptStrategy ?? "—"}</p>
            <p>intentos: {j.attempt}/{j.maxAttempts}</p>
            <p>origen: {j.origin}</p>
          {/if}
          {#if c}
            <p>candidate: {c.id}</p>
            <p>path: {c.localPath}</p>
            <p>origen: {c.origin}</p>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Daily feed compact -->
  <div class="rounded-lg border border-surface-800/80 bg-surface-950/40 p-2">
    <label class="flex cursor-pointer items-center gap-2 text-surface-300">
      <input
        type="checkbox"
        class="rounded"
        checked={dailyEnabled}
        onchange={(e) => toggleDaily((e.currentTarget as HTMLInputElement).checked)}
      />
      Alimentación diaria de la biblioteca (solo gratis/local)
    </label>
    {#if weekMsg}
      <p class="mt-1 text-[10px] text-surface-500">{weekMsg}</p>
    {/if}
  </div>
</div>
