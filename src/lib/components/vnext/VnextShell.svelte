<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/utils/tauri";
  import { vnextUiStore } from "$lib/stores/vnext/uiStore.svelte";
  import { vnextProjectStore } from "$lib/stores/vnext/projectStore.svelte";
  import { vnextJobStore } from "$lib/stores/vnext/jobStore.svelte";
  import { vnextReviewStore } from "$lib/stores/vnext/reviewStore.svelte";
  import type { ProjectSectionV1 } from "$lib/types/vnext/v1";
  let { onOpenLegacy }: { onOpenLegacy?: () => void } = $props();

  const sections: { id: ProjectSectionV1; label: string }[] = [
    { id: "summary", label: "Resumen" },
    { id: "candidates", label: "Candidatos" },
    { id: "review", label: "Revisión" },
    { id: "exports", label: "Exportaciones" },
  ];

  onMount(() => {
    void vnextProjectStore.refreshList();
    void vnextJobStore.refreshQueue();
  });

  async function openMediaAsProject() {
    if (!api.isTauri()) {
      vnextUiStore.setError("Abrí la app de escritorio para crear proyectos vNext");
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Video", extensions: ["mp4", "mov", "mkv", "webm"] }],
      });
      if (typeof p !== "string") return;
      const project = await vnextProjectStore.createFromPath(p);
      await vnextReviewStore.loadForProject(project.id);
      await vnextJobStore.refreshProject(project.id);
      vnextUiStore.setSection("summary");
    } catch (e) {
      vnextUiStore.setError(String(e));
    }
  }

  async function selectProject(id: string) {
    await vnextProjectStore.select(id);
    if (vnextProjectStore.current) {
      await vnextReviewStore.loadForProject(id);
      await vnextJobStore.refreshProject(id);
    }
  }

  const sel = $derived(vnextReviewStore.selected);
</script>

<div class="flex h-full min-h-0 w-full flex-col gap-2 overflow-hidden p-2">
  <!-- Top nav: Proyectos | Cola | Biblioteca | Legacy -->
  <nav class="flex shrink-0 flex-wrap items-center gap-1 rounded-xl border border-surface-800 bg-surface-950 p-1">
    {#each [
      { id: "projects" as const, label: "Proyectos" },
      { id: "queue" as const, label: "Cola" },
      { id: "library" as const, label: "Biblioteca" },
    ] as item}
      <button
        type="button"
        class="rounded-lg px-3 py-1.5 text-xs font-semibold transition
          {vnextUiStore.nav === item.id ? 'bg-vigil-600 text-white' : 'text-surface-400 hover:bg-surface-800'}"
        onclick={() => {
          vnextUiStore.setNav(item.id);
          if (item.id === "queue") void vnextJobStore.refreshQueue();
          if (item.id === "projects") void vnextProjectStore.refreshList();
        }}
      >
        {item.label}
      </button>
    {/each}
    <div class="flex-1"></div>
    <button type="button" class="btn-secondary text-[10px]" onclick={() => onOpenLegacy?.()}>
      Herramientas legacy
    </button>
  </nav>

  {#if vnextUiStore.error}
    <div class="shrink-0 rounded-lg border border-cut/40 bg-cut/10 px-3 py-2 text-xs text-cut">
      {vnextUiStore.error}
      <button type="button" class="ml-2 underline" onclick={() => vnextUiStore.setError(null)}>cerrar</button>
    </div>
  {/if}
  {#if vnextUiStore.statusMessage}
    <p class="shrink-0 text-[11px] text-surface-500">{vnextUiStore.statusMessage}</p>
  {/if}

  <div class="min-h-0 flex-1 overflow-hidden">
    {#if vnextUiStore.nav === "queue"}
      <div class="h-full overflow-y-auto rounded-xl border border-surface-800 p-3">
        <h2 class="mb-2 text-sm font-semibold text-white">Cola de trabajos</h2>
        {#if vnextJobStore.queue.length === 0}
          <p class="text-xs text-surface-500">No hay jobs recientes.</p>
        {:else}
          <ul class="space-y-2">
            {#each vnextJobStore.queue as j (j.id)}
              <li class="rounded-lg border border-surface-800 bg-surface-900/50 p-2 text-[11px]">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="font-mono text-surface-300">{j.kind}</span>
                  <span
                    class="rounded px-1.5 py-0.5 text-[10px]
                      {j.status === 'completed'
                      ? 'bg-keep/20 text-keep'
                      : j.status === 'failed' || j.status === 'interrupted'
                        ? 'bg-cut/20 text-cut'
                        : 'bg-surface-800 text-surface-300'}"
                  >{j.status}</span
                  >
                  <span class="text-surface-500">{j.progressPct.toFixed(0)}%</span>
                  <div class="flex-1"></div>
                  {#if j.status === "failed" || j.status === "interrupted"}
                    <button type="button" class="btn-primary text-[10px]" disabled={vnextUiStore.busy} onclick={() => void vnextJobStore.retry(j.id)}>Reintentar</button>
                  {/if}
                  {#if j.status === "queued" || j.status === "running"}
                    <button type="button" class="btn-ghost text-[10px]" disabled={vnextUiStore.busy} onclick={() => void vnextJobStore.cancel(j.id)}>Cancelar</button>
                  {/if}
                </div>
                <p class="mt-1 truncate font-mono text-[10px] text-surface-600">{j.id}</p>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else if vnextUiStore.nav === "library"}
      <div class="flex h-full items-center justify-center rounded-xl border border-dashed border-surface-700 p-6 text-center">
        <div>
          <p class="text-sm text-surface-300">Biblioteca Visual</p>
          <p class="mt-1 max-w-sm text-xs text-surface-500">
            Sigue disponible como modo legacy. En vNext el foco es el proyecto de cliente y sus entregables.
          </p>
          <button type="button" class="btn-secondary mt-3 text-xs" onclick={() => onOpenLegacy?.()}>Abrir Biblioteca legacy</button>
        </div>
      </div>
    {:else if !vnextProjectStore.current}
      <!-- Project list -->
      <div class="flex h-full min-h-0 flex-col gap-2 overflow-hidden">
        <div class="flex shrink-0 items-center justify-between">
          <h2 class="text-sm font-semibold text-white">Proyectos</h2>
          <button type="button" class="btn-primary text-xs" disabled={vnextUiStore.busy} onclick={() => void openMediaAsProject()}>
            Nuevo desde video
          </button>
        </div>
        {#if vnextProjectStore.loading}
          <p class="text-xs text-surface-500">Cargando…</p>
        {:else if vnextProjectStore.projects.length === 0}
          <div class="flex flex-1 flex-col items-center justify-center rounded-xl border border-dashed border-surface-700 p-8 text-center">
            <p class="text-sm text-surface-300">No hay proyectos vNext todavía</p>
            <p class="mt-1 max-w-md text-xs text-surface-500">
              Creá un proyecto desde un video de cliente: ingesta → candidatos → revisión → render 1080×1920.
            </p>
            <button type="button" class="btn-primary mt-4 text-xs" onclick={() => void openMediaAsProject()}>Abrir video</button>
          </div>
        {:else}
          <ul class="min-h-0 flex-1 space-y-2 overflow-y-auto">
            {#each vnextProjectStore.projects as p (p.id)}
              <li>
                <button
                  type="button"
                  class="w-full rounded-xl border border-surface-800 bg-surface-900/40 p-3 text-left hover:border-vigil-600/50"
                  onclick={() => void selectProject(p.id)}
                >
                  <div class="text-sm font-semibold text-white">{p.title}</div>
                  <div class="mt-0.5 truncate font-mono text-[10px] text-surface-500">{p.sourceMediaPath}</div>
                  <div class="mt-1 text-[10px] text-vigil-300">Siguiente: {p.nextAction ?? "—"}</div>
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {:else}
      <!-- Project detail -->
      {@const project = vnextProjectStore.current}
      <div class="flex h-full min-h-0 flex-col gap-2">
        <div class="flex shrink-0 flex-wrap items-center gap-2">
          <button type="button" class="btn-ghost text-[10px]" onclick={() => vnextProjectStore.clearCurrent()}>← Proyectos</button>
          <h2 class="text-sm font-semibold text-white">{project.title}</h2>
          <span class="rounded-full bg-surface-800 px-2 py-0.5 text-[10px] text-surface-300">{project.nextAction ?? "—"}</span>
          <div class="flex-1"></div>
          {#each sections as s}
            <button
              type="button"
              class="rounded-md px-2 py-1 text-[10px] font-semibold
                {vnextUiStore.projectSection === s.id ? 'bg-amber-600 text-white' : 'bg-surface-800 text-surface-400'}"
              onclick={() => vnextUiStore.setSection(s.id)}
            >{s.label}</button>
          {/each}
        </div>

        <div class="min-h-0 flex-1 overflow-y-auto rounded-xl border border-surface-800 p-3">
          {#if vnextUiStore.projectSection === "summary"}
            <dl class="grid gap-2 text-[11px] sm:grid-cols-2">
              <div><dt class="text-surface-500">Media</dt><dd class="break-all font-mono text-surface-200">{project.sourceMediaPath}</dd></div>
              <div><dt class="text-surface-500">Work dir</dt><dd class="break-all font-mono text-surface-200">{project.workDir}</dd></div>
              <div><dt class="text-surface-500">Privacidad</dt><dd>{project.privacyMode}</dd></div>
              <div><dt class="text-surface-500">Actualizado</dt><dd>{project.updatedAt}</dd></div>
            </dl>
            <div class="mt-4 flex flex-wrap gap-2">
              <button type="button" class="btn-primary text-xs" disabled={vnextUiStore.busy} onclick={() => void vnextReviewStore.generateCandidates(project.id)}>
                Generar candidatos
              </button>
              <button type="button" class="btn-secondary text-xs" onclick={() => vnextUiStore.setSection("candidates")}>Ver candidatos</button>
            </div>
          {:else if vnextUiStore.projectSection === "candidates"}
            {#if vnextReviewStore.candidates.length === 0}
              <p class="text-xs text-surface-500">Sin candidatos. Generá desde Resumen.</p>
            {:else}
              <ul class="space-y-1">
                {#each vnextReviewStore.candidates as c (c.id)}
                  <li>
                    <button
                      type="button"
                      class="flex w-full items-center gap-2 rounded-lg border px-2 py-1.5 text-left text-[11px]
                        {vnextReviewStore.selectedId === c.id ? 'border-amber-500 bg-amber-950/30' : 'border-surface-800'}"
                      onclick={() => {
                        vnextReviewStore.selectCandidate(c.id);
                        vnextUiStore.setSection("review");
                      }}
                    >
                      <span class="font-semibold text-white">{c.title || c.id.slice(0, 8)}</span>
                      <span class="text-surface-500">{c.score.toFixed(0)}</span>
                      <span class="text-surface-400">{c.status}</span>
                      <span class="ml-auto font-mono text-[10px] text-surface-600">{c.start.toFixed(1)}–{c.end.toFixed(1)}s</span>
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          {:else if vnextUiStore.projectSection === "review"}
            {#if !sel}
              <p class="text-xs text-surface-500">Elegí un candidato.</p>
            {:else}
              <div class="grid gap-3 lg:grid-cols-2">
                <div class="min-h-[200px] overflow-hidden rounded-lg border border-surface-800 bg-black">
                  <!-- ShortPlayer uses projectStore media; show text fallback context -->
                  <div class="p-3 text-[11px] text-surface-400">
                    <p class="font-semibold text-white">{sel.title}</p>
                    <p class="mt-1 whitespace-pre-wrap text-surface-300">{sel.transcript || "Sin transcript"}</p>
                    <p class="mt-2 font-mono text-[10px]">{sel.start.toFixed(2)}s → {sel.end.toFixed(2)}s · score {sel.score.toFixed(1)}</p>
                    <p class="mt-2 text-[10px] text-surface-500">
                      Reproducí el tramo en modo Shorts legacy si necesitás preview de video; el estado de revisión ya es durable en vNext.
                    </p>
                  </div>
                </div>
                <div class="space-y-2 text-[11px]">
                  <div class="flex flex-wrap gap-2">
                    <button type="button" class="btn-primary text-[10px]" disabled={vnextUiStore.busy} onclick={() => void vnextReviewStore.approve()}>Aprobar</button>
                    <button type="button" class="btn-ghost text-[10px] text-red-300" disabled={vnextUiStore.busy} onclick={() => void vnextReviewStore.reject()}>Rechazar</button>
                  </div>
                  <label class="block text-surface-400">Inicio (s)
                    <input class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1" type="number" step="0.05" bind:value={vnextReviewStore.draftStart} />
                  </label>
                  <label class="block text-surface-400">Fin (s)
                    <input class="mt-0.5 w-full rounded border border-surface-700 bg-surface-950 px-2 py-1" type="number" step="0.05" bind:value={vnextReviewStore.draftEnd} />
                  </label>
                  <button type="button" class="btn-secondary text-[10px]" disabled={vnextUiStore.busy} onclick={() => void vnextReviewStore.applySpan()}>Guardar tramo</button>
                  {#if vnextReviewStore.draftFraming}
                    <label class="block text-surface-400">Framing center X
                      <input class="mt-0.5 w-full" type="range" min="0" max="1" step="0.01" bind:value={vnextReviewStore.draftFraming.centerX} />
                    </label>
                    <label class="block text-surface-400">Framing center Y
                      <input class="mt-0.5 w-full" type="range" min="0" max="1" step="0.01" bind:value={vnextReviewStore.draftFraming.centerY} />
                    </label>
                    <button type="button" class="btn-secondary text-[10px]" disabled={vnextUiStore.busy} onclick={() => void vnextReviewStore.applyFraming()}>Guardar framing</button>
                  {/if}
                  <label class="block text-surface-400">Cue subtítulo (preset safe_center_bottom_v1)
                    <textarea
                      class="mt-0.5 min-h-16 w-full rounded border border-surface-700 bg-surface-950 p-2"
                      value={vnextReviewStore.cues[0]?.text ?? ""}
                      oninput={(e) => {
                        const text = (e.currentTarget as HTMLTextAreaElement).value;
                        if (!vnextReviewStore.cues[0]) {
                          vnextReviewStore.cues = [
                            {
                              id: "auto-0",
                              startS: 0,
                              endS: Math.max(0.5, sel.duration),
                              text,
                            },
                          ];
                        } else {
                          vnextReviewStore.cues[0].text = text;
                          vnextReviewStore.cues = [...vnextReviewStore.cues];
                        }
                      }}
                    ></textarea>
                  </label>
                  <button type="button" class="btn-primary w-full text-xs" disabled={vnextUiStore.busy || sel.status === "rejected"} onclick={() => void vnextReviewStore.createPlanAndRender(true)}>
                    Render 1080×1920
                  </button>
                </div>
              </div>
            {/if}
          {:else}
            <!-- exports -->
            <h3 class="mb-2 text-xs font-semibold text-white">Artifacts</h3>
            {#if vnextJobStore.artifacts.length === 0}
              <p class="text-xs text-surface-500">Sin artifacts todavía.</p>
            {:else}
              <ul class="space-y-2">
                {#each vnextJobStore.artifacts as a (a.id)}
                  <li class="rounded-lg border border-surface-800 p-2 text-[11px]">
                    <div class="font-semibold text-surface-200">{a.kind} · {a.validationStatus}</div>
                    <div class="mt-0.5 break-all font-mono text-[10px] text-surface-500">{a.path}</div>
                    <div class="mt-1 text-[10px] text-surface-600">{a.byteSize} bytes · {a.sha256.slice(0, 12)}…</div>
                  </li>
                {/each}
              </ul>
            {/if}
            <h3 class="mb-2 mt-4 text-xs font-semibold text-white">Jobs del proyecto</h3>
            <ul class="space-y-1 text-[10px]">
              {#each vnextJobStore.projectJobs as j (j.id)}
                <li class="flex gap-2 border-b border-surface-900 py-1">
                  <span>{j.kind}</span><span>{j.status}</span>
                  {#if j.status === "failed"}
                    <button type="button" class="text-vigil-400 underline" onclick={() => void vnextJobStore.retry(j.id)}>retry</button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
