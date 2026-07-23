<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { CandidateView } from "./imageGenTypes";
  import { costLabel, isMockProvider } from "./imageGenTypes";

  let {
    candidates = [] as CandidateView[],
    busyId = null as string | null,
    onApprove,
    onReject,
    onRegenerate = undefined as ((needId: string) => void) | undefined,
  }: {
    candidates?: CandidateView[];
    busyId?: string | null;
    onApprove: (c: CandidateView, place: boolean) => void;
    onReject: (c: CandidateView, reason: string) => void;
    onRegenerate?: ((needId: string) => void) | undefined;
  } = $props();

  let rejectId = $state<string | null>(null);
  let rejectReason = $state("");
  let postApproveId = $state<string | null>(null);
  /** PM-004: after confirmed reject, offer regenerate */
  let postRejectId = $state<string | null>(null);
  let postRejectNeedId = $state<string | null>(null);

  const forVideo = $derived(candidates.filter((c) => c.origin !== "daily_feed"));
  const forLibrary = $derived(candidates.filter((c) => c.origin === "daily_feed"));

  function fileUrl(path?: string | null) {
    if (!path) return null;
    try {
      return convertFileSrc(path);
    } catch {
      return null;
    }
  }

  function titleOf(c: CandidateView) {
    return c.conceptTitle || c.needLabel || "Imagen generada";
  }

  async function confirmReject(c: CandidateView) {
    const reason = rejectReason || "Rechazo humano";
    await Promise.resolve(onReject(c, reason));
    postRejectId = c.id;
    postRejectNeedId = c.needId ?? null;
    rejectId = null;
    rejectReason = "";
  }
</script>

<div class="flex min-h-0 flex-col gap-3 overflow-y-auto text-[11px]">
  {#if candidates.length === 0 && !postRejectId}
    <p class="rounded-lg border border-dashed border-surface-700 p-3 text-surface-400">
      No hay imágenes pendientes de revisión.
    </p>
  {/if}

  {#if forVideo.length > 0}
    <section>
      <h3 class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-surface-500">
        Para este video
      </h3>
      <div class="space-y-2">
        {#each forVideo as c (c.id)}
          {@render card(c, true)}
        {/each}
      </div>
    </section>
  {/if}

  {#if forLibrary.length > 0}
    <section>
      <h3 class="mb-1 text-[10px] font-semibold uppercase tracking-wide text-surface-500">
        Para la Biblioteca
      </h3>
      <div class="space-y-2">
        {#each forLibrary as c (c.id)}
          {@render card(c, false)}
        {/each}
      </div>
    </section>
  {/if}

  {#if postRejectId && !candidates.some((c) => c.id === postRejectId)}
    <div class="rounded-lg border border-surface-700 bg-surface-900/60 p-2">
      <p class="text-surface-200">¿Quieres generar otra versión?</p>
      <div class="mt-2 flex flex-wrap gap-1">
        {#if onRegenerate && postRejectNeedId}
          <button
            type="button"
            class="btn-primary text-[10px]"
            onclick={() => {
              onRegenerate(postRejectNeedId!);
              postRejectId = null;
              postRejectNeedId = null;
            }}>Generar otra</button
          >
        {/if}
        <button
          type="button"
          class="btn-ghost text-[10px]"
          onclick={() => {
            postRejectId = null;
            postRejectNeedId = null;
          }}>Ahora no</button
        >
      </div>
    </div>
  {/if}
</div>

{#snippet card(c: CandidateView, allowPlace: boolean)}
  {@const u = c.fileExists ? fileUrl(c.localPath) : null}
  <div class="rounded-lg border border-surface-800 bg-surface-950/50 p-2">
    <div class="flex gap-2">
      <div class="h-20 w-28 shrink-0 overflow-hidden rounded bg-black/40 sm:h-24 sm:w-36">
        {#if u}
          <img src={u} alt={titleOf(c)} class="h-full w-full object-cover" />
        {:else}
          <div class="flex h-full items-center justify-center text-[9px] text-surface-500">—</div>
        {/if}
      </div>
      <div class="min-w-0 flex-1">
        <p class="font-medium text-surface-100">{titleOf(c)}</p>
        <p class="text-[10px] text-surface-500">
          {c.origin === "daily_feed" ? "Biblioteca automática" : "Este video"}
          · {costLabel(c.costKind, c.freeVerified)}
        </p>
        {#if isMockProvider(c.provider)}
          <p class="text-[9px] text-violet-300">Simulación (mock) — no es IA</p>
        {/if}
        {#if c.qaReason}
          <p class="mt-0.5 line-clamp-2 text-[10px] text-surface-400">{c.qaReason}</p>
        {/if}
      </div>
    </div>

    {#if postApproveId === c.id && allowPlace}
      <div class="mt-2 flex flex-wrap gap-1">
        <button
          type="button"
          class="btn-primary text-[10px]"
          disabled={busyId === c.id}
          onclick={() => onApprove(c, true)}>Usar ahora en el video</button
        >
        <button
          type="button"
          class="btn-secondary text-[10px]"
          disabled={busyId === c.id}
          onclick={() => {
            postApproveId = null;
            onApprove(c, false);
          }}>Solo guardar en Biblioteca</button
        >
      </div>
    {:else if postRejectId === c.id}
      <div class="mt-2 space-y-1">
        <p class="text-surface-200">¿Quieres generar otra versión?</p>
        <div class="flex flex-wrap gap-1">
          {#if onRegenerate && c.needId}
            <button
              type="button"
              class="btn-primary text-[10px]"
              disabled={busyId === c.id}
              onclick={() => {
                onRegenerate(c.needId!);
                postRejectId = null;
                postRejectNeedId = null;
              }}>Generar otra</button
            >
          {/if}
          <button
            type="button"
            class="btn-ghost text-[10px]"
            onclick={() => {
              postRejectId = null;
              postRejectNeedId = null;
            }}>Ahora no</button
          >
        </div>
      </div>
    {:else if rejectId === c.id}
      <div class="mt-2 space-y-1">
        <div class="flex flex-wrap gap-1">
          {#each ["No representa el concepto", "Mala calidad", "Contexto incorrecto"] as chip}
            <button
              type="button"
              class="rounded bg-surface-800 px-1.5 py-0.5 text-[9px]"
              onclick={() => (rejectReason = chip)}>{chip}</button
            >
          {/each}
        </div>
        <input
          class="w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 text-[10px]"
          placeholder="Motivo…"
          bind:value={rejectReason}
        />
        <div class="flex gap-1">
          <button
            type="button"
            class="btn-primary text-[10px]"
            disabled={busyId === c.id}
            onclick={() => void confirmReject(c)}>Confirmar rechazo</button
          >
          <button type="button" class="btn-ghost text-[10px]" onclick={() => (rejectId = null)}
            >Cerrar</button
          >
        </div>
      </div>
    {:else}
      <!-- PM-004: solo Aprobar y Rechazar (sin Generar otra aquí) -->
      <div class="mt-2 flex flex-wrap gap-1">
        <button
          type="button"
          class="btn-primary text-[10px]"
          disabled={busyId === c.id}
          onclick={() => {
            if (allowPlace) {
              postApproveId = c.id;
            } else {
              onApprove(c, false);
            }
          }}
        >
          Aprobar
        </button>
        <button
          type="button"
          class="btn-ghost text-[10px]"
          disabled={busyId === c.id}
          onclick={() => {
            rejectId = c.id;
            rejectReason = "";
          }}
        >
          Rechazar
        </button>
      </div>
    {/if}
  </div>
{/snippet}
