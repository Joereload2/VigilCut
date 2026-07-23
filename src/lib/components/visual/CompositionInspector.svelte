<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import type { CompositionIssue, DisplayMode, VisualPlacement } from "./types";
  import { isFullscreenMode, reviewLabel } from "./types";

  interface Props {
    placement: VisualPlacement | null;
    thumbPath?: string | null;
    issues?: CompositionIssue[];
    busy?: boolean;
    onUpdate: (patch: {
      outputStart?: number;
      outputEnd?: number;
      displayMode?: string;
      positionX?: number;
      positionY?: number;
      sizeW?: number;
      sizeH?: number;
      fit?: string;
      reviewStatus?: string;
      manualOverride?: boolean;
      restoreAi?: boolean;
      opacity?: number;
    }) => void;
    onRemove: () => void;
    onApplySuggestion?: (issue: CompositionIssue) => void;
  }

  let {
    placement,
    thumbPath = null,
    issues = [],
    busy = false,
    onUpdate,
    onRemove,
    onApplySuggestion,
  }: Props = $props();

  function thumbUrl(p?: string | null) {
    if (!p) return null;
    try {
      return convertFileSrc(p);
    } catch {
      return null;
    }
  }

  const url = $derived(thumbUrl(thumbPath));
  const mine = $derived(issues.filter((i) => placement && i.placementId === placement.id));
  const full = $derived(placement ? isFullscreenMode(placement.mode) : true);
</script>

{#if !placement}
  <section class="rounded-xl border border-dashed border-surface-800 p-3 text-[10px] text-surface-500">
    Selecciona un B-roll en la línea de tiempo. Solo las excepciones (⚠) requieren atención.
  </section>
{:else}
  <section class="space-y-2 rounded-xl border border-surface-800 bg-surface-950/60 p-3">
    <div class="flex items-center gap-2">
      {#if url}
        <img src={url} alt="" class="h-12 w-12 rounded object-cover ring-1 ring-surface-700" />
      {/if}
      <div class="min-w-0 flex-1">
        <div class="truncate text-xs font-semibold text-surface-100">
          {placement.label || placement.assetId}
        </div>
        <div class="font-mono text-[9px] text-surface-500">
          {placement.outputStart.toFixed(1)}–{placement.outputEnd.toFixed(1)}s ·
          {reviewLabel(placement.reviewStatus)}
          {#if placement.confidence != null}
            · {(placement.confidence * 100).toFixed(0)}%
          {/if}
        </div>
        {#if placement.relatedText}
          <div class="mt-0.5 truncate text-[10px] text-amber-200/90">
            «{placement.relatedText}»
          </div>
        {/if}
      </div>
    </div>

    {#if mine.length}
      <div class="space-y-1 rounded-lg border border-amber-700/40 bg-amber-950/30 p-2">
        <div class="text-[10px] font-semibold text-amber-200">Excepciones a supervisar</div>
        {#each mine as iss (iss.id)}
          <div class="text-[10px] text-amber-100/90">
            <span class="font-mono text-amber-400/80">[{iss.severity}]</span>
            {iss.message}
            {#if onApplySuggestion && (iss.suggestedX != null || iss.suggestedY != null)}
              <button
                type="button"
                class="ml-1 text-sky-300 underline"
                disabled={busy}
                onclick={() => onApplySuggestion(iss)}>Usar posición alt.</button
              >
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <div class="grid grid-cols-2 gap-2 text-[10px]">
      <label class="text-surface-400">
        Entrada
        <input
          type="number"
          step="0.1"
          class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
          value={placement.outputStart.toFixed(1)}
          disabled={busy}
          onchange={(e) =>
            onUpdate({ outputStart: parseFloat((e.currentTarget as HTMLInputElement).value) })}
        />
      </label>
      <label class="text-surface-400">
        Salida
        <input
          type="number"
          step="0.1"
          class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1 font-mono text-surface-100"
          value={placement.outputEnd.toFixed(1)}
          disabled={busy}
          onchange={(e) =>
            onUpdate({ outputEnd: parseFloat((e.currentTarget as HTMLInputElement).value) })}
        />
      </label>
    </div>

    <div class="flex flex-wrap gap-1">
      <button
        type="button"
        class="rounded border px-2 py-0.5 text-[10px]
          {full ? 'border-sky-400 bg-sky-700 text-white' : 'border-surface-700 text-surface-400'}"
        disabled={busy}
        onclick={() => onUpdate({ displayMode: "fullscreen" })}>Fullscreen</button
      >
      <button
        type="button"
        class="rounded border px-2 py-0.5 text-[10px]
          {!full ? 'border-sky-400 bg-sky-700 text-white' : 'border-surface-700 text-surface-400'}"
        disabled={busy}
        onclick={() => onUpdate({ displayMode: "overlay" })}>Overlay</button
      >
    </div>

    {#if !full}
      <div class="flex flex-wrap gap-1 text-[10px]">
        {#each ["contain", "cover", "crop"] as f}
          <button
            type="button"
            class="rounded border px-2 py-0.5
              {placement.fit === f
              ? 'border-violet-400 bg-violet-800 text-white'
              : 'border-surface-700 text-surface-400'}"
            disabled={busy}
            onclick={() => onUpdate({ fit: f })}>{f}</button
          >
        {/each}
      </div>
      <p class="text-[9px] text-surface-500">
        Posición y tamaño: arrastra en la vista previa (esquinas = redimensionar).
      </p>
    {/if}

    <div class="flex flex-wrap gap-1">
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
    </div>

    {#if placement.manualOverride}
      <p class="text-[9px] text-sky-400/80">Override manual: un re-análisis no lo sobrescribe.</p>
    {/if}
  </section>
{/if}
