import type {
  AnalysisRun,
  ExportEstimate,
  MediaInfo,
  ProcessingPreset,
  Project,
  Segment,
  SegmentDecision,
  SilenceDetectionOptions,
  WaveformData,
} from "$lib/types";
import { DEFAULT_SILENCE_OPTIONS, segmentDuration } from "$lib/types";
import * as api from "$lib/utils/tauri";

/** Reactive project / timeline state using Svelte 5 runes. */
class ProjectStore {
  project = $state<Project | null>(null);
  mediaPath = $state<string | null>(null);
  media = $state<MediaInfo | null>(null);
  segments = $state<Segment[]>([]);
  waveform = $state<WaveformData | null>(null);
  presets = $state<ProcessingPreset[]>([]);
  activePresetId = $state("default");
  silenceOptions = $state<SilenceDetectionOptions>({ ...DEFAULT_SILENCE_OPTIONS });

  currentTime = $state(0);
  isPlaying = $state(false);
  /**
   * original = fuente completa
   * edited   = previsualiza el resultado (salta tramos CUT)
   */
  previewMode = $state<"original" | "edited">("edited");
  keepRanges = $state<[number, number][]>([]);
  estimate = $state<ExportEstimate | null>(null);

  /** @deprecated use previewMode === 'edited' */
  get skipCutsPreview() {
    return this.previewMode === "edited";
  }
  set skipCutsPreview(v: boolean) {
    this.previewMode = v ? "edited" : "original";
  }

  busy = $state(false);
  statusMessage = $state("Listo");
  error = $state<string | null>(null);
  selectedSegmentId = $state<string | null>(null);
  /** Segment ids the user has explicitly decided (review progress). */
  touchedIds = $state<string[]>([]);
  /** Last successful export — drives success panel. */
  lastExport = $state<{
    path: string;
    duration: number;
    keptDuration: number;
    cutDuration: number;
  } | null>(null);
  showExportSuccess = $state(false);
  /** Engine analysis run (events + policy + exceptions + EDL). */
  analysisRun = $state<AnalysisRun | null>(null);
  /** Supervisor mode: focus exceptions, not every segment. */
  supervisorMode = $state(true);

  duration = $derived(this.media?.duration ?? this.segments.at(-1)?.end ?? 0);

  /** Matches export: keep + pending (pending not cut yet). Prefer EDL estimate when fresh. */
  keptDuration = $derived(
    this.estimate?.estimatedDuration != null && this.estimate.estimatedDuration > 0
      ? this.estimate.estimatedDuration
      : this.segments
          .filter((s) => s.decision === "keep" || s.decision === "pending")
          .reduce((a, s) => a + segmentDuration(s), 0),
  );

  cutDuration = $derived(
    this.estimate?.cutDuration != null && this.estimate.cutDuration >= 0
      ? this.estimate.cutDuration
      : this.segments
          .filter((s) => s.decision === "cut")
          .reduce((a, s) => a + segmentDuration(s), 0),
  );

  selectedSegment = $derived(
    this.segments.find((s) => s.id === this.selectedSegmentId) ?? null,
  );

  keepCount = $derived(
    this.segments.filter((s) => s.decision === "keep" || s.decision === "pending").length,
  );
  cutCount = $derived(this.segments.filter((s) => s.decision === "cut").length);
  silenceCount = $derived(this.segments.filter((s) => s.kind === "silence").length);
  autoCutCount = $derived(this.segments.filter((s) => s.autoApplied && s.decision === "cut").length);
  pendingExceptions = $derived(
    this.analysisRun?.exceptions.filter((e) => e.resolution === "pending") ?? [],
  );
  pendingExceptionCount = $derived(this.pendingExceptions.length);
  needsReviewSegments = $derived(this.segments.filter((s) => s.needsReview || s.decision === "pending"));

  selectedIndex = $derived(
    this.selectedSegmentId
      ? this.segments.findIndex((s) => s.id === this.selectedSegmentId)
      : -1,
  );

  reviewPosition = $derived(
    this.segments.length === 0
      ? { current: 0, total: 0 }
      : {
          current: this.selectedIndex >= 0 ? this.selectedIndex + 1 : 1,
          total: this.segments.length,
        },
  );

  reviewedCount = $derived(
    this.touchedIds.filter((id) => this.segments.some((s) => s.id === id)).length,
  );

  fileName = $derived(
    this.mediaPath
      ? (this.mediaPath.split(/[/\\]/).pop() ?? this.mediaPath)
      : null,
  );

  markTouched(id: string) {
    if (!this.touchedIds.includes(id)) {
      this.touchedIds = [...this.touchedIds, id];
    }
  }

  /** Select segment and seek playhead to its start. */
  selectSegment(id: string, seek = true) {
    this.selectedSegmentId = id;
    const seg = this.segments.find((s) => s.id === id);
    if (seg && seek) {
      this.currentTime = seg.start;
    }
  }

  /** After analysis: focus first exception (or first silence if none). */
  focusReviewStart() {
    this.touchedIds = [];
    this.lastExport = null;
    this.showExportSuccess = false;
    if (!this.segments.length) {
      this.selectedSegmentId = null;
      return;
    }
    // Supervisor path: first item that needs human eyes
    const needs = this.segments.find((s) => s.needsReview || s.decision === "pending");
    if (needs) {
      this.selectSegment(needs.id, true);
      return;
    }
    // All auto — select first keep for preview context
    const keep = this.segments.find((s) => s.decision === "keep");
    this.selectSegment((keep ?? this.segments[0]).id, true);
  }

  applyAnalysisRun(run: AnalysisRun) {
    this.analysisRun = run;
    this.segments = run.segments;
    this.keepRanges = run.edl.videoTrack.map((s) => [s.start, s.end] as [number, number]);
    this.estimate = {
      estimatedDuration: run.edl.outputDuration,
      keepRanges: this.keepRanges,
      cutDuration: run.edl.removedDuration,
      sourceDuration: run.duration,
    };
  }

  /** Mark decision and jump to next segment (single review behavior). */
  markAndAdvance(id: string, decision: SegmentDecision) {
    this.markTouched(id);
    this.setDecision(id, decision);
    const idx = this.segments.findIndex((s) => s.id === id);
    if (idx >= 0 && idx < this.segments.length - 1) {
      this.selectSegment(this.segments[idx + 1].id, true);
    }
  }

  /** Toggle keep/cut and advance — same path as ActionBar / list badge. */
  toggleAndAdvance(id: string) {
    const seg = this.segments.find((s) => s.id === id);
    if (!seg) return;
    const next: SegmentDecision = seg.decision === "keep" ? "cut" : "keep";
    this.markAndAdvance(id, next);
  }

  async refreshPresets() {
    try {
      this.presets = await api.listPresets();
    } catch {
      this.presets = [];
    }
  }

  async openMedia(path: string, name?: string) {
    this.busy = true;
    this.error = null;
    this.showExportSuccess = false;
    this.lastExport = null;
    this.analysisRun = null;
    this.statusMessage = "Abriendo video…";
    try {
      this.mediaPath = path;
      if (api.isTauri()) {
        this.media = await api.probeMedia(path);
        this.project = await api.createProject(
          name ?? path.split(/[/\\]/).pop() ?? "Untitled",
          path,
        );
        this.statusMessage = "Analizando (eventos + política)…";
        const run = await api.runAnalysis(path, this.silenceOptions);
        this.applyAnalysisRun(run);
        if (this.project) {
          this.project = {
            ...this.project,
            segments: run.segments,
            media: this.media,
          };
          await api.saveProject(this.project);
        }
        try {
          this.waveform = await api.extractWaveform(path, 80);
        } catch {
          this.waveform = null;
        }
        this.previewMode = "edited";
        this.focusReviewStart();
        const pe = run.stats.pendingExceptionCount;
        const auto = run.stats.autoCutCount;
        this.statusMessage =
          pe > 0
            ? `Auto-cortados ${auto} · ${pe} excepción(es) por revisar`
            : `Auto-cortados ${auto} silencios · sin excepciones · listo para oír y exportar`;
      } else {
        this.media = {
          path,
          duration: 60,
          width: 1920,
          height: 1080,
          fps: 30,
          hasAudio: true,
          hasVideo: true,
          sizeBytes: 0,
        };
        this.segments = api.demoSegments(60);
        this.statusMessage = "Modo demo (sin app de escritorio)";
        await this.refreshKeepRanges();
        this.focusReviewStart();
      }
    } catch (e) {
      this.error = String(e);
      this.statusMessage = "Error al abrir";
    } finally {
      this.busy = false;
    }
  }

  async reanalyze() {
    if (!this.mediaPath) return;
    this.busy = true;
    this.statusMessage = "Re-analizando…";
    try {
      if (api.isTauri()) {
        const run = await api.runAnalysis(this.mediaPath, this.silenceOptions);
        this.applyAnalysisRun(run);
        await this.persistSegments();
        const pe = run.stats.pendingExceptionCount;
        this.statusMessage =
          pe > 0
            ? `Re-análisis · ${pe} excepciones pendientes`
            : `Re-análisis · ${run.stats.autoCutCount} auto-cortes · sin excepciones`;
      } else {
        this.segments = api.demoSegments(this.duration || 60);
        this.statusMessage = "Demo re-analizado";
        await this.refreshKeepRanges();
      }
      this.focusReviewStart();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.busy = false;
    }
  }

  async resolveException(exceptionId: string, accept: boolean) {
    if (!this.analysisRun || !api.isTauri()) {
      // Local fallback: map exception span to segment
      const ex = this.pendingExceptions.find((e) => e.id === exceptionId);
      if (ex) {
        this.segments = this.segments.map((s) => {
          if (Math.abs(s.start - ex.span.start) < 0.05 && Math.abs(s.end - ex.span.end) < 0.05) {
            return {
              ...s,
              decision: accept ? ("cut" as const) : ("keep" as const),
              needsReview: false,
              label: accept ? "aprobado" : "conservar",
            };
          }
          return s;
        });
        void this.refreshKeepRanges();
      }
      return;
    }
    this.busy = true;
    try {
      const run = await api.resolveAnalysisException(
        this.analysisRun.id,
        exceptionId,
        accept ? "accepted" : "rejected",
      );
      this.applyAnalysisRun(run);
      await this.persistSegments();
      this.focusReviewStart();
      this.statusMessage = accept ? "Excepción: cortar" : "Excepción: conservar";
    } catch (e) {
      this.error = String(e);
    } finally {
      this.busy = false;
    }
  }

  async resolveAllExceptions(accept: boolean) {
    if (!this.analysisRun || !api.isTauri()) return;
    this.busy = true;
    try {
      const run = await api.resolveAllExceptions(this.analysisRun.id, accept);
      this.applyAnalysisRun(run);
      await this.persistSegments();
      this.focusReviewStart();
      this.statusMessage = accept ? "Todas las excepciones → cortar" : "Todas → conservar";
    } catch (e) {
      this.error = String(e);
    } finally {
      this.busy = false;
    }
  }

  /** @deprecated prefer toggleAndAdvance for consistent UX */
  toggleSegment(id: string) {
    this.toggleAndAdvance(id);
  }

  recordExportSuccess(path: string, duration: number) {
    this.lastExport = {
      path,
      duration,
      keptDuration: this.keptDuration,
      cutDuration: this.cutDuration,
    };
    this.showExportSuccess = true;
    this.statusMessage = "Exportación lista";
  }

  dismissExportSuccess() {
    this.showExportSuccess = false;
  }

  resetProject() {
    this.project = null;
    this.mediaPath = null;
    this.media = null;
    this.segments = [];
    this.waveform = null;
    this.currentTime = 0;
    this.isPlaying = false;
    this.selectedSegmentId = null;
    this.touchedIds = [];
    this.keepRanges = [];
    this.estimate = null;
    this.lastExport = null;
    this.showExportSuccess = false;
    this.analysisRun = null;
    this.error = null;
    this.statusMessage = "Listo";
  }

  setDecision(id: string, decision: SegmentDecision) {
    this.segments = this.segments.map((s) => (s.id === id ? { ...s, decision } : s));
    void this.refreshKeepRanges();
    void this.persistSegments();
  }

  keepAllSpeech() {
    this.segments = this.segments.map((s) => ({
      ...s,
      decision: s.kind === "silence" ? "cut" : "keep",
    }));
    void this.refreshKeepRanges();
    void this.persistSegments();
  }

  keepEverything() {
    this.segments = this.segments.map((s) => ({ ...s, decision: "keep" }));
    void this.refreshKeepRanges();
    void this.persistSegments();
  }

  async splitSelectedAtPlayhead() {
    if (!this.selectedSegmentId) return;
    try {
      if (api.isTauri()) {
        this.segments = await api.splitSegmentAt(
          this.segments,
          this.selectedSegmentId,
          this.currentTime,
        );
      } else {
        const id = this.selectedSegmentId;
        const t = this.currentTime;
        const next: Segment[] = [];
        for (const s of this.segments) {
          if (s.id !== id || t <= s.start || t >= s.end) {
            next.push(s);
            continue;
          }
          next.push({ ...s, id: `${s.id}-a`, end: t, kind: "manual" });
          next.push({ ...s, id: `${s.id}-b`, start: t, kind: "manual" });
        }
        this.segments = next;
      }
      await this.refreshKeepRanges();
      await this.persistSegments();
    } catch (e) {
      this.error = String(e);
    }
  }

  applyPreset(preset: ProcessingPreset) {
    this.activePresetId = preset.id;
    // Always fill factory knobs even if older presets omit new fields
    this.silenceOptions = {
      ...DEFAULT_SILENCE_OPTIONS,
      ...preset.silence,
      autoApproveMinScore:
        preset.silence.autoApproveMinScore ?? DEFAULT_SILENCE_OPTIONS.autoApproveMinScore,
    };
    if (this.project) {
      this.project = { ...this.project, preset };
    }
  }

  async refreshKeepRanges() {
    try {
      if (api.isTauri() && this.segments.length) {
        // Prefer EDL keep ranges when analysis is in sync (no manual tramo edits)
        const fromEdl = this.analysisRun?.edl?.videoTrack?.map(
          (s) => [s.start, s.end] as [number, number],
        );
        const plan = await api.previewSkipCuts(
          this.segments,
          this.touchedIds.length === 0 && fromEdl?.length ? fromEdl : undefined,
        );
        this.keepRanges = plan.keepRanges;
        this.estimate = await api.estimateExport(
          this.segments,
          this.duration,
          this.keepRanges,
        );
      } else {
        this.keepRanges = this.segments
          .filter((s) => s.decision === "keep" || s.decision === "pending")
          .map((s) => [s.start, s.end] as [number, number]);
        this.estimate = {
          estimatedDuration: this.keptDuration,
          keepRanges: this.keepRanges,
          cutDuration: this.cutDuration,
          sourceDuration: this.duration,
        };
      }
    } catch {
      this.keepRanges = this.segments
        .filter((s) => s.decision === "keep" || s.decision === "pending")
        .map((s) => [s.start, s.end] as [number, number]);
    }
  }

  private async persistSegments() {
    if (!this.project || !api.isTauri()) return;
    try {
      this.project = await api.saveProject({
        ...this.project,
        segments: this.segments,
      });
    } catch {
      /* non-fatal */
    }
  }

  /**
   * Keep ranges for cut-preview playback.
   * Prefer engine EDL ranges when user has not hand-edited tramos; else segment decisions
   * (keep + pending = still in the video until human cuts).
   */
  localKeepRanges(): [number, number][] {
    if (this.keepRanges.length > 0 && this.touchedIds.length === 0) {
      return this.keepRanges.map(([s, e]) => [s, e] as [number, number]);
    }
    const fromSegs = this.segments
      .filter((s) => s.decision === "keep" || s.decision === "pending")
      .map((s) => [s.start, s.end] as [number, number]);
    if (fromSegs.length > 0) return fromSegs;
    return this.keepRanges.length > 0
      ? this.keepRanges.map(([s, e]) => [s, e] as [number, number])
      : [];
  }

  /**
   * Source media time → continuous "edited" timeline (after cuts removed).
   */
  sourceToEdited(sourceTime: number): number {
    const ranges = this.localKeepRanges();
    if (!ranges.length) return 0;
    let edited = 0;
    for (const [s, e] of ranges) {
      if (sourceTime < s) return edited;
      if (sourceTime <= e) return edited + (sourceTime - s);
      edited += e - s;
    }
    return edited;
  }

  /**
   * Edited timeline time → source media time (for scrubbing the cut preview).
   */
  editedToSource(editedTime: number): number {
    const ranges = this.localKeepRanges();
    if (!ranges.length) return 0;
    let remaining = Math.max(0, editedTime);
    for (const [s, e] of ranges) {
      const d = e - s;
      if (remaining <= d + 1e-6) return s + remaining;
      remaining -= d;
    }
    return ranges[ranges.length - 1][1];
  }

  /**
   * While playing the cut preview: stay inside KEEP ranges; jump over CUT gaps.
   * Returns { time, ended } — ended when past the last keep range.
   */
  advanceEditedPlayback(sourceTime: number): { time: number; ended: boolean } {
    const ranges = this.localKeepRanges();
    if (!ranges.length) return { time: sourceTime, ended: true };

    for (let i = 0; i < ranges.length; i++) {
      const [s, e] = ranges[i];
      // In a cut gap before this keep → jump into it
      if (sourceTime < s) return { time: s, ended: false };
      // Still inside this keep
      if (sourceTime < e - 0.04) return { time: sourceTime, ended: false };
      // Past this keep → try next range in loop
    }
    // Past the last keep
    const last = ranges[ranges.length - 1];
    return { time: last[1], ended: true };
  }

  /**
   * If playhead is inside a cut region, jump to the next keep start.
   */
  snapPlayheadOverCuts(sourceTime: number): number {
    return this.advanceEditedPlayback(sourceTime).time;
  }
}

export const projectStore = new ProjectStore();
