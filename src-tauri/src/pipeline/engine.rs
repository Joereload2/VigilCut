use std::path::Path;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::models::analysis::{AnalysisRun, AnalysisStats};
use crate::models::edl::{
    EditOp, EditOpKind, Edl, ExceptionItem, ExceptionResolution, PolicyConfig,
};
use crate::models::event::{Event, Span, TYPE_AUDIO_SILENCE, TYPE_AUDIO_SPEECH};
use crate::models::segment::{Segment, SegmentDecision, SegmentKind, SilenceDetectionOptions};
use crate::pipeline::detectors::{run_secondary, DetectorContext};
use crate::pipeline::policy::{apply_policies, effective_removes};
use crate::state::AppState;

/// Progress callback: (stage, message, percent 0..100).
pub type ProgressFn<'a> = dyn FnMut(&str, &str, f64) + Send + 'a;

/// Run full analysis: detect → events → secondary detectors → policy → EDL → segment projection.
pub async fn run_silence_analysis(
    media_path: &Path,
    policy: &PolicyConfig,
) -> AppResult<AnalysisRun> {
    run_silence_analysis_with_progress(media_path, policy, &mut |_, _, _| {}).await
}

pub async fn run_silence_analysis_with_progress(
    media_path: &Path,
    policy: &PolicyConfig,
    on_progress: &mut ProgressFn<'_>,
) -> AppResult<AnalysisRun> {
    let run_id = AnalysisRun::new_id();
    on_progress("probe", "Leyendo vídeo…", 4.0);
    let ffmpeg = Ffmpeg::new()?;
    let info = ffmpeg.probe(media_path).await?;
    let duration = info.duration;
    let path_str = media_path.to_string_lossy().into_owned();

    // Warm feature cache (16 kHz mono) for Silero / Whisper / future detectors
    on_progress("audio", "Extrayendo audio 16 kHz…", 12.0);
    let wav_path = match crate::pipeline::features::ensure_audio_16k(media_path).await {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::debug!("feature cache wav skip: {e}");
            None
        }
    };

    on_progress("vad", "Detectando silencios (VAD)…", 35.0);
    let (method, silence_ranges) = detect_silence_ranges(media_path, policy).await?;

    // Build alternating speech/silence events covering [0, duration]
    let mut events = ranges_to_events(&run_id, duration, &silence_ranges, &method, policy);

    // Whisper only when explicitly requested (slow on long files)
    let mut caption_srt: Option<std::path::PathBuf> = None;
    if policy.prefer_whisper {
        on_progress("whisper", "Whisper (subtítulos / muletillas)…", 55.0);
        match crate::pipeline::detectors::whisper_cli::try_generate_srt(media_path).await {
            Ok(Some(cap)) => {
                tracing::info!("Whisper captions via {}", cap.method);
                caption_srt = Some(cap.srt_path.clone());
            }
            Ok(None) => {
                tracing::debug!("No whisper CLI on PATH — skip captions/fillers");
            }
            Err(e) => tracing::warn!("Whisper failed: {e}"),
        }
    } else {
        tracing::debug!("Whisper skipped (prefer_whisper=false)");
    }

    on_progress("detectors", "Capítulos, breath, shorts…", 72.0);
    // Secondary detectors: breath, chapters, shorts, fillers (registry)
    {
        let mut ctx = DetectorContext {
            run_id: &run_id,
            media_path,
            duration,
            policy,
            events: &mut events,
            wav_path: wav_path.as_deref(),
            srt_path: caption_srt.as_deref(),
        };
        run_secondary(&mut ctx);
    }

    on_progress("policy", "Aplicando política de cortes…", 88.0);

    // Policies: event_type → auto-cut / exception (registry)
    let outcome = apply_policies(&events, policy);
    let edit_ops = outcome.ops;
    let exceptions = outcome.exceptions;

    // EDL is the export source of truth
    let remove_spans = effective_removes(&edit_ops, &exceptions);
    let edl = Edl::from_remove_spans(&path_str, duration, &remove_spans);

    let segments = project_segments(duration, &events, &edit_ops, &exceptions);

    let silence_event_count = events
        .iter()
        .filter(|e| e.event_type == TYPE_AUDIO_SILENCE)
        .count();
    let auto_cut_count = edit_ops.iter().filter(|o| o.auto_applied).count();
    let pending_exception_count = exceptions.iter().filter(|e| e.is_pending()).count();

    let speech_duration: f64 = events
        .iter()
        .filter(|e| e.event_type == TYPE_AUDIO_SPEECH)
        .map(|e| e.span.duration())
        .sum();
    let silence_duration: f64 = events
        .iter()
        .filter(|e| e.event_type == TYPE_AUDIO_SILENCE)
        .map(|e| e.span.duration())
        .sum();
    let auto_removed_duration: f64 = edit_ops
        .iter()
        .filter(|o| o.auto_applied && o.op == EditOpKind::RemoveSpan)
        .map(|o| o.span.duration())
        .sum();

    let mut run = AnalysisRun {
        id: run_id,
        media_path: path_str,
        duration,
        method,
        policy: policy.clone(),
        events,
        edit_ops,
        exceptions,
        edl,
        segments,
        stats: AnalysisStats {
            event_count: 0, // set below
            silence_event_count,
            auto_cut_count,
            exception_count: 0,
            pending_exception_count,
            speech_duration,
            silence_duration,
            auto_removed_duration,
            output_duration: 0.0,
        },
        artifacts: Vec::new(),
    }
    .with_stats_filled();

    // Stash caption path as pseudo-artifact for later copy on export
    if let Some(srt) = caption_srt {
        run.artifacts.push(crate::models::artifacts::ArtifactRef {
            kind: "captions_srt_cache".into(),
            path: srt.to_string_lossy().into_owned(),
            label: Some("Captions (cache)".into()),
        });
    }

    on_progress("done", "Análisis listo", 100.0);
    Ok(run)
}

trait WithStats {
    fn with_stats_filled(self) -> Self;
}

impl WithStats for AnalysisRun {
    fn with_stats_filled(mut self) -> Self {
        self.stats.event_count = self.events.len();
        self.stats.exception_count = self.exceptions.len();
        self.stats.pending_exception_count =
            self.exceptions.iter().filter(|e| e.is_pending()).count();
        self.stats.output_duration = self.edl.output_duration;
        self
    }
}

async fn detect_silence_ranges(
    media_path: &Path,
    policy: &PolicyConfig,
) -> AppResult<(String, Vec<(f64, f64)>)> {
    let silero_model = AppState::models_dir()
        .ok()
        .map(|d| d.join("silero_vad.onnx"));
    let silero_available = silero_model
        .as_ref()
        .map(|p| p.is_file())
        .unwrap_or(false);

    if policy.prefer_silero && silero_available {
        match crate::pipeline::detectors::detect_silences_silero(
            media_path,
            policy.min_silence_duration,
            policy.threshold,
        )
        .await
        {
            Ok(ranges) => {
                tracing::info!("Silero VAD OK ({} ranges)", ranges.len());
                return Ok(("silero_vad".into(), ranges));
            }
            Err(e) => {
                tracing::warn!("Silero VAD failed, falling back to FFmpeg: {e}");
            }
        }
    }

    let ffmpeg = Ffmpeg::new()?;
    let noise_db = threshold_to_noise_db(policy.threshold);
    let ranges = ffmpeg
        .detect_silences_ffmpeg(media_path, noise_db, policy.min_silence_duration)
        .await?;
    Ok(("ffmpeg_silencedetect".into(), ranges))
}

fn threshold_to_noise_db(threshold: f64) -> f64 {
    let t = threshold.clamp(0.05, 0.95);
    -50.0 + t * 30.0
}

/// Convert raw silence ranges into speech/silence events with scores.
fn ranges_to_events(
    run_id: &str,
    duration: f64,
    silences: &[(f64, f64)],
    method: &str,
    policy: &PolicyConfig,
) -> Vec<Event> {
    let mut cleaned: Vec<(f64, f64)> = silences
        .iter()
        .map(|(s, e)| {
            let start = (s + policy.padding).min(*e);
            let end = (e - policy.padding).max(start);
            (start.max(0.0), end.min(duration))
        })
        .filter(|(s, e)| e - s >= policy.min_silence_duration)
        .collect();

    cleaned.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (s, e) in cleaned {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }

    let mut events = Vec::new();
    let mut cursor = 0.0_f64;

    for (s, e) in &merged {
        if *s > cursor + 0.01 {
            events.push(
                Event::new(
                    run_id,
                    TYPE_AUDIO_SPEECH,
                    method,
                    Span::new(cursor, *s),
                    0.9,
                    serde_json::json!({ "kind": "speech" }),
                )
                .with_tag("keep_default"),
            );
        }

        let dur = e - s;
        // Heuristic score: longer clear silences score higher; ffmpeg base ~0.78
        let score = silence_confidence(dur, method);
        let mut ev = Event::new(
            run_id,
            TYPE_AUDIO_SILENCE,
            method,
            Span::new(*s, *e),
            score,
            serde_json::json!({
                "kind": "silence",
                "duration": dur,
                "method": method,
            }),
        )
        .with_tag("removable_candidate");
        if score >= policy.auto_approve_min_score {
            ev = ev.with_tag("auto_cut_eligible");
        } else {
            ev = ev.with_tag("needs_review");
        }
        events.push(ev);
        cursor = *e;
    }

    if cursor < duration - 0.01 {
        events.push(
            Event::new(
                run_id,
                TYPE_AUDIO_SPEECH,
                method,
                Span::new(cursor, duration),
                0.9,
                serde_json::json!({ "kind": "speech" }),
            )
            .with_tag("keep_default"),
        );
    }

    if events.is_empty() && duration > 0.0 {
        events.push(
            Event::new(
                run_id,
                TYPE_AUDIO_SPEECH,
                method,
                Span::new(0.0, duration),
                1.0,
                serde_json::json!({ "kind": "speech" }),
            )
            .with_tag("keep_default"),
        );
    }

    events
}

fn silence_confidence(duration: f64, method: &str) -> f64 {
    let base = if method == "silero_vad" {
        0.93
    } else if method.contains("silero") {
        0.88
    } else {
        0.84 // ffmpeg silencedetect — good enough for factory auto-cut
    };
    // Longer silences are more likely true gaps; very short ones are riskier
    let boost = ((duration - 0.5) / 2.0).clamp(0.0, 0.10);
    let penalty = if duration < 0.55 { 0.08 } else { 0.0 };
    (base + boost - penalty).clamp(0.5, 0.98)
}

/// Project events + decisions into Segment[] for the supervision UI (derived view).
fn project_segments(
    duration: f64,
    events: &[Event],
    ops: &[EditOp],
    exceptions: &[ExceptionItem],
) -> Vec<Segment> {
    let auto_cut_ids: std::collections::HashSet<&str> = ops
        .iter()
        .filter(|o| o.auto_applied)
        .flat_map(|o| o.source_event_ids.iter().map(|s| s.as_str()))
        .collect();

    let mut exception_by_event: std::collections::HashMap<&str, &ExceptionItem> =
        std::collections::HashMap::new();
    for ex in exceptions {
        for eid in &ex.event_ids {
            exception_by_event.insert(eid.as_str(), ex);
        }
    }

    let mut segments = Vec::new();
    for ev in events {
        let mut seg = if ev.event_type == TYPE_AUDIO_SILENCE {
            Segment::new(
                ev.span.start,
                ev.span.end,
                SegmentKind::Silence,
                SegmentDecision::Keep, // default; may override
            )
        } else {
            Segment::new(
                ev.span.start,
                ev.span.end,
                SegmentKind::Speech,
                SegmentDecision::Keep,
            )
        };
        seg.confidence = ev.score;
        seg.event_id = Some(ev.id.clone());

        if ev.event_type == TYPE_AUDIO_SILENCE {
            if auto_cut_ids.contains(ev.id.as_str()) {
                seg.decision = SegmentDecision::Cut;
                seg.auto_applied = true;
                seg.needs_review = false;
                seg.label = Some("auto".into());
            } else if let Some(ex) = exception_by_event.get(ev.id.as_str()) {
                match ex.resolution {
                    ExceptionResolution::Pending => {
                        seg.decision = SegmentDecision::Pending;
                        seg.needs_review = true;
                        seg.auto_applied = false;
                        seg.label = Some("revisar".into());
                    }
                    ExceptionResolution::Accepted => {
                        seg.decision = SegmentDecision::Cut;
                        seg.needs_review = false;
                        seg.label = Some("aprobado".into());
                    }
                    ExceptionResolution::Rejected => {
                        seg.decision = SegmentDecision::Keep;
                        seg.needs_review = false;
                        seg.label = Some("conservar".into());
                    }
                }
            }
        }

        segments.push(seg);
    }

    if segments.is_empty() && duration > 0.0 {
        segments.push(Segment::new(
            0.0,
            duration,
            SegmentKind::Speech,
            SegmentDecision::Keep,
        ));
    }

    segments
}

/// Rebuild EDL + segments after human resolves exceptions.
pub fn recompile_run(mut run: AnalysisRun) -> AnalysisRun {
    let remove = effective_removes(&run.edit_ops, &run.exceptions);
    run.edl = Edl::from_remove_spans(&run.media_path, run.duration, &remove);
    run.segments = project_segments(run.duration, &run.events, &run.edit_ops, &run.exceptions);
    run.stats.pending_exception_count = run.exceptions.iter().filter(|e| e.is_pending()).count();
    run.stats.output_duration = run.edl.output_duration;
    run.stats.auto_removed_duration = run.edl.removed_duration;
    run
}

pub fn resolve_exception(mut run: AnalysisRun, exception_id: &str, accept: bool) -> AnalysisRun {
    for ex in &mut run.exceptions {
        if ex.id == exception_id {
            ex.resolution = if accept {
                ExceptionResolution::Accepted
            } else {
                ExceptionResolution::Rejected
            };
        }
    }
    recompile_run(run)
}

pub fn accept_all_exceptions(mut run: AnalysisRun) -> AnalysisRun {
    for ex in &mut run.exceptions {
        if ex.is_pending() {
            ex.resolution = ExceptionResolution::Accepted;
        }
    }
    recompile_run(run)
}

pub fn reject_all_exceptions(mut run: AnalysisRun) -> AnalysisRun {
    for ex in &mut run.exceptions {
        if ex.is_pending() {
            ex.resolution = ExceptionResolution::Rejected;
        }
    }
    recompile_run(run)
}

/// Map SilenceDetectionOptions → PolicyConfig (single knobs surface for UI/CLI).
pub fn policy_from_silence_options(opts: &SilenceDetectionOptions) -> PolicyConfig {
    PolicyConfig {
        auto_approve_min_score: opts.auto_approve_min_score.clamp(0.5, 0.99),
        min_silence_duration: opts.min_silence_duration,
        padding: opts.padding,
        threshold: opts.threshold,
        prefer_silero: opts.prefer_silero,
        prefer_whisper: opts.prefer_whisper,
    }
}

/// Bridge: old API shape still works for UI during migration.
pub async fn detect_and_build_segments_legacy(
    media_path: &Path,
    options: &SilenceDetectionOptions,
) -> AppResult<crate::models::segment::SilenceDetectionResult> {
    let policy = policy_from_silence_options(options);
    let run = run_silence_analysis(media_path, &policy).await?;

    // If user wanted auto_cut_silence false, mark all silences pending instead
    let segments = if !options.auto_cut_silence {
        run.segments
            .into_iter()
            .map(|mut s| {
                if s.kind == SegmentKind::Silence {
                    s.decision = SegmentDecision::Pending;
                    s.auto_applied = false;
                    s.needs_review = true;
                }
                s
            })
            .collect()
    } else {
        run.segments
    };

    let speech_duration = segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Speech)
        .map(|s| s.duration())
        .sum();
    let silence_duration = segments
        .iter()
        .filter(|s| s.kind == SegmentKind::Silence)
        .map(|s| s.duration())
        .sum();
    let cut_duration = segments
        .iter()
        .filter(|s| s.decision == SegmentDecision::Cut)
        .map(|s| s.duration())
        .sum();

    Ok(crate::models::segment::SilenceDetectionResult {
        media_path: run.media_path,
        duration: run.duration,
        segments,
        method: run.method,
        speech_duration,
        silence_duration,
        cut_duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::policy::apply_policies;

    #[test]
    fn policy_auto_cuts_high_score() {
        let run_id = "test";
        let events = vec![
            Event::new(
                run_id,
                TYPE_AUDIO_SPEECH,
                "test",
                Span::new(0.0, 2.0),
                0.9,
                serde_json::json!({}),
            ),
            Event::new(
                run_id,
                TYPE_AUDIO_SILENCE,
                "test",
                Span::new(2.0, 3.5),
                0.95,
                serde_json::json!({}),
            ),
            Event::new(
                run_id,
                TYPE_AUDIO_SPEECH,
                "test",
                Span::new(3.5, 10.0),
                0.9,
                serde_json::json!({}),
            ),
        ];
        let policy = PolicyConfig {
            auto_approve_min_score: 0.80,
            ..Default::default()
        };
        let out = apply_policies(&events, &policy);
        assert_eq!(out.ops.len(), 1);
        assert!(out.ops[0].auto_applied);
        assert!(out.exceptions.is_empty());
    }

    #[test]
    fn policy_exception_on_low_score() {
        let events = vec![Event::new(
            "r",
            TYPE_AUDIO_SILENCE,
            "test",
            Span::new(1.0, 1.6),
            0.70,
            serde_json::json!({}),
        )];
        let policy = PolicyConfig::default();
        let out = apply_policies(&events, &policy);
        assert!(out.ops.is_empty());
        assert_eq!(out.exceptions.len(), 1);
        assert!(out.exceptions[0].is_pending());
    }

    #[test]
    fn edl_removes_gap() {
        let edl = Edl::from_remove_spans("x.mp4", 10.0, &[(2.0, 3.0)]);
        assert!((edl.output_duration - 9.0).abs() < 0.05);
        assert_eq!(edl.video_track.len(), 2);
    }

    #[test]
    fn recompile_after_accept_updates_edl() {
        let events = vec![Event::new(
            "r",
            TYPE_AUDIO_SILENCE,
            "t",
            Span::new(2.0, 3.0),
            0.5,
            serde_json::json!({}),
        )];
        let policy = PolicyConfig::default();
        let out = apply_policies(&events, &policy);
        assert_eq!(out.exceptions.len(), 1);
        let mut run = AnalysisRun {
            id: "r".into(),
            media_path: "x.mp4".into(),
            duration: 10.0,
            method: "test".into(),
            policy,
            events,
            edit_ops: out.ops,
            exceptions: out.exceptions,
            edl: Edl::from_remove_spans("x.mp4", 10.0, &[]),
            segments: vec![],
            stats: AnalysisStats::default(),
            artifacts: vec![],
        };
        run = recompile_run(run);
        // pending → keep full-ish
        assert!((run.edl.output_duration - 10.0).abs() < 0.1);
        let ex_id = run.exceptions[0].id.clone();
        run = resolve_exception(run, &ex_id, true);
        assert!((run.edl.output_duration - 9.0).abs() < 0.1);
    }
}
