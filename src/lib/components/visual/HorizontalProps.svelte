<script lang="ts">
  /** Responsive horizontal property strip for selected B-roll (no sidebar). */
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { CompositionIssue, VisualPlacement } from "./types";
  import { isFullscreenMode, reviewLabel } from "./types";

  interface Props {
    placement: VisualPlacement | null;
    thumbPath?: string | null;
    issues?: CompositionIssue[];
    busy?: boolean;
    onUpdate: (patch: Record<string, unknown>) => void;
    onRemove: () => void;
    onApplySuggestion?: (issue: CompositionIssue) => void;
    onExport?: () => void;
  }

  let {
    placement,
    thumbPath = null,
    issues = [],
    busy = false,
    onUpdate,
    onRemove,
    onApplySuggestion,
    onExport,
  }: Props = $props();

  function url(p?: string | null) {
    if (!p) return null;
    try {
      return convertFileSrc(p);
    } catch {
      return null;
    }
  }

  const u = $derived(url(thumbPath));
  const mine = $derived(issues.filter((i) => placement && i.placementId === placement.id));
  const full = $derived(placement ? isFullscreenMode(placement.mode) : true);
  const dur = $derived(
    placement ? Math.max(0, placement.outputEnd - placement.outputStart) : 0,
  );
</script>

{#if !placement}
  <div class="flex min-h-[3rem] items-center px-1 text-[11px] text-surface-500">
    Selecciona un bloque B-roll en la línea de tiempo. Solo las excepciones requieren atención.
  </div>
{:else}
  <div
    class="grid min-w-0 w-full max-w-full grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6 xl:grid-cols-8"
    style="box-sizing:border-box"
  >
    <div class="col-span-2 flex min-w-0 items-center gap-2 sm:col-span-1 lg:col-span-2">
      {#if u}
        <img src={u} alt="" class="h-12 w-12 shrink-0 rounded object-cover ring-1 ring-surface-700" />
      {/if}
      <div class="min-w-0">
        <div class="truncate text-xs font-semibold text-surface-100">
          {placement.label || "B-roll"}
        </div>
        <div class="truncate text-[9px] text-surface-500">
          {reviewLabel(placement.reviewStatus)}
          {#if placement.confidence != null}
            · {(placement.confidence * 100).toFixed(0)}%
          {/if}
        </div>
        {#if placement.relatedText}
          <div class="truncate text-[10px] text-amber-200/90">«{placement.relatedText}»</div>
        {/if}
      </div>
    </div>

    <label class="min-w-0 text-[10px] text-surface-400">
      Entrada
      <input
        type="number"
        step="0.1"
        class="mt-0.5 w-full min-w-0 rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
        value={placement.outputStart.toFixed(1)}
        disabled={busy}
        onchange={(e) =>
          onUpdate({ outputStart: parseFloat((e.currentTarget as HTMLInputElement).value) })}
      />
    </label>
    <label class="min-w-0 text-[10px] text-surface-400">
      Salida
      <input
        type="number"
        step="0.1"
        class="mt-0.5 w-full min-w-0 rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
        value={placement.outputEnd.toFixed(1)}
        disabled={busy}
        onchange={(e) =>
          onUpdate({ outputEnd: parseFloat((e.currentTarget as HTMLInputElement).value) })}
      />
    </label>
    <div class="min-w-0 text-[10px] text-surface-400">
      Duración
      <div class="mt-0.5 font-mono text-surface-200">{dur.toFixed(1)}s</div>
    </div>

    <div class="col-span-2 flex min-w-0 flex-wrap items-end gap-1 sm:col-span-1 lg:col-span-2">
      <button
        type="button"
        class="rounded border px-2 py-1 text-[10px]
          {full ? 'border-sky-400 bg-sky-700 text-white' : 'border-surface-700 text-surface-400'}"
        disabled={busy}
        onclick={() => onUpdate({ displayMode: "fullscreen" })}>Fullscreen</button
      >
      <button
        type="button"
        class="rounded border px-2 py-1 text-[10px]
          {!full ? 'border-sky-400 bg-sky-700 text-white' : 'border-surface-700 text-surface-400'}"
        disabled={busy}
        onclick={() => onUpdate({ displayMode: "overlay" })}>Overlay</button
      >
      {#if !full}
        {#each ["contain", "cover", "crop"] as f}
          <button
            type="button"
            class="rounded border px-1.5 py-1 text-[9px]
              {placement.fit === f
              ? 'border-violet-400 bg-violet-800 text-white'
              : 'border-surface-700 text-surface-400'}"
            disabled={busy}
            onclick={() => onUpdate({ fit: f })}>{f}</button
          >
        {/each}
      {/if}
    </div>

    <div class="col-span-2 flex min-w-0 flex-wrap items-end gap-1 lg:col-span-2">
      <button
        type="button"
        class="btn-secondary text-[10px]"
        disabled={busy}
        onclick={() => onUpdate({ reviewStatus: "approved", manualOverride: true })}
        >Aceptar</button
      >
      <button
        type="button"
        class="btn-ghost text-[10px]"
        disabled={busy}
        onclick={() => onUpdate({ restoreAi: true })}>Restaurar IA</button
      >
      <button type="button" class="btn-ghost text-[10px] text-cut" disabled={busy} onclick={onRemove}
        >Eliminar</button
      >
      {#if onExport}
        <button type="button" class="btn-primary text-[10px]" disabled={busy} onclick={onExport}
          >Exportar</button
        >
      {/if}
    </div>
  </div>

  {#if mine.length}
    <div class="mt-2 flex min-w-0 flex-wrap gap-2 rounded-lg border border-amber-700/40 bg-amber-950/20 p-2">
      {#each mine as iss (iss.id)}
        <div class="min-w-0 max-w-full text-[10px] text-amber-100/90">
          <span class="font-mono text-amber-400/80">[{iss.severity}]</span>
          {iss.message}
          {#if onApplySuggestion && (iss.suggestedX != null || iss.suggestedY != null)}
            <button
              type="button"
              class="ml-1 text-sky-300 underline"
              disabled={busy}
              onclick={() => onApplySuggestion(iss)}>Alt.</button
            >
          {/if}
        </div>
      {/each}
    </div>
  {/if}
{/if}
