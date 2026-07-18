<script lang="ts">
  import { onMount } from "svelte";
  import * as api from "$lib/utils/tauri";

  interface Props {
    onOpen: () => void;
  }
  let { onOpen }: Props = $props();

  let paths = $state<{ inbox: string; outbox: string } | null>(null);

  onMount(() => {
    if (!api.isTauri()) return;
    void api.getFactoryPaths().then((p) => {
      paths = { inbox: p.inbox, outbox: p.outbox };
    });
  });

  async function openDir(which: string) {
    if (!api.isTauri()) return;
    try {
      const dir = await api.openFactoryFolder(which);
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(dir);
    } catch (e) {
      console.error(e);
    }
  }
</script>

<div class="flex flex-col items-center justify-center gap-6 px-4 py-8 text-center">
  <div
    class="flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-vigil-500 to-emerald-800 text-2xl font-bold text-white shadow-panel"
  >
    V
  </div>

  <div class="max-w-lg">
    <h1 class="text-2xl font-semibold tracking-tight text-white">VigilCut Factory</h1>
    <p class="mt-2 text-sm leading-relaxed text-surface-400">
      Motor local: la IA corta silencios, propone capítulos y shorts. Tú solo supervisas
      excepciones y exportas (o lanzas un lote completo).
    </p>
  </div>

  <div class="flex flex-wrap items-center justify-center gap-3">
    <button
      class="btn-primary px-8 py-3 text-base font-semibold shadow-lg shadow-vigil-950/40"
      onclick={onOpen}
    >
      Abrir un video
    </button>
  </div>

  <ol class="grid w-full max-w-xl grid-cols-1 gap-2 text-left sm:grid-cols-3">
    {#each [
      { n: "1", t: "Analizar", d: "Events + política auto-corte" },
      { n: "2", t: "Excepciones", d: "Solo baja confianza" },
      { n: "3", t: "Artefactos", d: "MP4 · capítulos · shorts · JSON" },
    ] as step}
      <li class="rounded-xl border border-surface-800 bg-surface-900/60 px-3 py-3">
        <div class="text-[10px] font-bold text-vigil-400">PASO {step.n}</div>
        <div class="mt-0.5 text-sm font-medium text-surface-100">{step.t}</div>
        <div class="mt-0.5 text-[11px] text-surface-500">{step.d}</div>
      </li>
    {/each}
  </ol>

  {#if paths}
    <div class="w-full max-w-xl rounded-xl border border-surface-800 bg-surface-900/50 p-3 text-left text-[11px]">
      <div class="mb-2 font-semibold text-surface-300">Carpetas de fábrica</div>
      <div class="space-y-1 font-mono text-surface-500">
        <div class="flex items-center justify-between gap-2">
          <span class="truncate" title={paths.inbox}>inbox: {paths.inbox}</span>
          <button type="button" class="btn-ghost shrink-0 text-[10px]" onclick={() => openDir("inbox")}
            >Abrir</button
          >
        </div>
        <div class="flex items-center justify-between gap-2">
          <span class="truncate" title={paths.outbox}>outbox: {paths.outbox}</span>
          <button type="button" class="btn-ghost shrink-0 text-[10px]" onclick={() => openDir("outbox")}
            >Abrir</button
          >
        </div>
      </div>
      <p class="mt-2 text-surface-600">
        Deja crudos en inbox y usa el panel Lote, o:
        <code class="text-surface-400">npm run cli -- batch inbox outbox</code>
      </p>
    </div>
  {/if}
</div>
