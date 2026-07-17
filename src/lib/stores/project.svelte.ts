import type {
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
  skipCutsPreview = $state(true);
  keepRanges = $state<[number, number][]>([]);
  estimate = $state<ExportEstimate | null>(null);

  busy = $state(false);
  statusMessage = $state("Listo / Ready");
  error = $state<string | null>(null);
  selectedSegmentId = $state<string | null>(null);

  duration = $derived(this.media?.duration ?? this.segments.at(-1)?.end ?? 0);

  keptDuration = $derived(
    this.segments.filter((s) => s.decision === "keep").reduce((a, s) => a + segmentDuration(s), 0),
  );

  cutDuration = $derived(
    this.segments.filter((s) => s.decision === "cut").reduce((a, s) => a + segmentDuration(s), 0),
  );

  selectedSegment = $derived(
    this.segments.find((s) => s.id === this.selectedSegmentId) ?? null,
  );

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
    this.statusMessage = "Abriendo medio / Opening media…";
    try {
      this.mediaPath = path;
      if (api.isTauri()) {
        this.media = await api.probeMedia(path);
        this.project = await api.createProject(
          name ?? path.split(/[/\\]/).pop() ?? "Untitled",
          path,
        );
        this.statusMessage = "Detectando silencios / Detecting silences…";
        const result = await api.detectSilences(path, this.silenceOptions);
        this.segments = result.segments;
        if (this.project) {
          this.project = {
            ...this.project,
            segments: result.segments,
            media: this.media,
          };
          await api.saveProject(this.project);
        }
        try {
          this.waveform = await api.extractWaveform(path, 80);
        } catch {
          this.waveform = null;
        }
        await this.refreshKeepRanges();
        this.statusMessage = `Listo — ${result.method} · ${result.segments.length} segmentos`;
      } else {
        // Browser mock
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
        this.statusMessage = "Modo demo (sin Tauri) / Demo mode";
        await this.refreshKeepRanges();
      }
      this.currentTime = 0;
    } catch (e) {
      this.error = String(e);
      this.statusMessage = "Error";
    } finally {
      this.busy = false;
    }
  }

  async reanalyze() {
    if (!this.mediaPath) return;
    this.busy = true;
    this.statusMessage = "Re-analizando / Re-analyzing…";
    try {
      if (api.isTauri()) {
        const result = await api.detectSilences(this.mediaPath, this.silenceOptions);
        this.segments = result.segments;
        await this.persistSegments();
        this.statusMessage = `Re-análisis listo · ${result.method}`;
      } else {
        this.segments = api.demoSegments(this.duration || 60);
        this.statusMessage = "Demo re-analizado";
      }
      await this.refreshKeepRanges();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.busy = false;
    }
  }

  toggleSegment(id: string) {
    this.segments = this.segments.map((s) => {
      if (s.id !== id) return s;
      const decision: SegmentDecision =
        s.decision === "keep" ? "cut" : s.decision === "cut" ? "keep" : "keep";
      return { ...s, decision };
    });
    void this.refreshKeepRanges();
    void this.persistSegments();
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
    this.silenceOptions = { ...preset.silence };
    if (this.project) {
      this.project = { ...this.project, preset };
    }
  }

  async refreshKeepRanges() {
    try {
      if (api.isTauri() && this.segments.length) {
        const plan = await api.previewSkipCuts(this.segments);
        this.keepRanges = plan.keepRanges;
        this.estimate = await api.estimateExport(this.segments, this.duration);
      } else {
        this.keepRanges = this.segments
          .filter((s) => s.decision === "keep")
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
        .filter((s) => s.decision === "keep")
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
   * Map source time → "edited" timeline time when skip-cuts preview is on.
   */
  sourceToEdited(sourceTime: number): number {
    if (!this.skipCutsPreview || this.keepRanges.length === 0) return sourceTime;
    let edited = 0;
    for (const [s, e] of this.keepRanges) {
      if (sourceTime < s) return edited;
      if (sourceTime <= e) return edited + (sourceTime - s);
      edited += e - s;
    }
    return edited;
  }

  /**
   * If playhead is inside a cut region, jump to the next keep start.
   */
  snapPlayheadOverCuts(sourceTime: number): number {
    if (!this.skipCutsPreview || this.keepRanges.length === 0) return sourceTime;
    for (const [s, e] of this.keepRanges) {
      if (sourceTime >= s && sourceTime < e) return sourceTime;
      if (sourceTime < s) return s;
    }
    return this.keepRanges.at(-1)?.[1] ?? sourceTime;
  }
}

export const projectStore = new ProjectStore();
