<script lang="ts">
  import type { StationGuide } from "$lib/vnext/presentation/stations";

  let { guide }: { guide: StationGuide } = $props();
</script>

<aside
  class="shrink-0 rounded-xl border border-surface-800 bg-surface-900/80 px-3 py-2.5"
  data-testid="station-guide"
  aria-label="Guía de la estación"
>
  <div class="grid gap-2 sm:grid-cols-2 xl:grid-cols-5">
    <div>
      <p class="text-[9px] font-bold uppercase tracking-wide text-surface-500">Qué hago</p>
      <p class="mt-0.5 text-[11px] leading-snug text-surface-200">{guide.youDo}</p>
    </div>
    <div>
      <p class="text-[9px] font-bold uppercase tracking-wide text-surface-500">Qué hace VigilCut</p>
      <p class="mt-0.5 text-[11px] leading-snug text-surface-200">{guide.systemDoes}</p>
    </div>
    <div>
      <p class="text-[9px] font-bold uppercase tracking-wide text-surface-500">Qué falta</p>
      <p class="mt-0.5 text-[11px] leading-snug text-surface-200">{guide.missing}</p>
    </div>
    <div>
      <p class="text-[9px] font-bold uppercase tracking-wide text-surface-500">Cuánto falta</p>
      <p class="mt-0.5 text-[11px] leading-snug text-surface-200">{guide.remaining}</p>
    </div>
    <div class="sm:col-span-2 xl:col-span-1">
      <p class="text-[9px] font-bold uppercase tracking-wide text-vigil-400">Qué debo hacer</p>
      <p class="mt-0.5 text-[12px] font-semibold leading-snug text-white">{guide.nextDo}</p>
    </div>
  </div>
</aside>
