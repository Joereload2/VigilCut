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
  let busyNeedId = $state<string | null>(null);
  let globalBusy = $state(false);
  let showDetails = $state(false);
  let dailyOpen = $state(false);
  let dailyEnabled = $state(false);
  let weekMsg = $state("");
  let imgLoading = $state(false);
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let rejectOpen = $state(false);
  let rejectReason = $state("");

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

  const dailyPending = $derived(
    (snap?.pendingReview ?? []).filter((c) => c.origin === "daily_feed"),
  );

  const cov = $derived(snap?.coverage);
  const isEmpty = $derived(!projectKey || (snap?.needs.length ?? 0) === 0);

  function stageLabel(stage?: string | null): string {
    const m: Record<string, string> = {
      queued: "En cola",
      preparing: "Preparando solicitud",
      waiting_provider: "Esperando proveedor",
      generating: "Generando imagen",
      downloading: "Descargando",
      file_review: "Revisando archivo",
      evaluating: "Evaluando imagen",
      cancelled: "Cancelada",
    };
    return m[stage ?? ""] ?? (stage ? stage.replaceAll("_", " ") : "—");
  }

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

  function isBusy(needId?: string) {
    if (globalBusy) return true;
    if (needId && busyNeedId === needId) return true;
    return false;
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

  async function resumeQueue() {
    try {
      await api.visualWorkerTick(3);
      await refresh();
      const still = snap?.needs.some((n) =>
        ["queued", "processing"].includes(n.uiState),
      );
      if (still) startPoll();
    } catch {
      /* offline / empty */
    }
  }

  async function detect() {
    if (!projectStore.mediaPath) {
      onError("Abre un video primero");
      return;
    }
    globalBusy = true;
    try {
      const res = (await api.visualDetectNeeds({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
      })) as { projectKey?: string };
      projectKey = res.projectKey ?? projectKey;
      await refresh();
      onMessage("Necesidades visuales actualizadas (se conservan las que ya tienen trabajo)");
    } catch (e) {
      onError(String(e));
    } finally {
      globalBusy = false;
    }
  }

  async function searchLib(n: NeedSupervision) {
    busyNeedId = n.need.id;
    try {
      const res = (await api.visualSearchLibraryForNeed(n.need.id)) as {
        matched?: boolean;
        message?: string;
      };
      await refresh();
      onMessage(res.message ?? (res.matched ? "Encontrada" : "Sin coincidencia"));
    } catch (e) {
      onError(String(e));
    } finally {
      busyNeedId = null;
    }
  }

  async function generate(n: NeedSupervision) {
    busyNeedId = n.need.id;
    try {
      const res = (await api.visualGenerateNeed(n.need.id)) as {
        action?: string;
        snapshot?: SupervisionSnapshot;
      };
      if (res.snapshot) snap = res.snapshot;
      else await refresh();
      onMessage(
        res.action === "reused"
          ? "Imagen reutilizada de la biblioteca"
          : "Generación iniciada — revisa el resultado cuando esté listo",
      );
      startPoll();
    } catch (e) {
      onError(String(e));
    } finally {
      busyNeedId = null;
    }
  }

  async function cancel(jobId: string, needId?: string) {
    busyNeedId = needId ?? null;
    try {
      await api.visualCancelJob(jobId);
      await refresh();
      onMessage("Generación cancelada");
    } catch (e) {
      onError(String(e));
    } finally {
      busyNeedId = null;
    }
  }

  async function approve(c: CandidateView, place: boolean) {
    busyNeedId = c.needId ?? null;
    try {
      const res = (await api.visualApproveAndUse({
        candidateId: c.id,
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        place,
      })) as { message?: string; placementAdded?: boolean };
      await refresh();
      onMessage(res.message ?? "Aprobada");
      if (res.placementAdded) {
        const sess = (await api.visualGetSession()) as { plan?: unknown };
        if (sess.plan) {
          onPlanUpdated(sess.plan);
          onMessage("En el plan ✓ — revisa la línea de tiempo");
        }
      }
    } catch (e) {
      onError(String(e));
    } finally {
      busyNeedId = null;
    }
  }

  async function reject(c: CandidateView) {
    busyNeedId = c.needId ?? null;
    try {
      await api.visualRejectCandidate(
        c.id,
        rejectReason.trim() || "Rechazo humano",
      );
      rejectOpen = false;
      rejectReason = "";
      await refresh();
      onMessage("Imagen rechazada");
    } catch (e) {
      onError(String(e));
    } finally {
      busyNeedId = null;
    }
  }

  async function regenerate(needId: string) {
    busyNeedId = needId;
    try {
      await api.visualRegenerateNeed(needId);
      await refresh();
      onMessage("Regenerando…");
      startPoll();
    } catch (e) {
      onError(String(e));
    } finally {
      busyNeedId = null;
    }
  }

  function startPoll() {
    if (pollTimer) return;
    pollTimer = setInterval(async () => {
      try {
        await api.visualWorkerTick(2);
        await refresh();
      } catch {
        /* ignore */
      }
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
        startPoll();
      }
      const w = (await api.visualDailyWeekSummary()) as { message?: string };
      weekMsg = w.message ?? "";
    } catch (e) {
      onError(String(e));
    }
  }

  $effect(() => {
    if (projectKey) {
      void (async () => {
        await refresh();
        await resumeQueue();
      })();
    }
    return () => {
      if (pollTimer) clearInterval(pollTimer);
      pollTimer = null;
    };
  });
</script>

<div class="flex min-h-0 min-w-0 flex-col gap-2 overflow-y-auto text-[11px]">
  {#if isEmpty}
    <div class="rounded-lg border border-dashed border-surface-700 bg-surface-950/50 p-3 text-surface-300">
      <p class="font-medium text-surface-100">Supervisión de imágenes</p>
      <ol class="mt-2 list-decimal space-y-1 pl-4 text-surface-400">
        <li>Detecta qué necesita el video a partir del texto.</li>
        <li>Revisa la lista y genera solo lo que falte.</li>
        <li>Aprueba o rechaza lo generado (la IA no publica sola).</li>
      </ol>
      <button
        type="button"
        class="btn-primary mt-3 text-[10px]"
        disabled={globalBusy || !projectStore.mediaPath}
        onclick={detect}
      >
        1 · Detectar necesidades
      </button>
      {#if !projectStore.mediaPath}
        <p class="mt-2 text-[10px] text-amber-200/80">Abre un video y analiza silencios primero.</p>
      {/if}
    </div>
  {/if}

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

  <div class="flex flex-wrap gap-1">
    <button type="button" class="btn-secondary text-[10px]" disabled={globalBusy} onclick={detect}>
      Detectar necesidades
    </button>
    <button
      type="button"
      class="btn-ghost text-[10px]"
      disabled={globalBusy || !projectKey}
      onclick={() => refresh()}
    >
      Actualizar
    </button>
  </div>

  {#if (snap?.needs.length ?? 0) > 0}
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

    <div class="flex max-h-36 flex-col gap-1 overflow-y-auto">
      {#each filtered as n (n.need.id)}
        <button
          type="button"
          class="flex w-full items-center gap-2 rounded-lg border px-2 py-1.5 text-left transition
            {selectedId === n.need.id
              ? 'border-brand-500/50 bg-surface-800/80'
              : 'border-surface-800 bg-surface-950/50 hover:border-surface-600'}"
          onclick={() => (selectedId = n.need.id)}
        >
          {#if n.candidate?.fileExists && n.candidate.localPath}
            {@const u = fileUrl(n.candidate.localPath)}
            {#if u}
              <img src={u} alt="" class="h-9 w-9 shrink-0 rounded object-cover" />
            {/if}
          {:else}
            <div class="flex h-9 w-9 shrink-0 items-center justify-center rounded bg-surface-800 text-[9px] text-surface-500">
              —
            </div>
          {/if}
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
        <p class="text-surface-500">No hay necesidades con este filtro.</p>
      {/each}
    </div>
  {/if}

  {#if selected}
    {@const n = selected.need}
    {@const c = selected.candidate}
    {@const j = selected.job}
    <div class="space-y-2 rounded-lg border border-surface-800 bg-surface-950/60 p-2">
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

      {#if j}
        <p class="text-[10px] text-surface-400">
          Coste: {costLabel(j.costKind, j.freeVerified)}
          {#if j.provider}· {j.provider}{/if}
        </p>
        {#if ["queued", "processing"].includes(selected.uiState)}
          <p class="text-sky-300/90">
            {stageLabel(j.stage)} · intento {j.attempt}/{j.maxAttempts}
          </p>
        {/if}
        {#if j.lastError}
          <p class="break-words text-red-300/90">{j.lastError}</p>
        {/if}
      {/if}

      <div class="flex flex-wrap gap-1">
        {#if ["uncovered", "cancelled", "skipped"].includes(selected.uiState)}
          <button
            type="button"
            class="btn-secondary text-[10px]"
            disabled={isBusy(n.id)}
            onclick={() => searchLib(selected)}
          >
            Buscar en biblioteca
          </button>
          <button
            type="button"
            class="btn-primary text-[10px]"
            disabled={isBusy(n.id)}
            onclick={() => generate(selected)}
          >
            Generar imagen
          </button>
        {/if}
        {#if selected.uiState === "failed"}
          <button
            type="button"
            class="btn-primary text-[10px]"
            disabled={isBusy(n.id)}
            onclick={() => regenerate(n.id)}
          >
            Reintentar
          </button>
        {/if}
        {#if (selected.uiState === "queued" || selected.uiState === "processing") && j}
          <button
            type="button"
            class="btn-secondary text-[10px]"
            disabled={isBusy(n.id)}
            onclick={() => cancel(j.id, n.id)}
          >
            Cancelar
          </button>
        {/if}
        {#if selected.uiState === "needs_human_review" || selected.uiState === "rejected"}
          <button
            type="button"
            class="btn-secondary text-[10px]"
            disabled={isBusy(n.id)}
            onclick={() => regenerate(n.id)}
          >
            Regenerar
          </button>
        {/if}
        <button
          type="button"
          class="btn-ghost text-[10px]"
          disabled={isBusy(n.id)}
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

      {#if c}
        <div class="space-y-1.5 border-t border-surface-800 pt-2">
          <p class="text-[10px] uppercase text-surface-500">Vista previa</p>
          {#if c.fileExists && c.localPath}
            {@const u = fileUrl(c.localPath)}
            {@const ar = c.width && c.height && c.height > c.width ? "9/16" : "16/9"}
            <div
              class="relative mx-auto w-full max-w-[220px] overflow-hidden rounded-lg border border-surface-700 bg-black/40"
              style="aspect-ratio: {ar}"
            >
              {#if imgLoading}
                <div class="absolute inset-0 flex items-center justify-center text-surface-500">
                  Cargando…
                </div>
              {/if}
              {#if u}
                <img
                  src={u}
                  alt="Candidato"
                  class="h-full w-full object-contain"
                  onload={() => (imgLoading = false)}
                  onerror={() => (imgLoading = false)}
                  onloadstart={() => (imgLoading = true)}
                />
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
            <span class="col-span-2">Origen: {c.origin === "daily_feed" ? "Alimentación diaria" : "Video"}</span>
          </div>
          {#if c.qaReason}
            <p class="text-surface-400">{c.qaReason}</p>
          {/if}
          <div class="flex flex-wrap gap-1">
            {#if c.status === "needs_human_review" || c.status === "automated_review" || c.status === "generated"}
              <button
                type="button"
                class="btn-primary text-[10px]"
                disabled={isBusy(n.id)}
                onclick={() => approve(c, true)}
              >
                Aprobar y usar
              </button>
              <button
                type="button"
                class="btn-secondary text-[10px]"
                disabled={isBusy(n.id)}
                onclick={() => approve(c, false)}
              >
                Solo biblioteca
              </button>
              <button
                type="button"
                class="btn-ghost text-[10px]"
                disabled={isBusy(n.id)}
                onclick={() => {
                  rejectOpen = true;
                }}
              >
                Rechazar
              </button>
              <button
                type="button"
                class="btn-ghost text-[10px]"
                disabled={isBusy(n.id)}
                onclick={() => regenerate(n.id)}
              >
                Regenerar
              </button>
            {:else if c.status === "approved"}
              <span class="text-emerald-400">En biblioteca ✓</span>
            {/if}
          </div>
          {#if rejectOpen}
            <div class="space-y-1 rounded border border-surface-700 bg-surface-900 p-2">
              <p class="text-[10px] text-surface-400">Motivo (opcional)</p>
              <div class="flex flex-wrap gap-1">
                {#each ["No representa el concepto", "Marca o texto", "Mala calidad", "Contexto incorrecto"] as chip}
                  <button
                    type="button"
                    class="rounded bg-surface-800 px-1.5 py-0.5 text-[9px] text-surface-300 hover:bg-surface-700"
                    onclick={() => (rejectReason = chip)}>{chip}</button
                  >
                {/each}
              </div>
              <input
                class="w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 text-[10px]"
                placeholder="Otro motivo…"
                bind:value={rejectReason}
              />
              <div class="flex gap-1">
                <button type="button" class="btn-primary text-[10px]" onclick={() => reject(c)}
                  >Confirmar rechazo</button
                >
                <button type="button" class="btn-ghost text-[10px]" onclick={() => (rejectOpen = false)}
                  >Cerrar</button
                >
              </div>
            </div>
          {/if}
        </div>
      {/if}

      <button
        type="button"
        class="text-[10px] text-brand-300 underline"
        onclick={() => (showDetails = !showDetails)}
      >
        {showDetails ? "Ocultar detalles" : "Ver detalles"}
      </button>
      {#if showDetails && (j || c)}
        <div
          class="space-y-1 break-all rounded bg-surface-900/80 p-2 font-mono text-[9px] text-surface-400"
        >
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
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <!-- Daily feed collapsed -->
  <div class="rounded-lg border border-surface-800/80 bg-surface-950/40 p-2">
    <button
      type="button"
      class="flex w-full items-center justify-between text-left text-surface-300"
      onclick={() => (dailyOpen = !dailyOpen)}
    >
      <span>Biblioteca automática (daily)</span>
      <span class="text-surface-500">{dailyOpen ? "▾" : "▸"}</span>
    </button>
    {#if dailyOpen}
      <label class="mt-2 flex cursor-pointer items-center gap-2 text-surface-300">
        <input
          type="checkbox"
          class="rounded"
          checked={dailyEnabled}
          onchange={(e) => toggleDaily((e.currentTarget as HTMLInputElement).checked)}
        />
        Activar cuando la app está abierta (solo gratis/local)
      </label>
      {#if weekMsg}
        <p class="mt-1 text-[10px] text-surface-500">{weekMsg}</p>
      {/if}
      {#if dailyPending.length > 0}
        <p class="mt-2 text-[10px] font-medium text-amber-200">
          {dailyPending.length} candidato(s) diarios por revisar
        </p>
        {#each dailyPending.slice(0, 3) as dc (dc.id)}
          <div class="mt-1 flex items-center gap-2 rounded border border-surface-800 p-1">
            {#if dc.fileExists && dc.localPath}
              {@const u = fileUrl(dc.localPath)}
              {#if u}<img src={u} alt="" class="h-8 w-8 rounded object-cover" />{/if}
            {/if}
            <span class="min-w-0 flex-1 truncate text-[10px] text-surface-400">{dc.id.slice(0, 8)}…</span>
            <button
              type="button"
              class="btn-primary px-1 py-0 text-[9px]"
              onclick={() => approve(dc, false)}>OK</button
            >
            <button
              type="button"
              class="btn-ghost px-1 py-0 text-[9px]"
              onclick={() => reject(dc)}>No</button
            >
          </div>
        {/each}
      {/if}
    {/if}
  </div>
</div>
