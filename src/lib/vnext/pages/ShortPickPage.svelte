<script lang="ts">
  /**
   * PARTE 1 — Revisar un short por vez (pestañas).
   * Cada pestaña: video del tramo + duración + texto propio.
   * Descartar o Usar; solo los "Usar" pasan a encuadre 9:16.
   */
  import SegmentPlayer from "$lib/vnext/components/SegmentPlayer.svelte";
  import MiniTimeline from "$lib/vnext/components/MiniTimeline.svelte";
  import { vnextSession } from "$lib/vnext/stores/sessionStore.svelte";
  import { formatDuration, scoreToHuman } from "$lib/vnext/presentation/jobLabels";

  type Decision = "pending" | "keep" | "discard";

  interface Draft {
    start: number;
    end: number;
    decision: Decision;
  }

  /** Incluye descartados para poder re-elegirlos (memoria / reabrir proyecto). */
  const list = $derived(
    [...vnextSession.candidates]
      .filter((c) => c.status !== "error" && c.status !== "exporting")
      .sort((a, b) => b.score - a.score),
  );

  let drafts = $state<Record<string, Draft>>({});
  let tabId = $state<string | null>(null);
  // plain let: no re-trigger seed effect
  let seededKey = "";

  const media = $derived(vnextSession.project?.sourceMediaPath ?? null);

  const current = $derived(
    list.find((c) => c.id === tabId) ?? list[0] ?? null,
  );
  const draft = $derived(
    current
      ? (drafts[current.id] ?? {
          start: current.start,
          end: current.end,
          decision: "pending" as Decision,
        })
      : null,
  );

  const keepCount = $derived(
    Object.values(drafts).filter((d) => d.decision === "keep").length,
  );
  const pendingCount = $derived(
    list.filter((c) => (drafts[c.id]?.decision ?? "pending") === "pending").length,
  );
  const tabIndex = $derived(
    current ? Math.max(0, list.findIndex((c) => c.id === current.id)) : 0,
  );

  // Seed drafts when candidate set changes (by ids only)
  $effect(() => {
    const key = list.map((c) => c.id).join("|");
    if (!key) {
      drafts = {};
      tabId = null;
      seededKey = "";
      return;
    }
    if (key === seededKey) {
      // Merge new candidates only; keep decisions + current tab
      return;
    }
    const next: Record<string, Draft> = {};
    for (const c of list) {
      const prev = drafts[c.id];
      let already: Decision;
      if (prev?.decision) {
        already = prev.decision; // prefer local session choice
      } else if (c.status === "rejected" || c.status === "discarded") {
        already = "discard"; // visible, re-elegible con "Usar"
      } else if (
        c.status === "approved" ||
        c.status === "modified" ||
        c.status === "exported"
      ) {
        already = "keep";
      } else {
        already = "pending";
      }
      next[c.id] = {
        start: prev?.start ?? c.start,
        end: prev?.end ?? c.end,
        decision: already,
      };
    }
    drafts = next;
    // Solo resetear pestaña si no hay tab válida
    if (!tabId || !next[tabId]) {
      tabId = list[0]?.id ?? null;
    }
    seededKey = key;
  });

  $effect(() => {
    if (current) {
      vnextSession.selectedCandidateId = current.id;
    }
  });

  function goTab(id: string) {
    tabId = id;
  }

  function setSpan(s: number, e: number) {
    if (!current) return;
    drafts = {
      ...drafts,
      [current.id]: {
        ...(drafts[current.id] ?? {
          decision: "pending" as Decision,
          start: s,
          end: e,
        }),
        start: s,
        end: e,
      },
    };
  }

  /** Avanza al siguiente pendiente usando el mapa de drafts recién actualizado. */
  function goNextPendingOrNext(fromId: string, map: Record<string, Draft>) {
    const idx = list.findIndex((c) => c.id === fromId);
    const isPending = (id: string) => (map[id]?.decision ?? "pending") === "pending";

    for (let i = idx + 1; i < list.length; i++) {
      if (isPending(list[i].id)) {
        tabId = list[i].id;
        return;
      }
    }
    for (let i = 0; i < idx; i++) {
      if (isPending(list[i].id)) {
        tabId = list[i].id;
        return;
      }
    }
    // Todos decididos: ir al primero marcado usar
    const firstKeep = list.find((c) => map[c.id]?.decision === "keep");
    if (firstKeep) tabId = firstKeep.id;
  }

  function markDiscard() {
    if (!current) return;
    const id = current.id;
    const d = drafts[id] ?? {
      start: current.start,
      end: current.end,
      decision: "pending" as Decision,
    };
    const next = { ...drafts, [id]: { ...d, decision: "discard" as Decision } };
    drafts = next;
    goNextPendingOrNext(id, next);
  }

  function markKeep() {
    if (!current || !draft) return;
    // No bloquear por pendingAction del backend: la UI avanza igual
    const id = current.id;
    const s = draft.start;
    const e = draft.end;
    const next = { ...drafts, [id]: { start: s, end: e, decision: "keep" as Decision } };
    drafts = next;
    // Avanzar YA (antes del await) — era el bug de "no avanza al aceptar"
    goNextPendingOrNext(id, next);
    // Guardar duración en segundo plano
    vnextSession.selectedCandidateId = id;
    void vnextSession.saveSpan(s, e).catch(() => {
      /* decisión local se conserva */
    });
  }

  async function finishAndGoFraming() {
    if (keepCount === 0 || vnextSession.pendingAction) return;
    const keep = list
      .filter((c) => drafts[c.id]?.decision === "keep")
      .map((c) => ({
        id: c.id,
        start: drafts[c.id]!.start,
        end: drafts[c.id]!.end,
      }));
    const discardIds = list
      .filter((c) => drafts[c.id]?.decision === "discard")
      .map((c) => c.id);
    await vnextSession.finalizeMomentSelection({ keep, discardIds });
  }

  function titleFor(c: (typeof list)[0], i: number) {
    const t = (c.title || "").trim();
    if (t) return t.length > 28 ? t.slice(0, 28) + "…" : t;
    return `Momento ${i + 1}`;
  }
</script>

<div class="flex h-full max-h-full min-h-0 flex-col overflow-hidden" data-testid="page-short-pick">
  <div class="shrink-0 border-b border-surface-800 px-3 py-1">
    <p class="text-[10px] font-bold uppercase tracking-wide text-vigil-400">
      Parte 1 de 2 · Revisar cada Short
      <span class="ml-2 font-normal normal-case tracking-normal text-surface-500">
        · pestaña = momento · play · duración · usar/descartar
      </span>
    </p>
  </div>

  {#if list.length === 0}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center">
      <p class="text-lg font-semibold text-white">Sin momentos</p>
      <button
        type="button"
        class="rounded-xl bg-vigil-600 px-5 py-2 text-sm font-semibold text-white"
        onclick={() => vnextSession.goCreate()}
      >Analizar un video</button>
    </div>
  {:else}
    <!-- Pestañas: un short cada una -->
    <div
      class="flex shrink-0 gap-1 overflow-x-auto border-b border-surface-800 px-2 py-1"
      data-testid="moment-tabs"
      role="tablist"
    >
      {#each list as c, i (c.id)}
        {@const d = drafts[c.id]?.decision ?? "pending"}
        {@const active = current?.id === c.id}
        <button
          type="button"
          role="tab"
          aria-selected={active}
          class="shrink-0 rounded-md border px-2 py-1 text-left text-[10px] transition
            {active
            ? 'border-vigil-500 bg-vigil-950/50 text-white'
            : d === 'keep'
              ? 'border-emerald-700/50 bg-emerald-950/30 text-emerald-200'
              : d === 'discard'
                ? 'border-surface-600 bg-surface-900/50 text-surface-400'
                : 'border-surface-700 bg-surface-900 text-surface-300 hover:border-surface-500'}"
          onclick={() => goTab(c.id)}
          title={d === "discard"
            ? "Descartado — podés volver a usarlo"
            : c.title || `Momento ${i + 1}`}
        >
          <span class="font-bold">#{i + 1}</span>
          {#if d === "keep"}
            <span class="text-emerald-400"> ✓</span>
          {:else if d === "discard"}
            <span class="text-amber-500/90" title="Descartado (re-elegible)"> ↩</span>
          {/if}
          <span class="ml-1 font-mono text-[10px] opacity-70"
            >{formatDuration((drafts[c.id]?.end ?? c.end) - (drafts[c.id]?.start ?? c.start))}</span
          >
        </button>
      {/each}
      <span class="ml-auto self-center whitespace-nowrap px-2 text-[10px] text-surface-500">
        {keepCount} usar · {pendingCount} por revisar · {list.length} total
      </span>
    </div>

    {#if current && draft}
      <div
        class="grid min-h-0 flex-1 grid-cols-1 gap-1.5 overflow-hidden p-1.5 lg:grid-cols-[minmax(0,1.35fr)_minmax(14rem,0.9fr)]"
      >
        <!-- Video + duración de ESTE short -->
        <section class="flex min-h-0 flex-col gap-1 overflow-hidden rounded-lg border border-surface-800 bg-black/50 p-1.5">
          <div class="flex shrink-0 items-baseline justify-between gap-2">
            <p class="text-[9px] font-bold uppercase text-surface-500">
              #{tabIndex + 1} · original
            </p>
            <p class="truncate text-[10px] text-surface-400">{titleFor(current, tabIndex)}</p>
          </div>

          <div class="min-h-0 flex-1 overflow-hidden">
            <SegmentPlayer
              sourcePath={media}
              start={draft.start}
              end={draft.end}
              autoplay={false}
            />
          </div>

          <div class="shrink-0 rounded border border-surface-800 bg-surface-900/80 px-2 py-1">
            <MiniTimeline
              start={draft.start}
              end={draft.end}
              editable={true}
              onChange={(s, e) => setSpan(s, e)}
            />
          </div>
        </section>

        <!-- Texto + decisión de ESTE short -->
        <aside class="flex min-h-0 flex-col overflow-hidden rounded-lg border border-surface-800 bg-surface-900/40">
          <div class="shrink-0 border-b border-surface-800 px-2 py-1.5">
            <p class="text-[9px] font-bold uppercase text-surface-400">Este momento</p>
            <p class="truncate text-xs font-semibold text-white">{titleFor(current, tabIndex)}</p>
            <p class="text-[10px] text-surface-500">
              {scoreToHuman(current.score)}
              · {formatDuration(draft.start)}–{formatDuration(draft.end)}
            </p>
            {#if draft.decision === "keep"}
              <p class="text-[10px] font-medium text-emerald-400">Usar</p>
            {:else if draft.decision === "discard"}
              <p class="text-[10px] font-medium text-amber-400/90">Descartado (re-elegible)</p>
            {:else}
              <p class="text-[10px] text-amber-400/90">Sin decidir</p>
            {/if}
          </div>

          <div class="min-h-0 flex-1 overflow-y-auto px-2 py-1.5">
            <p class="text-[9px] font-bold uppercase text-surface-500">Texto</p>
            <p class="mt-1 text-[11px] leading-snug text-surface-100">
              {current.transcript || current.summary || "Sin texto"}
            </p>
          </div>

          <div class="shrink-0 space-y-1.5 border-t border-surface-800 p-2">
            <div class="grid grid-cols-2 gap-1.5">
              <button
                type="button"
                class="rounded-lg border border-surface-600 py-2 text-xs font-semibold text-surface-200 hover:bg-surface-800 disabled:opacity-40"
                data-testid="cta-discard-moment"
                disabled={!!vnextSession.pendingAction}
                onclick={() => markDiscard()}
              >
                Descartar
              </button>
              <button
                type="button"
                class="rounded-lg bg-vigil-600 py-2 text-xs font-semibold text-white hover:bg-vigil-500 disabled:opacity-40"
                data-testid="cta-keep-moment"
                onclick={() => markKeep()}
              >
                {draft.decision === "keep" ? "Usar ✓ · sig." : "Usar · sig."}
              </button>
            </div>
            <button
              type="button"
              class="w-full rounded-lg border border-vigil-600/60 py-2 text-xs font-semibold text-vigil-300 hover:bg-vigil-950/40 disabled:opacity-40"
              data-testid="cta-confirm-selected"
              disabled={keepCount === 0 || !!vnextSession.pendingAction}
              onclick={() => void finishAndGoFraming()}
            >
              {#if vnextSession.pendingAction}
                Guardando…
              {:else if keepCount === 0}
                Usá al menos un Short para continuar
              {:else}
                Continuar con {keepCount} Short{keepCount === 1 ? "" : "s"} → Encuadre 9:16
              {/if}
            </button>
            {#if pendingCount > 0}
              <p class="text-center text-[9px] text-surface-500">
                {pendingCount} sin revisar · podés continuar igual
              </p>
            {/if}
          </div>
        </aside>
      </div>
    {/if}
  {/if}
</div>
