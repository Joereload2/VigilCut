<script lang="ts">
  import { projectStore } from "$lib/stores/project.svelte";
  import * as api from "$lib/utils/tauri";

  type Seg = {
    id?: string;
    text: string;
    span: { start: number; end: number };
  };

  type SemanticEv = {
    id: string;
    label: string;
    terms?: string[];
    score: number;
    kind?: string;
    sourceSpan?: { start: number; end: number };
    outputSpan?: { start: number; end: number } | null;
  };

  type Suggestion = {
    id: string;
    assetId: string;
    matchScore: number;
    matchReasons: string[];
    status: string;
    assetTitle?: string;
    outputSpan: { start: number; end: number };
    sourceSpan: { start: number; end: number };
  };

  type Asset = {
    id: string;
    title: string;
    concepts: string[];
    tags: string[];
    thumbnailPath?: string;
    timesUsed?: number;
    licenseStatus?: string;
    status?: string;
  };

  const STOP = new Set([
    "el","la","los","las","un","una","unos","unas","de","del","al","a","en","y","o",
    "que","se","es","son","por","para","con","sin","como","más","muy","ya","lo","su",
    "sus","me","te","nos","les","le","mi","tu","si","no","esto","esta","ese","esa",
    "hay","fue","ser","está","están","porque","cuando","donde","qué","cuál","the","and",
    "of","to","is","are","in","on","for","with","that","this","it","was","be","as","at",
  ]);

  let busy = $state(false);
  let error = $state<string | null>(null);
  let session = $state<{
    transcript?: {
      segments?: Seg[];
      status?: string;
      warnings?: string[];
      engine?: string;
    };
    semanticEvents?: SemanticEv[];
    suggestions?: Suggestion[];
    plan?: { placements?: unknown[]; warnings?: string[]; version?: number };
    planPath?: string;
    timeMap?: { sourceDuration: number; outputDuration: number };
  } | null>(null);
  let assets = $state<Asset[]>([]);
  let showSourceTime = $state(false);
  let transcriptQuery = $state("");
  let lastMessage = $state("");
  let lastArtifacts = $state<string[]>([]);
  let explicitSrt = $state<string | null>(null);
  let whisper = $state<{
    available: boolean;
    kind: string;
    detail: string;
    installHint: string;
  } | null>(null);
  let installingWhisper = $state(false);

  /** User-selected keyword from transcript / chips */
  let selectedWord = $state<string | null>(null);
  let selectedSeg = $state<Seg | null>(null);
  let manualKeyword = $state("");

  async function refreshAssets() {
    try {
      assets = (await api.visualListAssets(null, 80)) as Asset[];
    } catch (e) {
      console.warn(e);
    }
  }

  async function refreshWhisperStatus() {
    try {
      whisper = await api.visualWhisperStatus();
    } catch {
      whisper = {
        available: false,
        kind: "none",
        detail: "No se pudo consultar Whisper",
        installHint: "npm run setup:whisper",
      };
    }
  }

  function applyEnrichmentResult(res: {
    transcript: NonNullable<typeof session>["transcript"];
    semanticEvents?: SemanticEv[];
    suggestions: Suggestion[];
    plan: NonNullable<typeof session>["plan"];
    planPath?: string;
    timeMap?: { sourceDuration: number; outputDuration: number };
    transcriptArtifacts?: [string, string][];
  }) {
    session = {
      transcript: res.transcript,
      semanticEvents: res.semanticEvents ?? [],
      suggestions: res.suggestions,
      plan: res.plan,
      planPath: res.planPath,
      timeMap: res.timeMap,
    };
    lastArtifacts = (res.transcriptArtifacts || []).map(([, p]) => p);
    const nSeg = res.transcript?.segments?.length ?? 0;
    const nSug = res.suggestions?.length ?? 0;
    lastMessage = `Texto: ${nSeg} frases · ${nSug} sugerencias · motor ${res.transcript?.engine ?? "?"}`;
    if (res.plan?.warnings?.length) {
      lastMessage += ` · ${res.plan.warnings[0]}`;
    }
    if (nSeg === 0 && res.transcript?.warnings?.[0]) {
      error = res.transcript.warnings[0];
    }
    projectStore.statusMessage = lastMessage;
  }

  async function runEnrichment(forceWhisper = false) {
    if (!projectStore.mediaPath) {
      error = "Abre un video primero";
      return;
    }
    busy = true;
    error = null;
    projectStore.busy = true;
    projectStore.clearProgress();
    projectStore.setProgress(
      5,
      forceWhisper ? "Transcribiendo con Whisper…" : "Cargando texto…",
      forceWhisper ? "whisper" : "load",
    );
    try {
      const res = (await api.visualRunEnrichment(
        projectStore.mediaPath,
        projectStore.analysisRun?.id ?? null,
        forceWhisper ? null : explicitSrt,
        forceWhisper,
      )) as {
        transcript: NonNullable<typeof session>["transcript"];
        semanticEvents?: SemanticEv[];
        suggestions: Suggestion[];
        plan: NonNullable<typeof session>["plan"];
        planPath?: string;
        timeMap?: { sourceDuration: number; outputDuration: number };
        transcriptArtifacts?: [string, string][];
      };
      applyEnrichmentResult(res);
      projectStore.setProgress(100, "Listo", "done");
    } catch (e) {
      error = String(e);
      projectStore.error = String(e);
    } finally {
      busy = false;
      projectStore.busy = false;
      projectStore.clearProgress();
    }
  }

  /** Explicit primary action: always runs Whisper (not a checkbox). */
  async function runWhisper() {
    if (!projectStore.mediaPath) {
      error = "Abre un video primero";
      return;
    }
    await refreshWhisperStatus();
    if (!whisper?.available) {
      error =
        "Whisper no está instalado. Pulsa «Instalar Whisper» o en terminal: npm run setup:whisper";
      return;
    }
    busy = true;
    error = null;
    projectStore.busy = true;
    projectStore.clearProgress();
    projectStore.setProgress(3, "Iniciando Whisper…", "whisper");
    try {
      const res = (await api.visualTranscribeWhisper(
        projectStore.mediaPath,
        projectStore.analysisRun?.id ?? null,
      )) as {
        transcript: NonNullable<typeof session>["transcript"];
        semanticEvents?: SemanticEv[];
        suggestions: Suggestion[];
        plan: NonNullable<typeof session>["plan"];
        planPath?: string;
        timeMap?: { sourceDuration: number; outputDuration: number };
        transcriptArtifacts?: [string, string][];
      };
      applyEnrichmentResult(res);
      projectStore.setProgress(100, "Transcripción lista", "done");
      if ((res.transcript?.segments?.length ?? 0) > 0) {
        lastMessage = `Whisper OK (${res.transcript?.engine}) · ${res.transcript?.segments?.length} frases`;
      }
    } catch (e) {
      error = String(e);
      projectStore.error = String(e);
    } finally {
      busy = false;
      projectStore.busy = false;
      projectStore.clearProgress();
      await refreshWhisperStatus();
    }
  }

  async function installWhisper() {
    if (!api.isTauri()) return;
    installingWhisper = true;
    error = null;
    projectStore.busy = true;
    projectStore.statusMessage = "Instalando openai-whisper (pip)…";
    try {
      const msg = await api.visualInstallWhisper();
      lastMessage = msg;
      await refreshWhisperStatus();
      projectStore.statusMessage = msg;
    } catch (e) {
      error = String(e);
    } finally {
      installingWhisper = false;
      projectStore.busy = false;
    }
  }

  async function pickSrt() {
    if (!api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Subtítulos", extensions: ["srt", "vtt"] }],
        title: "Importar transcripción SRT/VTT",
      });
      if (typeof p !== "string") return;
      explicitSrt = p;
      lastMessage = `SRT seleccionado: ${p.split(/[/\\]/).pop()}`;
      await runEnrichment(false);
    } catch (e) {
      error = String(e);
    }
  }

  function selectWord(word: string, seg?: Seg | null) {
    const w = word.trim().toLowerCase();
    if (w.length < 2) return;
    selectedWord = w;
    selectedSeg = seg ?? null;
    manualKeyword = w;
  }

  async function importImageForSelection() {
    const concept = (selectedWord || manualKeyword).trim().toLowerCase();
    if (!concept) {
      error = "Selecciona una palabra del texto o escribe un concepto";
      return;
    }
    if (!projectStore.mediaPath) {
      error = "Abre un video primero";
      return;
    }
    if (!api.isTauri()) return;

    // Need a moment on the timeline (selected phrase or current playhead)
    let srcStart = selectedSeg?.span.start;
    let srcEnd = selectedSeg?.span.end;
    if (srcStart == null || srcEnd == null) {
      const t = projectStore.currentTime || 0;
      srcStart = Math.max(0, t - 0.5);
      srcEnd = t + 3.5;
    }

    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Imagen", extensions: ["jpg", "jpeg", "png", "webp"] }],
        title: `Imagen para: ${concept} (${srcStart.toFixed(1)}s)`,
      });
      if (typeof p !== "string") return;

      // Attach without re-running Whisper / wiping transcript
      const res = (await api.visualAttachImage({
        mediaPath: projectStore.mediaPath,
        analysisRunId: projectStore.analysisRun?.id ?? null,
        path: p,
        concept,
        sourceStart: srcStart,
        sourceEnd: srcEnd,
      })) as {
        message?: string;
        transcript?: NonNullable<typeof session>["transcript"];
        suggestions?: Suggestion[];
        plan?: NonNullable<typeof session>["plan"];
        timeMap?: { sourceDuration: number; outputDuration: number };
        suggestion?: Suggestion;
      };

      await refreshAssets();
      // Merge into session — never drop transcript
      session = {
        transcript: res.transcript ?? session?.transcript,
        semanticEvents: session?.semanticEvents,
        suggestions: res.suggestions ?? session?.suggestions ?? [],
        plan: res.plan ?? session?.plan,
        planPath: session?.planPath,
        timeMap: res.timeMap ?? session?.timeMap,
      };
      lastMessage =
        res.message ||
        `Imagen «${concept}» adherida al video (transcripción conservada)`;
      projectStore.statusMessage = lastMessage;
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  async function importImageGeneric() {
    // If a word is selected, attach to that moment; else library-only import
    if (selectedWord || manualKeyword.trim()) {
      await importImageForSelection();
      return;
    }
    if (!api.isTauri()) return;
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const p = await open({
        multiple: false,
        filters: [{ name: "Imagen", extensions: ["jpg", "jpeg", "png", "webp"] }],
      });
      if (typeof p !== "string") return;
      const def = "concepto";
      const raw = prompt("Conceptos (coma). Primera = principal", def) ?? def;
      const concepts = raw
        .split(",")
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean);
      await api.visualImportImage(p, concepts[0] ?? null, concepts, concepts);
      await refreshAssets();
      lastMessage = `Imagen en biblioteca · ${concepts.join(", ")}. Selecciona una palabra del texto y «Añadir imagen» para adherirla al video.`;
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
      const def = selectedWord || "general";
      const raw = prompt("Conceptos para toda la carpeta (coma)", def) ?? def;
      const concepts = raw
        .split(",")
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean);
      const res = (await api.visualImportFolder(p, concepts, concepts, false)) as {
        scanned: number;
        imported: number;
        duplicates: number;
        failed: number;
      };
      await refreshAssets();
      lastMessage = `Carpeta: ${res.imported} nuevas · ${res.duplicates} dup · ${res.failed} fallos (transcripción intacta)`;
      // Do NOT re-run enrichment — that used to wipe Whisper text
    } catch (e) {
      error = String(e);
    }
  }

  async function exportTranscript() {
    if (!api.isTauri() || !session?.transcript?.segments?.length) {
      error = "Carga o genera la transcripción primero";
      return;
    }
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const dir = await open({
        directory: true,
        multiple: false,
        title: "Carpeta para TXT/SRT/JSON",
      });
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
          ? "Imagen aceptada en el VisualPlan"
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
      lastMessage = `Render visual → ${path}`;
      projectStore.statusMessage = lastMessage;
      projectStore.recordExportSuccess(path, projectStore.keptDuration);
      await refreshAssets();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  /** Tokenize a segment into display tokens (words + separators). */
  function tokenizeDisplay(text: string): { t: string; word: boolean }[] {
    const out: { t: string; word: boolean }[] = [];
    const re = /([A-Za-zÁÉÍÓÚÜÑáéíóúüñ0-9]+)|([^A-Za-zÁÉÍÓÚÜÑáéíóúüñ0-9]+)/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      if (m[1]) out.push({ t: m[1], word: true });
      else if (m[2]) out.push({ t: m[2], word: false });
    }
    return out;
  }

  function isKeywordCandidate(w: string): boolean {
    const l = w.toLowerCase();
    if (l.length < 4) return false;
    if (STOP.has(l)) return false;
    if (/^\d+$/.test(l)) return false;
    return true;
  }

  /** Keywords from engine + local freq on transcript */
  const keywordList = $derived.by(() => {
    const map = new Map<string, { word: string; score: number; count: number }>();
    for (const ev of session?.semanticEvents || []) {
      const label = (ev.label || "").toLowerCase();
      if (!label || label.length < 3) continue;
      const prev = map.get(label);
      const sc = ev.score ?? 0.5;
      if (!prev || sc > prev.score) {
        map.set(label, { word: label, score: sc, count: (prev?.count ?? 0) + 1 });
      } else {
        prev.count += 1;
      }
      for (const t of ev.terms || []) {
        const tl = t.toLowerCase();
        if (tl.length < 4 || STOP.has(tl)) continue;
        if (!map.has(tl)) map.set(tl, { word: tl, score: sc * 0.85, count: 1 });
      }
    }
    for (const seg of session?.transcript?.segments || []) {
      for (const tok of tokenizeDisplay(seg.text)) {
        if (!tok.word) continue;
        const l = tok.t.toLowerCase();
        if (!isKeywordCandidate(l)) continue;
        const prev = map.get(l);
        if (prev) prev.count += 1;
        else map.set(l, { word: l, score: 0.4, count: 1 });
      }
    }
    return [...map.values()]
      .sort((a, b) => b.score - a.score || b.count - a.count)
      .slice(0, 36);
  });

  const keywordSet = $derived(new Set(keywordList.map((k) => k.word)));

  const filteredSegments = $derived.by(() => {
    const segs = session?.transcript?.segments || [];
    const q = transcriptQuery.trim().toLowerCase();
    if (!q) return segs;
    return segs.filter((s) => s.text.toLowerCase().includes(q));
  });

  const acceptedCount = $derived(
    session?.suggestions?.filter((s) => s.status === "accepted").length ?? 0,
  );

  const hasText = $derived((session?.transcript?.segments?.length ?? 0) > 0);

  const assetsForSelected = $derived.by(() => {
    const c = (selectedWord || "").toLowerCase();
    if (!c) return [] as Asset[];
    return assets.filter(
      (a) =>
        (a.concepts || []).some((x) => x.toLowerCase().includes(c) || c.includes(x.toLowerCase())) ||
        (a.tags || []).some((x) => x.toLowerCase().includes(c) || c.includes(x.toLowerCase())) ||
        (a.title || "").toLowerCase().includes(c),
    );
  });

  $effect(() => {
    void refreshAssets();
    void refreshWhisperStatus();
  });
</script>

<div class="panel flex min-h-0 flex-col overflow-hidden border-sky-800/30">
  <div class="border-b border-surface-800 px-3 py-2">
    <div class="text-sm font-semibold text-surface-100">Visual · Texto e imágenes</div>
    <div class="text-[10px] text-surface-500">
      1) Carga el texto · 2) Pulsa una palabra · 3) Añade imagen · 4) Acepta sugerencias · 5) Render
    </div>
    <div class="mt-2 flex flex-wrap gap-1.5">
      <button
        type="button"
        class="btn-primary text-xs"
        disabled={busy || installingWhisper}
        onclick={runWhisper}
        title={whisper?.available ? whisper.detail : "Requiere instalar Whisper"}
      >
        {busy ? "Transcribiendo…" : "Transcribir con Whisper"}
      </button>
      <button type="button" class="btn-secondary text-xs" disabled={busy} onclick={pickSrt}
        >Importar SRT…</button
      >
      <button
        type="button"
        class="btn-ghost text-xs"
        disabled={busy}
        onclick={() => runEnrichment(false)}>Solo sugerencias</button
      >
      <button type="button" class="btn-ghost text-xs" onclick={importImageGeneric}>+ Imagen</button>
      <button type="button" class="btn-ghost text-xs" onclick={importFolder}>+ Carpeta</button>
      <button
        type="button"
        class="btn-ghost text-xs"
        disabled={!hasText}
        onclick={exportTranscript}>Exportar TXT/SRT</button
      >
      <button type="button" class="btn-ghost text-xs" disabled={busy} onclick={renderPlan}
        >Render plan</button
      >
    </div>
    <div class="mt-1.5 flex flex-wrap items-center gap-2 text-[10px]">
      {#if whisper?.available}
        <span class="rounded-full border border-keep/40 bg-keep/10 px-2 py-0.5 text-keep">
          Whisper listo · {whisper.kind}
        </span>
      {:else}
        <span class="rounded-full border border-amber-600/40 bg-amber-950/40 px-2 py-0.5 text-amber-200">
          Whisper no instalado
        </span>
        <button
          type="button"
          class="btn-secondary text-[10px]"
          disabled={installingWhisper || busy}
          onclick={installWhisper}
        >
          {installingWhisper ? "Instalando…" : "Instalar Whisper"}
        </button>
        <span class="text-surface-600" title={whisper?.installHint || ""}
          >o: npm run setup:whisper</span
        >
      {/if}
    </div>
  </div>

  {#if error}
    <div class="border-b border-cut/30 bg-cut/10 px-3 py-1.5 text-[11px] text-cut">{error}</div>
  {/if}
  {#if lastMessage}
    <div class="border-b border-surface-800 px-3 py-1 text-[10px] text-surface-400">{lastMessage}</div>
  {/if}

  <!-- Sticky selection bar -->
  <div
    class="flex flex-wrap items-center gap-2 border-b border-sky-900/40 bg-sky-950/30 px-3 py-2"
  >
    <span class="text-[10px] font-semibold uppercase tracking-wide text-sky-300/90">Selección</span>
    {#if selectedWord}
      <span
        class="rounded-full border border-sky-500/50 bg-sky-900/50 px-2.5 py-0.5 text-xs font-semibold text-sky-100"
      >
        {selectedWord}
      </span>
      {#if selectedSeg}
        <span class="font-mono text-[9px] text-surface-500">
          {selectedSeg.span.start.toFixed(1)}–{selectedSeg.span.end.toFixed(1)}s
        </span>
      {/if}
    {:else}
      <span class="text-[10px] text-surface-500">Pulsa una palabra del texto o un chip abajo</span>
    {/if}
    <input
      type="text"
      class="min-w-[7rem] flex-1 rounded border border-surface-700 bg-surface-950 px-2 py-1 text-[11px] text-surface-100"
      placeholder="O escribe un concepto…"
      bind:value={manualKeyword}
      onkeydown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          if (manualKeyword.trim()) selectWord(manualKeyword.trim());
        }
      }}
    />
    <button
      type="button"
      class="btn-primary text-xs"
      disabled={!(selectedWord || manualKeyword.trim())}
      onclick={importImageForSelection}
      title="Importa la imagen y la pega en el VisualPlan en el momento de la frase"
    >
      Añadir y adherir al video
    </button>
  </div>

  <div class="min-h-0 flex-1 space-y-3 overflow-y-auto p-2">
    <!-- TRANSCRIPT FIRST -->
    <section>
      <div class="mb-1 flex items-center justify-between gap-2">
        <div class="text-[11px] font-semibold text-surface-300">
          Lo que se dice
          {#if hasText}
            <span class="font-normal text-surface-500"
              >({session?.transcript?.segments?.length} frases)</span
            >
          {/if}
        </div>
        <label class="flex items-center gap-1 text-[9px] text-surface-500">
          <input type="checkbox" class="accent-vigil-500" bind:checked={showSourceTime} />
          tiempos fuente
        </label>
      </div>

      {#if !session}
        <div
          class="rounded-xl border border-dashed border-surface-700 bg-surface-950/60 px-3 py-4 text-center"
        >
          <p class="text-[11px] text-surface-400">
            Pulsa <strong class="text-surface-200">Transcribir con Whisper</strong> o
            <strong class="text-surface-200">Importar SRT</strong> para ver el texto del video.
          </p>
          <div class="mt-3 flex flex-wrap justify-center gap-2">
            <button
              type="button"
              class="btn-primary text-xs"
              disabled={busy || installingWhisper}
              onclick={runWhisper}>Transcribir con Whisper</button
            >
            <button type="button" class="btn-secondary text-xs" onclick={pickSrt}
              >Importar SRT…</button
            >
          </div>
          <p class="mt-2 text-[10px] text-surface-600">
            Luego toca una palabra y usa «Añadir imagen».
          </p>
        </div>
      {:else if !hasText}
        <div class="rounded-xl border border-amber-800/40 bg-amber-950/20 px-3 py-3">
          <p class="text-[11px] text-amber-200/90">
            Sin texto todavía. Usa el botón verde <strong>Transcribir con Whisper</strong> o importa
            un <strong>.srt</strong>.
          </p>
          <div class="mt-2 flex flex-wrap gap-1.5">
            <button
              type="button"
              class="btn-primary text-[10px]"
              disabled={busy || installingWhisper}
              onclick={runWhisper}
            >
              {busy ? "…" : "Transcribir con Whisper"}
            </button>
            <button type="button" class="btn-secondary text-[10px]" onclick={pickSrt}
              >Importar SRT…</button
            >
            {#if !whisper?.available}
              <button
                type="button"
                class="btn-ghost text-[10px]"
                disabled={installingWhisper}
                onclick={installWhisper}
              >
                {installingWhisper ? "Instalando…" : "Instalar Whisper"}
              </button>
            {/if}
          </div>
          {#if session.transcript?.warnings?.[0]}
            <p class="mt-2 text-[9px] text-surface-500">{session.transcript.warnings[0]}</p>
          {/if}
        </div>
      {:else}
        {#if session.timeMap}
          <p class="mb-1 text-[9px] text-surface-500">
            {session.transcript?.engine || "texto"} · fuente
            {session.timeMap.sourceDuration.toFixed(0)}s → salida
            {session.timeMap.outputDuration.toFixed(0)}s
          </p>
        {/if}
        <input
          type="search"
          class="mb-1.5 w-full rounded border border-surface-800 bg-surface-950 px-2 py-1 text-[10px] text-surface-200"
          placeholder="Buscar en el texto…"
          bind:value={transcriptQuery}
        />
        <div
          class="max-h-52 space-y-2 overflow-y-auto rounded-xl border border-surface-800 bg-surface-950/90 p-2"
        >
          {#each filteredSegments as seg, i (seg.id ?? i)}
            <div
              class="rounded-lg px-1.5 py-1 text-[11px] leading-relaxed
                {selectedSeg === seg ? 'bg-sky-950/50 ring-1 ring-sky-700/50' : ''}"
            >
              <div class="mb-0.5 font-mono text-[9px] text-surface-600">
                {seg.span.start.toFixed(1)}–{seg.span.end.toFixed(1)}s
                {#if showSourceTime}<span class="text-surface-700"> fuente</span>{/if}
              </div>
              <p class="text-surface-200">
                {#each tokenizeDisplay(seg.text) as tok}
                  {#if tok.word}
                    {@const low = tok.t.toLowerCase()}
                    {@const isKw = keywordSet.has(low) || isKeywordCandidate(low)}
                    <button
                      type="button"
                      class="rounded px-0.5 transition
                        {selectedWord === low
                        ? 'bg-sky-500 text-white'
                        : isKw
                          ? 'bg-sky-900/40 text-sky-100 hover:bg-sky-700/50'
                          : 'text-surface-300 hover:bg-surface-800 hover:text-white'}"
                      title={isKw
                        ? `Palabra clave — clic para añadir imagen a «${low}»`
                        : `Clic para usar «${low}» como concepto`}
                      onclick={() => selectWord(tok.t, seg)}
                    >{tok.t}</button
                    >
                  {:else}<span class="text-surface-400">{tok.t}</span>{/if}
                {/each}
              </p>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <!-- KEYWORD CHIPS -->
    {#if keywordList.length > 0}
      <section>
        <div class="mb-1 text-[11px] font-semibold text-surface-300">
          Palabras clave ({keywordList.length})
        </div>
        <p class="mb-1.5 text-[9px] text-surface-500">
          Destacadas del texto. Clic = seleccionar · luego «Añadir imagen».
        </p>
        <div class="flex flex-wrap gap-1">
          {#each keywordList as k (k.word)}
            <button
              type="button"
              class="rounded-full border px-2 py-0.5 text-[10px] font-medium transition
                {selectedWord === k.word
                ? 'border-sky-400 bg-sky-600 text-white'
                : 'border-surface-700 bg-surface-900 text-surface-300 hover:border-sky-600 hover:text-sky-100'}"
              onclick={() => selectWord(k.word)}
              title={`score ${Math.round(k.score * 100)} · ${k.count}×`}
            >
              {k.word}
            </button>
          {/each}
        </div>
      </section>
    {/if}

    <!-- LIBRARY filtered by selection -->
    <section>
      <div class="mb-1 text-[11px] font-semibold text-surface-300">
        Biblioteca
        {#if selectedWord && assetsForSelected.length}
          <span class="font-normal text-sky-400/90"
            >· {assetsForSelected.length} para «{selectedWord}»</span
          >
        {:else}
          <span class="font-normal text-surface-500">({assets.length})</span>
        {/if}
      </div>
      {#if assets.length === 0}
        <p class="text-[10px] text-surface-500">
          Aún no hay imágenes. Selecciona una palabra y pulsa <strong>Añadir imagen</strong>. No se
          descarga de Internet; el original no se modifica.
        </p>
      {:else}
        {@const list = selectedWord && assetsForSelected.length ? assetsForSelected : assets}
        <ul class="space-y-1">
          {#each list.slice(0, 10) as a (a.id)}
            <li class="rounded-lg border border-surface-800 bg-surface-950/50 px-2 py-1.5 text-[10px]">
              <div class="font-medium text-surface-200">{a.title}</div>
              <div class="text-surface-500">
                {(a.concepts || []).join(", ") || (a.tags || []).join(", ") || "—"}
                {#if a.timesUsed}· {a.timesUsed}×{/if}
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <!-- SUGGESTIONS -->
    <section>
      <div class="mb-1 text-[11px] font-semibold text-surface-300">
        Sugerencias ({session?.suggestions?.length ?? 0})
        {#if acceptedCount}
          <span class="font-normal text-keep"> · {acceptedCount} aceptada(s)</span>
        {/if}
      </div>
      {#if !session?.suggestions?.length}
        <p class="text-[10px] text-surface-500">
          Cuando haya texto + imágenes con el mismo concepto, aparecerán aquí para aceptar o
          rechazar.
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
                  · {s.outputSpan.start.toFixed(1)}–{s.outputSpan.end.toFixed(1)}s · score
                  {Math.round(s.matchScore * 100)}
                </span>
              </div>
              <div class="mt-0.5 text-surface-500">{(s.matchReasons || []).join(" · ")}</div>
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
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if session?.plan}
      <section class="rounded-lg border border-surface-800 bg-surface-950/40 p-2 text-[10px] text-surface-400">
        <p class="font-semibold text-surface-300">
          VisualPlan · v{session.plan.version ?? 1}
        </p>
        <p>
          Imágenes en el video: {session.plan.placements?.length ?? 0} (aceptadas / adheridas)
        </p>
        <p class="mt-1 text-[9px] text-surface-600">
          Usa Render plan sobre el MP4 cortado para verlas en el export.
        </p>
      </section>
    {/if}
  </div>
</div>
