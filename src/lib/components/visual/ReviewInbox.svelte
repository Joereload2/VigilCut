<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { CandidateView } from "./imageGenTypes";
  import { costLabel, isMockProvider } from "./imageGenTypes";

  let { candidates = [] as CandidateView[], busyId = null as string | null, onApprove, onReject, onRegenerate }: {
    candidates?: CandidateView[]; busyId?: string | null;
    onApprove: (c: CandidateView, place: boolean) => void;
    onReject: (c: CandidateView, reason: string) => void;
    onRegenerate: (c: CandidateView, prompt: string, negativePrompt: string) => void;
  } = $props();
  let rejectId = $state<string | null>(null);
  let rejectReason = $state("");
  let editingId = $state<string | null>(null);
  let editPrompt = $state("");
  let editNegative = $state("");

  function fileUrl(path?: string | null) { if (!path) return null; try { return convertFileSrc(path); } catch { return null; } }
  function titleOf(c: CandidateView) { return c.conceptTitle || c.needLabel || "Imagen generada"; }
  function originLabel(c: CandidateView) {
    if (c.origin === "library_request") return "Solicitud manual de Biblioteca";
    if (c.origin === "daily_feed") return "Alimentación diaria";
    return "Solicitud de B-roll";
  }
  function beginEdit(c: CandidateView) { editingId = c.id; editPrompt = c.prompt ?? ""; editNegative = c.negativePrompt ?? ""; }
</script>

<div class="flex min-h-0 flex-col gap-3 overflow-y-auto text-[11px]">
  <div><h2 class="text-sm font-semibold text-white">Por revisar</h2><p class="text-[11px] text-surface-400">Nada entra en la Biblioteca hasta que lo apruebes.</p></div>
  {#if candidates.length === 0}<div class="rounded-xl border border-dashed border-surface-700 p-6 text-center"><p class="text-sm text-surface-300">No hay imágenes pendientes.</p><p class="mt-1 text-[10px] text-surface-500">Las nuevas generaciones aparecerán aquí.</p></div>{/if}

  <div class="grid gap-3 xl:grid-cols-2">
    {#each candidates as c (c.id)}
      {@const src = c.fileExists ? fileUrl(c.localPath) : null}
      <article class="overflow-hidden rounded-xl border border-surface-800 bg-surface-950/60">
        <div class="aspect-video w-full bg-black/40">{#if src}<img src={src} alt={titleOf(c)} class="h-full w-full object-contain" />{:else}<div class="flex h-full items-center justify-center text-surface-500">Vista previa no disponible</div>{/if}</div>
        <div class="space-y-2 p-3">
          <div class="flex flex-wrap items-start justify-between gap-2"><div><h3 class="text-sm font-semibold text-white">{titleOf(c)}</h3><p class="text-[10px] text-violet-300">{c.themeTitle ? `Tema: ${c.themeTitle} · ` : ""}{originLabel(c)}</p></div>{#if isMockProvider(c.provider)}<span class="rounded-full bg-violet-900/60 px-2 py-1 text-[9px] font-semibold text-violet-100">SIMULACIÓN · NO ES IA REAL</span>{/if}</div>
          <dl class="grid grid-cols-2 gap-x-3 gap-y-1 text-[10px]"><div><dt class="text-surface-500">Formato</dt><dd>{c.width ?? "?"}×{c.height ?? "?"} · {c.mimeType ?? "—"}</dd></div><div><dt class="text-surface-500">Proveedor / modelo</dt><dd>{c.provider ?? "—"} · {c.model ?? "—"}</dd></div><div><dt class="text-surface-500">Coste</dt><dd>{costLabel(c.costKind, c.freeVerified)}</dd></div><div><dt class="text-surface-500">Fecha</dt><dd>{c.createdAt?.slice(0, 16) ?? "—"}</dd></div></dl>
          {#if c.qaReason}<p class="rounded bg-surface-900 p-2 text-[10px] text-surface-300"><span class="text-surface-500">QA:</span> {c.qaReason}</p>{/if}
          {#if editingId === c.id}
            <label class="block text-[10px] text-surface-400">Prompt positivo<textarea class="mt-1 min-h-24 w-full rounded border border-surface-700 bg-surface-900 p-2 text-[11px]" bind:value={editPrompt}></textarea></label>
            <label class="block text-[10px] text-surface-400">Prompt negativo<textarea class="mt-1 min-h-16 w-full rounded border border-surface-700 bg-surface-900 p-2 text-[11px]" bind:value={editNegative}></textarea></label>
            <p class="text-[9px] text-surface-500">Estrategia del proveedor: {c.promptStrategy ?? "separado cuando está soportado"}</p>
            <div class="flex gap-2"><button type="button" class="btn-primary text-[10px]" disabled={busyId === c.id || !c.requestId} onclick={() => onRegenerate(c, editPrompt, editNegative)}>Regenerar con cambios</button><button type="button" class="btn-ghost text-[10px]" onclick={() => (editingId = null)}>Cerrar</button></div>
          {:else if rejectId === c.id}
            <input class="w-full rounded border border-surface-700 bg-surface-900 p-2" bind:value={rejectReason} placeholder="Motivo del rechazo" />
            <div class="flex gap-2"><button type="button" class="btn-primary text-[10px]" disabled={busyId === c.id} onclick={() => { onReject(c, rejectReason || "Rechazo humano"); rejectId = null; rejectReason = ""; }}>Confirmar rechazo</button><button type="button" class="btn-ghost text-[10px]" onclick={() => (rejectId = null)}>Volver</button></div>
          {:else}
            <details class="rounded border border-surface-800 bg-surface-900/60 p-2"><summary class="cursor-pointer text-[10px] text-violet-300">Ver prompts</summary><p class="mt-2 whitespace-pre-wrap text-[10px] text-surface-300"><span class="text-surface-500">Positivo:</span> {c.prompt || "—"}</p><p class="mt-2 whitespace-pre-wrap text-[10px] text-surface-300"><span class="text-surface-500">Negativo:</span> {c.negativePrompt || "—"}</p></details>
            <div class="flex flex-wrap gap-2"><button type="button" class="btn-primary text-[10px]" disabled={busyId === c.id} onclick={() => onApprove(c, false)}>Aprobar para Biblioteca</button><button type="button" class="btn-secondary text-[10px]" disabled={busyId === c.id || !c.requestId} onclick={() => beginEdit(c)}>Editar y regenerar</button><button type="button" class="btn-secondary text-[10px]" disabled={busyId === c.id || !c.requestId} onclick={() => onRegenerate(c, c.prompt ?? "", c.negativePrompt ?? "")}>Regenerar</button><button type="button" class="btn-ghost text-[10px] text-red-300" disabled={busyId === c.id} onclick={() => { rejectId = c.id; rejectReason = ""; }}>Rechazar</button></div>
          {/if}
        </div>
      </article>
    {/each}
  </div>
</div>
