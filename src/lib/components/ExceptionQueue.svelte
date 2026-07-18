<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import { formatTime } from "$lib/types";

  const pending = $derived(projectStore.pendingExceptions);
  const stats = $derived(projectStore.analysisRun?.stats);

  function seekTo(start: number) {
    projectStore.currentTime = start;
    const seg = projectStore.segments.find(
      (s) => Math.abs(s.start - start) < 0.08 || (s.start <= start && s.end > start),
    );
    if (seg) projectStore.selectedSegmentId = seg.id;
  }
</script>

<div class="panel flex min-h-0 flex-col overflow-hidden border-warning/30">
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="flex items-center justify-between gap-2">
      <div>
        <div class="text-sm font-semibold text-surface-100">Supervisión</div>
        <div class="text-[10px] text-surface-500">
          Solo excepciones · el resto ya lo decidió la política
        </div>
      </div>
      {#if pending.length > 0}
        <span
          class="rounded-full bg-warning/20 px-2 py-0.5 text-[11px] font-bold text-warning"
        >
          {pending.length}
        </span>
      {:else}
        <span class="rounded-full bg-keep/20 px-2 py-0.5 text-[11px] font-bold text-keep">0</span>
      {/if}
    </div>

    {#if stats}
      <div class="mt-2 flex flex-wrap gap-2 text-[10px] text-surface-400">
        <span class="rounded bg-surface-800 px-1.5 py-0.5"
          >Auto-corte <strong class="text-cut">{stats.autoCutCount}</strong></span
        >
        <span class="rounded bg-surface-800 px-1.5 py-0.5"
          >−{formatTime(stats.autoRemovedDuration)} auto</span
        >
        <span class="rounded bg-surface-800 px-1.5 py-0.5"
          >Final ≈ {formatTime(stats.outputDuration)}</span
        >
      </div>
    {/if}
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if !projectStore.analysisRun}
      <p class="p-3 text-center text-xs text-surface-500">
        Abre un video para analizar. La IA auto-corta silencios de alta confianza.
      </p>
    {:else if pending.length === 0}
      <div class="flex flex-col items-center gap-2 p-4 text-center">
        <div class="text-2xl text-keep">✓</div>
        <p class="text-sm font-medium text-surface-200">Sin excepciones</p>
        <p class="text-[11px] text-surface-500">
          Todo lo dudoso está resuelto o no hubo casos límite.<br />
          Oye el resultado y exporta.
        </p>
        <button
          type="button"
          class="btn-primary mt-1 text-xs"
          onclick={() => window.dispatchEvent(new CustomEvent("vigilcut:listen-result"))}
        >
          ▶ Oír video cortado
        </button>
      </div>
    {:else}
      <ul class="space-y-2">
        {#each pending as ex (ex.id)}
          <li class="rounded-xl border border-warning/25 bg-surface-950/60 p-2.5">
            <button
              type="button"
              class="w-full text-left"
              onclick={() => seekTo(ex.span.start)}
            >
              <div class="flex items-center justify-between gap-2">
                <span class="font-mono text-xs text-surface-200">
                  {formatTime(ex.span.start)} – {formatTime(ex.span.end)}
                </span>
                <span class="text-[10px] font-semibold text-warning">
                  {(ex.confidence * 100).toFixed(0)}%
                </span>
              </div>
              <p class="mt-1 text-[11px] leading-snug text-surface-400">{ex.rationale}</p>
            </button>
            <div class="mt-2 flex gap-2">
              <button
                type="button"
                class="btn flex-1 bg-cut py-1.5 text-xs font-bold text-white hover:bg-red-400"
                disabled={projectStore.busy}
                onclick={() => projectStore.resolveException(ex.id, true)}
              >
                Cortar
              </button>
              <button
                type="button"
                class="btn flex-1 bg-keep py-1.5 text-xs font-bold text-white hover:bg-green-400"
                disabled={projectStore.busy}
                onclick={() => projectStore.resolveException(ex.id, false)}
              >
                Conservar
              </button>
            </div>
          </li>
        {/each}
      </ul>

      <div class="mt-3 flex gap-2 border-t border-surface-800 pt-2">
        <button
          type="button"
          class="btn-ghost flex-1 text-[11px]"
          disabled={projectStore.busy}
          onclick={() => projectStore.resolveAllExceptions(true)}
        >
          Cortar todas
        </button>
        <button
          type="button"
          class="btn-ghost flex-1 text-[11px]"
          disabled={projectStore.busy}
          onclick={() => projectStore.resolveAllExceptions(false)}
        >
          Conservar todas
        </button>
      </div>
    {/if}
  </div>
</div>
