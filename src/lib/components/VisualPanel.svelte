<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";

  let busy = $state(false);
  let error = $state<string | null>(null);
  let session = $state<{
    transcript?: {
      segments?: { text: string; span: { start: number; end: number } }[];
      status?: string;
      warnings?: string[];
      engine?: string;
    };
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
    plan?: { placements?: unknown[]; warnings?: string[]; version?: number };
    planPath?: string;
    timeMap?: { sourceDuration: number; outputDuration: number };
  } | null>(null);
  let assets = $state<
    {
      id: string;
      title: string;
      concepts: string[];
      tags: string[];
      thumbnailPath?: string;
      timesUsed?: number;
      licenseStatus?: string;
      status?: string;
    }[]
  >([]);
  let preferWhisper = $state(false);
  let showSourceTime = $state(false);
  let transcriptQuery = $state("");
  let lastMessage = $state("");
  let lastArtifacts = $state<string[]>([]);

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
        transcript: NonNullable<typeof session>["transcript"];
        suggestions: NonNullable<typeof session>["suggestions"];
        plan: NonNullable<typeof session>["plan"];
        planPath?: string;
        timeMap?: { sourceDuration: number; outputDuration: number };
        transcriptArtifacts?: [string, string][];
      };
      session = {
        transcript: res.transcript,
        suggestions: res.suggestions,
        plan: res.plan,
        planPath: res.planPath,
        timeMap: res.timeMap,
      };
      lastArtifacts = (res.transcriptArtifacts || []).map(([, p]) => p);
      lastMessage = `Sugerencias: ${res.suggestions?.length ?? 0} · plan v${res.plan?.version ?? 1}`;
      if (res.plan?.warnings?.length) {
        lastMessage += ` · ${res.plan.warnings[0]}`;
      }
      projectStore.statusMessage = lastMessage;
    } catch (e) {
      error = String(e);
      projectStore.error = String(e);
    } finally {
      busy = false;
      projectStore.busy = false;
    }
  }

  function promptConcepts(defaultValue = "inflacion,economia"): string[] {
    return (
      prompt("Conceptos (separados por coma)", defaultValue)
        ?.split(",")
        .map((s) => s.trim())
        .filter(Boolean) ?? []
    );
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
      const concepts = promptConcepts();
      await api.visualImportImage(p, null, concepts, concepts);
      await refreshAssets();
      lastMessage = "Imagen importada a la biblioteca local (original intacto)";
    } catch (e) {
      error = String(e);
    }
  }

  async function importFolder() {
    if (!api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({ directory: true, multiple: false });
      if (typeof p !== "string") return;
      const concepts = promptConcepts();
      const res = (await api.visualImportFolder(p, concepts, concepts, false)) as {
        scanned: number;
        imported: number;
        duplicates: number;
        failed: number;
      };
      await refreshAssets();
      lastMessage = `Carpeta: ${res.imported} nuevas · ${res.duplicates} duplicados · ${res.failed} fallos (de ${res.scanned})`;
    } catch (e) {
      error = String(e);
    }
  }

  async function exportTranscript() {
    if (!api.isTauri() || !session?.transcript) {
      error = "Genera la transcripción primero";
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const dir = await open({ directory: true, multiple: false, title: "Carpeta para TXT/SRT/JSON" });
      if (typeof dir !== "string") return;
      const stem =
        projectStore.mediaPath?.split(/[/\\]/).pop()?.replace(/\.[^.]+$/, "") || "transcript";
      const res = (await api.visualExportTranscript(dir, stem)) as {
        artifacts: [string, string][];
      };
      lastArtifacts = (res.artifacts || []).map(([, p]) => p);
      lastMessage = `Transcripción exportada (${lastArtifacts.length} archivos)`;
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
      lastMessage =
        status === "accepted"
          ? "Placement añadido al VisualPlan (no al EDL de cortes)"
          : status === "rejected"
            ? "Sugerencia rechazada"
            : `Estado: ${status}`;
    } catch (e) {
      error = String(e);
    }
  }

  async function renderPlan() {
    if (!projectStore.mediaPath || !api.isTauri()) return;
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
      lastMessage = `Render visual → ${path} (+ manifiesto)`;
      projectStore.statusMessage = lastMessage;
      projectStore.recordExportSuccess(path, projectStore.keptDuration);
      await refreshAssets();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const filteredSegments = $derived.by(() => {
    const segs = session?.transcript?.segments || [];
    const q = transcriptQuery.trim().toLowerCase();
    if (!q) return segs.slice(0, 40);
    return segs.filter((s) => s.text.toLowerCase().includes(q)).slice(0, 40);
  });

  const acceptedCount = $derived(
    session?.suggestions?.filter((s) => s.status === "accepted").length ?? 0,
  );

  $effect(() => {
    void refreshAssets();
  });
</script>

<div class="panel flex min-h-0 flex-col overflow-hidden border-amber-800/30">
  <div class="border-b border-surface-800 px-3 py-2.5">
    <div class="text-sm font-semibold text-surface-100">Visual · Biblioteca</div>
    <div class="text-[10px] text-surface-500">
      Transcripción → conceptos → imágenes → supervisión humana → VisualPlan → render
    </div>
    <div class="mt-2 flex flex-wrap gap-1.5">
      <button type="button" class="btn-primary text-xs" disabled={busy} onclick={runEnrichment}>
        {busy ? "…" : "Generar sugerencias"}
      </button>
      <button type="button" class="btn-secondary text-xs" onclick={importImage}>+ Imagen</button>
      <button type="button" class="btn-secondary text-xs" onclick={importFolder}>+ Carpeta</button>
      <button
        type="button"
        class="btn-ghost text-xs"
        disabled={busy || !session?.transcript}
        onclick={exportTranscript}>Exportar TXT/SRT/JSON</button
      >
      <button type="button" class="btn-ghost text-xs" disabled={busy} onclick={renderPlan}
        >Render plan</button
      >
    </div>
    <label class="mt-2 flex items-center gap-2 text-[10px] text-surface-400">
      <input type="checkbox" class="accent-vigil-500" bind:checked={preferWhisper} />
      Whisper si no hay SRT (lento; el corte de silencios no lo necesita)
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
          Los originales no se modifican.
        </p>
      {:else}
        <ul class="space-y-1">
          {#each assets.slice(0, 12) as a (a.id)}
            <li class="rounded-lg border border-surface-800 bg-surface-950/50 px-2 py-1.5 text-[10px]">
              <div class="flex items-center justify-between gap-2">
                <div class="font-medium text-surface-200">{a.title}</div>
                {#if a.licenseStatus === "unknown"}
                  <span class="text-[9px] text-amber-400">lic. ?</span>
                {/if}
              </div>
              <div class="text-surface-500">
                {(a.concepts || []).join(", ") || (a.tags || []).join(", ") || "—"}
                {#if a.timesUsed}
                  · usada {a.timesUsed}×
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if session?.transcript}
      <section>
        <div class="mb-1 flex items-center justify-between gap-2">
          <div class="text-[11px] font-semibold text-surface-300">Transcripción</div>
          <label class="flex items-center gap-1 text-[9px] text-surface-500">
            <input type="checkbox" class="accent-vigil-500" bind:checked={showSourceTime} />
            tiempos fuente
          </label>
        </div>
        <p class="mb-1 text-[9px] text-surface-500">
          Estado: {session.transcript.status}
          {#if session.transcript.engine}
            · {session.transcript.engine}
          {/if}
          {#if session.timeMap}
            · fuente {session.timeMap.sourceDuration.toFixed(0)}s → salida
            {session.timeMap.outputDuration.toFixed(0)}s
          {/if}
        </p>
        {#if session.transcript.warnings?.length}
          <p class="mb-1 text-[9px] text-amber-400/90">{session.transcript.warnings[0]}</p>
        {/if}
        <input
          type="search"
          class="mb-1 w-full rounded border border-surface-800 bg-surface-950 px-2 py-1 text-[10px] text-surface-200"
          placeholder="Buscar en transcripción…"
          bind:value={transcriptQuery}
        />
        <div class="max-h-36 space-y-1 overflow-y-auto rounded-lg bg-surface-950/80 p-2 text-[10px]">
          {#each filteredSegments as seg}
            <div>
              <span class="font-mono text-surface-500"
                >{seg.span.start.toFixed(1)}–{seg.span.end.toFixed(1)}s
                {#if showSourceTime}
                  <span class="text-surface-600">(fuente)</span>
                {/if}</span
              >
              {seg.text}
            </div>
          {:else}
            <p class="text-surface-500">Vacía — importa .srt o activa Whisper.</p>
          {/each}
        </div>
        {#if lastArtifacts.length}
          <p class="mt-1 truncate text-[9px] text-surface-600" title={lastArtifacts.join("\n")}>
            Artefactos: {lastArtifacts.length} archivo(s)
          </p>
        {/if}
      </section>
    {/if}

    <section>
      <div class="mb-1 text-[11px] font-semibold text-surface-300">
        Sugerencias ({session?.suggestions?.length ?? 0})
        {#if acceptedCount}
          <span class="font-normal text-keep"> · {acceptedCount} aceptada(s)</span>
        {/if}
      </div>
      {#if !session?.suggestions?.length}
        <p class="text-[10px] text-surface-500">
          1) Analiza en Silencios · 2) Importa imágenes con conceptos · 3) Generar sugerencias · 4)
          Acepta/rechaza · 5) Exporta corte · 6) Render plan
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
                  · out {s.outputSpan.start.toFixed(1)}–{s.outputSpan.end.toFixed(1)}s
                  · src {s.sourceSpan.start.toFixed(1)}–{s.sourceSpan.end.toFixed(1)}s · score
                  {Math.round(s.matchScore * 100)}
                </span>
              </div>
              <div class="mt-0.5 text-surface-500" title="Razones heurísticas, no probabilidad">
                {(s.matchReasons || []).join(" · ")}
              </div>
              <div class="mt-1.5 flex gap-1">
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-keep"
                  disabled={s.status === "accepted"}
                  onclick={() => setStatus(s.id, "accepted")}>Aceptar</button
                >
                <button
                  type="button"
                  class="btn-ghost text-[10px] text-cut"
                  disabled={s.status === "rejected"}
                  onclick={() => setStatus(s.id, "rejected")}>Rechazar</button
                >
                {#if s.status === "accepted" || s.status === "rejected"}
                  <button
                    type="button"
                    class="btn-ghost text-[10px] text-surface-400"
                    onclick={() => setStatus(s.id, "suggested")}>Deshacer</button
                  >
                {/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if session?.plan}
      <section class="rounded-lg border border-surface-800 bg-surface-950/40 p-2 text-[10px] text-surface-400">
        <div class="font-semibold text-surface-300">VisualPlan (separado del EDL)</div>
        <div>
          Placements: {session.plan.placements?.length ?? 0} · v{session.plan.version ?? 1}
        </div>
        {#if session.plan.warnings?.length}
          <div class="mt-0.5 text-amber-400/90">{session.plan.warnings.join(" · ")}</div>
        {/if}
        <p class="mt-1 text-[9px] text-surface-600">
          El EDL decide qué del original permanece. El plan decide qué imagen va sobre la salida.
        </p>
      </section>
    {/if}
  </div>
</div>
