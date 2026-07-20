<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";

  let busy = $state(false);
  let error = $state<string | null>(null);
  let session = $state<{
    transcript?: { segments?: { text: string; span: { start: number; end: number } }[]; status?: string; warnings?: string[] };
    suggestions?: {
      id: string;
      assetId: string;
      matchScore: number;
      matchReasons: string[];
      status: string;
      assetTitle?: string;
      outputSpan: { start: number; end: number };
      sourceSpan: { start: number; end: number };
    }[];
    plan?: { placements?: unknown[]; warnings?: string[] };
  } | null>(null);
  let assets = $state<{ id: string; title: string; concepts: string[]; tags: string[]; thumbnailPath?: string }[]>([]);
  let preferWhisper = $state(false);
  let lastMessage = $state("");

  async function refreshAssets() {
    try {
      assets = (await api.visualListAssets(null, 50)) as typeof assets;
    } catch (e) {
      console.warn(e);
    }
  }

  async function runEnrichment() {
    if (!projectStore.mediaPath) {
      error = "Abre un video primero";
      return;
    }
    busy = true;
    error = null;
    projectStore.busy = true;
    projectStore.statusMessage = "Generando transcripción y sugerencias visuales…";
    try {
      const res = (await api.visualRunEnrichment(
        projectStore.mediaPath,
        projectStore.analysisRun?.id ?? null,
        null,
        preferWhisper,
      )) as {
        transcript: typeof session extends null ? never : NonNullable<typeof session>["transcript"];
        suggestions: NonNullable<typeof session>["suggestions"];
        plan: NonNullable<typeof session>["plan"];
      };
      session = {
        transcript: res.transcript,
        suggestions: res.suggestions,
        plan: res.plan,
      };
      lastMessage = `Sugerencias: ${res.suggestions?.length ?? 0}`;
      projectStore.statusMessage = lastMessage;
    } catch (e) {
      error = String(e);
      projectStore.error = String(e);
    } finally {
      busy = false;
      projectStore.busy = false;
    }
  }

  async function importImage() {
    if (!api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Imagen", extensions: ["jpg", "jpeg", "png", "webp"] }],
      });
      if (typeof p !== "string") return;
      const concepts = prompt("Conceptos (separados por coma)", "inflacion,economia")
        ?.split(",")
        .map((s) => s.trim())
        .filter(Boolean) ?? [];
      await api.visualImportImage(p, null, concepts, concepts);
      await refreshAssets();
      lastMessage = "Imagen importada a la biblioteca local";
    } catch (e) {
      error = String(e);
    }
  }

  async function setStatus(id: string, status: string) {
    try {
      const plan = await api.visualSetSuggestionStatus(id, status);
      if (session) {
        session = {
          ...session,
          suggestions: session.suggestions?.map((s) =>
            s.id === id ? { ...s, status } : s,
          ),
          plan: plan as typeof session.plan,
        };
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function renderPlan() {
    if (!projectStore.mediaPath || !api.isTauri()) return;
    // Need cut video path — use last export or ask user
    const cut =
      projectStore.lastExport?.path ||
      (await (async () => {
        const { open } = await import("@tauri-apps/plugin-dialog");
        const p = await open({
          multiple: false,
          filters: [{ name: "Video cortado", extensions: ["mp4"] }],
          title: "Selecciona el MP4 ya cortado (timeline de salida)",
        });
        return typeof p === "string" ? p : null;
      })());
    if (!cut) {
      error = "Exporta primero el video cortado (Silencios) o elige el MP4 editado";
      return;
    }
    const parts = cut.split(/[/\\]/);
    parts.pop();
    const dir = parts.join(cut.includes("\\") ? "\\" : "/") || ".";
    const sep = cut.includes("\\") ? "\\" : "/";
    const out = `${dir}${sep}visual-enriched.mp4`;
    busy = true;
    try {
      const path = await api.visualRenderPlan(cut, out, projectStore.mediaPath);
      lastMessage = `Render visual → ${path}`;
      projectStore.statusMessage = lastMessage;
      projectStore.recordExportSuccess(path, projectStore.keptDuration);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    void refreshAssets();
  });
</script>

<div class="panel flex min-h-0 flex-col overflow-hidden border-amber-800/30">
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="text-sm font-semibold text-surface-100">Visual · Biblioteca</div>
    <div class="text-[10px] text-surface-500">
      Transcripción → conceptos → imágenes → supervisión → render
    </div>
    <div class="mt-2 flex flex-wrap gap-1.5">
      <button type="button" class="btn-primary text-xs" disabled={busy} onclick={runEnrichment}>
        {busy ? "…" : "Generar sugerencias"}
      </button>
      <button type="button" class="btn-secondary text-xs" onclick={importImage}>+ Imagen</button>
      <button type="button" class="btn-ghost text-xs" disabled={busy} onclick={renderPlan}
        >Render plan</button
      >
    </div>
    <label class="mt-2 flex items-center gap-2 text-[10px] text-surface-400">
      <input type="checkbox" class="accent-vigil-500" bind:checked={preferWhisper} />
      Whisper si no hay SRT (lento)
    </label>
  </div>

  {#if error}
    <div class="border-b border-cut/30 bg-cut/10 px-3 py-1.5 text-[11px] text-cut">{error}</div>
  {/if}
  {#if lastMessage}
    <div class="border-b border-surface-800 px-3 py-1 text-[10px] text-surface-400">{lastMessage}</div>
  {/if}

  <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-2">
    <section>
      <div class="mb-1 text-[11px] font-semibold text-surface-300">
        Biblioteca ({assets.length})
      </div>
      {#if assets.length === 0}
        <p class="text-[10px] text-surface-500">
          Importa JPG/PNG con conceptos (ej. inflación, alimentos). No se descarga de Internet.
        </p>
      {:else}
        <ul class="space-y-1">
          {#each assets.slice(0, 12) as a (a.id)}
            <li class="rounded-lg border border-surface-800 bg-surface-950/50 px-2 py-1.5 text-[10px]">
              <div class="font-medium text-surface-200">{a.title}</div>
              <div class="text-surface-500">
                {(a.concepts || []).join(", ") || (a.tags || []).join(", ") || "—"}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if session?.transcript}
      <section>
        <div class="mb-1 text-[11px] font-semibold text-surface-300">Transcripción</div>
        <p class="mb-1 text-[9px] text-surface-500">
          Estado: {session.transcript.status}
          {#if session.transcript.warnings?.length}
            · {session.transcript.warnings[0]}
          {/if}
        </p>
        <div class="max-h-32 space-y-1 overflow-y-auto rounded-lg bg-surface-950/80 p-2 text-[10px]">
          {#each (session.transcript.segments || []).slice(0, 20) as seg}
            <div>
              <span class="font-mono text-surface-500"
                >{seg.span.start.toFixed(1)}–{seg.span.end.toFixed(1)}</span
              >
              {seg.text}
            </div>
          {:else}
            <p class="text-surface-500">Vacía — importa .srt o activa Whisper.</p>
          {/each}
        </div>
      </section>
    {/if}

    <section>
      <div class="mb-1 text-[11px] font-semibold text-surface-300">
        Sugerencias ({session?.suggestions?.length ?? 0})
      </div>
      {#if !session?.suggestions?.length}
        <p class="text-[10px] text-surface-500">
          Pulsa Generar tras analizar silencios e importar imágenes con conceptos.
        </p>
      {:else}
        <ul class="space-y-2">
          {#each session.suggestions as s (s.id)}
            <li
              class="rounded-xl border p-2 text-[10px]
                {s.status === 'accepted'
                ? 'border-keep/40 bg-keep/10'
                : s.status === 'rejected'
                  ? 'border-surface-800 opacity-50'
                  : 'border-surface-800 bg-surface-950/50'}"
            >
              <div class="font-semibold text-surface-100">
                {s.assetTitle || s.assetId}
                <span class="font-mono text-surface-500">
                  · out {s.outputSpan.start.toFixed(1)}–{s.outputSpan.end.toFixed(1)}s · score
                  {Math.round(s.matchScore * 100)}
                </span>
              </div>
              <div class="mt-0.5 text-surface-500">{(s.matchReasons || []).join(" · ")}</div>
              <div class="mt-1.5 flex gap-1">
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-keep"
                  onclick={() => setStatus(s.id, "accepted")}>Aceptar</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-cut"
                  onclick={() => setStatus(s.id, "rejected")}>Rechazar</button
                >
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if session?.plan?.placements?.length}
      <section class="text-[10px] text-surface-400">
        Plan visual: {session.plan.placements.length} placement(s) aceptados
      </section>
    {/if}
  </div>
</div>
