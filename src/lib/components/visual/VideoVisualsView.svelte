<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { CoverageSummary, NeedSupervision } from "./imageGenTypes";
  import { formatTimeRange } from "./imageGenTypes";

  let {
    needs = [] as NeedSupervision[],
    coverage = null as CoverageSummary | null,
    busy = false,
    onDetect,
    onPrimary,
    onCancel,
  }: {
    needs?: NeedSupervision[];
    coverage?: CoverageSummary | null;
    busy?: boolean;
    onDetect: () => void;
    onPrimary: (n: NeedSupervision) => void;
    onCancel: (jobId: string) => void;
  } = $props();

  const total = $derived(coverage?.total ?? needs.length);
  const covered = $derived(
    (coverage?.reused ?? 0) + (coverage?.generated ?? 0) +
      needs.filter((n) => ["approved", "matched"].includes(n.uiState) || n.need.coverage === "covered" || n.need.coverage === "matched").length,
  );
  // Prefer backend coverage when present
  const coveredN = $derived(
    coverage && (coverage.total ?? 0) > 0
      ? (coverage.reused ?? 0) + (coverage.generated ?? 0)
      : needs.filter((n) =>
          ["approved"].includes(n.uiState) ||
          n.need.coverage === "covered" ||
          n.need.coverage === "matched",
        ).length,
  );
  const pct = $derived(total > 0 ? Math.round((coveredN / total) * 100) : 0);

  function actionLabel(n: NeedSupervision): string {
    switch (n.uiState) {
      case "queued":
      case "processing":
      case "cancelling":
        return n.uiState === "cancelling" ? "…" : "Cancelar";
      case "needs_human_review":
      case "reviewing":
        return "Revisar";
      case "approved":
        return "Cambiar";
      case "failed":
        return "Reintentar";
      default:
        if (n.need.coverage === "matched" || n.need.coverage === "covered") return "Cambiar";
        return "Buscar imagen";
    }
  }

  function stateText(n: NeedSupervision): string {
    if (n.uiLabel) return n.uiLabel;
    switch (n.uiState) {
      case "queued":
        return "En cola";
      case "processing":
        return "Generando…";
      case "cancelling":
        return "Cancelando…";
      case "needs_human_review":
        return "Revisión pendiente";
      case "approved":
        return "Lista";
      case "failed":
        return "No se pudo generar";
      case "cancelled":
      case "skipped":
        return "Sin imagen";
      default:
        return n.need.coverage === "matched" || n.need.coverage === "covered"
          ? "Lista"
          : "Sin imagen";
    }
  }

  function thumb(n: NeedSupervision) {
    const p = n.candidate?.localPath;
    if (!p || !n.candidate?.fileExists) return null;
    if (!["approved", "needs_human_review"].includes(n.uiState) && n.need.coverage !== "covered")
      return null;
    try {
      return convertFileSrc(p);
    } catch {
      return null;
    }
  }
</script>

<div class="flex min-h-0 flex-col gap-2 overflow-y-auto text-[11px]">
  <div class="rounded-lg border border-surface-800 bg-surface-950/60 p-2">
    <div class="flex items-center justify-between gap-2">
      <p class="font-medium text-surface-100">
        {coveredN} de {total || "—"} momentos visuales están cubiertos
      </p>
      <button type="button" class="btn-ghost shrink-0 text-[10px]" disabled={busy} onclick={onDetect}>
        Detectar nuevamente
      </button>
    </div>
    <div class="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-800">
      <div class="h-full rounded-full bg-sky-500 transition-all" style="width: {pct}%"></div>
    </div>
  </div>

  {#if needs.length === 0}
    <div class="rounded-lg border border-dashed border-surface-700 p-3 text-surface-400">
      <p class="font-medium text-surface-200">Aún no hay momentos detectados</p>
      <p class="mt-1 text-[10px]">
        Analiza silencios y pulsa Detectar nuevamente para ver qué imágenes necesita el video.
      </p>
      <button type="button" class="btn-primary mt-2 text-[10px]" disabled={busy} onclick={onDetect}>
        Detectar momentos
      </button>
    </div>
  {:else}
    <ul class="space-y-1.5">
      {#each needs as n (n.need.id)}
        {@const u = thumb(n)}
        <li
          class="flex items-center gap-2 rounded-lg border border-surface-800 bg-surface-950/40 p-1.5"
        >
          <div class="h-11 w-16 shrink-0 overflow-hidden rounded bg-black/40">
            {#if u}
              <img src={u} alt="" class="h-full w-full object-cover" />
            {/if}
          </div>
          <div class="min-w-0 flex-1">
            <p class="truncate font-medium text-surface-100">{n.need.label}</p>
            <p class="text-[10px] text-surface-500">
              {formatTimeRange(n.need.outputStart, n.need.outputEnd)} · {stateText(n)}
            </p>
          </div>
          {#if n.uiState === "queued" || n.uiState === "processing"}
            <button
              type="button"
              class="btn-ghost shrink-0 text-[10px]"
              disabled={busy || !n.job?.id}
              onclick={() => n.job?.id && onCancel(n.job.id)}
            >
              Cancelar
            </button>
          {:else if n.uiState === "cancelling"}
            <span class="shrink-0 text-[10px] text-surface-500">Cancelando…</span>
          {:else}
            <button
              type="button"
              class="btn-primary shrink-0 text-[10px]"
              disabled={busy}
              onclick={() => onPrimary(n)}
            >
              {actionLabel(n)}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>
