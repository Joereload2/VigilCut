<script lang="ts">
  import type { StationGuide } from "$lib/vnext/presentation/stations";
  import { STATION_STEP_NAMES } from "$lib/vnext/presentation/stations";

  let {
    guide,
    projectTitle = null as string | null,
  }: {
    guide: StationGuide;
    projectTitle?: string | null;
  } = $props();

  const steps = $derived(
    Array.from({ length: guide.stepTotal }, (_, i) => ({
      n: i + 1,
      name: STATION_STEP_NAMES[i] ?? `${i + 1}`,
      active: guide.stepIndex === i + 1,
      done: guide.stepIndex > i + 1,
    })),
  );
</script>

<header
  class="flex shrink-0 flex-col gap-2 border-b border-surface-800/80 bg-surface-950/95 px-3 py-2 sm:px-4"
  data-testid="station-header"
>
  <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-[10px] font-bold uppercase tracking-wider text-vigil-400">
          {guide.stationName}
        </span>
        <span class="text-[10px] text-surface-600">·</span>
        <span class="text-[10px] text-surface-500">{guide.stepLabel}</span>
        {#if projectTitle}
          <span
            class="max-w-[12rem] truncate text-[11px] font-medium text-surface-400"
            title={projectTitle}
          >
            · {projectTitle}
          </span>
        {/if}
      </div>
      <p class="mt-0.5 text-sm font-semibold leading-snug text-white">{guide.objective}</p>
    </div>
    <div class="flex shrink-0 flex-col items-end gap-0.5 text-right">
      <span
        class="rounded-full border border-surface-700 bg-surface-900 px-2.5 py-0.5 text-[10px] font-medium text-surface-100"
        data-testid="station-status"
      >
        {guide.statusLabel}
      </span>
      {#if guide.remaining && guide.remaining !== "—"}
        <span class="max-w-[12rem] text-[10px] text-surface-500" data-testid="station-remaining">
          {guide.remaining}
        </span>
      {/if}
    </div>
  </div>

  {#if guide.stepIndex > 0}
    <ol
      class="hidden items-center gap-0.5 sm:flex"
      aria-label="Progreso"
      data-testid="station-steps"
    >
      {#each steps as s (s.n)}
        <li class="flex min-w-0 flex-1 flex-col items-center gap-0.5">
          <div class="flex w-full items-center gap-0.5">
            <span
              class="flex h-5 w-5 shrink-0 items-center justify-center rounded-full text-[9px] font-bold
                {s.active
                ? 'bg-vigil-600 text-white'
                : s.done
                  ? 'bg-keep/20 text-keep'
                  : 'bg-surface-800 text-surface-500'}"
              aria-current={s.active ? "step" : undefined}
            >
              {s.done ? "✓" : s.n}
            </span>
            {#if s.n < guide.stepTotal}
              <span
                class="h-0.5 min-w-0 flex-1 rounded {s.done ? 'bg-keep/40' : 'bg-surface-800'}"
              ></span>
            {/if}
          </div>
          <span
            class="w-full truncate text-center text-[9px]
              {s.active ? 'font-semibold text-vigil-300' : 'text-surface-600'}"
          >
            {s.name}
          </span>
        </li>
      {/each}
    </ol>
  {/if}
</header>
