//! Orchestrate clipping analysis run.

use std::path::Path;
use std::time::Instant;

use chrono::Utc;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::models::analysis::AnalysisRun;
use crate::models::clipping::{
    ClipReviewStatus, ClippingOptions, ClippingRun, ClippingSummary, TranscriptSourceKind,
    MIN_CLIP_SCORE,
};
use crate::models::edl::PolicyConfig;
use crate::pipeline::clipping::dedupe::dedupe_and_group;
use crate::pipeline::clipping::framing::default_framing_for_media;
use crate::pipeline::clipping::generate::generate_candidates;
use crate::pipeline::clipping::preselect::apply_preselection;
use crate::pipeline::clipping::titles::finalize_clip_titles;
use crate::pipeline::clipping::transcript::{
    cues_from_speech_events, cues_to_semantic_units, load_transcript_cues,
};
use crate::pipeline::engine::{run_silence_analysis_with_progress, ProgressFn};
use crate::state::AppState;

pub async fn run_clipping_analysis(
    media_path: &Path,
    options: ClippingOptions,
) -> AppResult<ClippingRun> {
    run_clipping_analysis_with_progress(media_path, options, None, &mut |_, _, _| {}).await
}

pub async fn run_clipping_analysis_with_progress(
    media_path: &Path,
    options: ClippingOptions,
    // Prefer speech events from this analysis run (avoids re-VAD).
    reuse_analysis: Option<&AnalysisRun>,
    on_progress: &mut ProgressFn<'_>,
) -> AppResult<ClippingRun> {
    let t0 = Instant::now();
    let run_id = ClippingRun::new_id();
    let path_str = media_path.to_string_lossy().into_owned();

    on_progress("probe", "Preparando clipping…", 5.0);
    let ffmpeg = Ffmpeg::new()?;
    let info = ffmpeg.probe(media_path).await?;
    let source_duration = info.duration;
    let framing = default_framing_for_media(&info);

    let mut warnings = Vec::new();
    let mut transcript_source = TranscriptSourceKind::AnalysisSpeechFallback;
    let mut has_real = false;

    // 1) Explicit transcript  2) sidecar  3) Whisper  4) reuse analysis  5) re-VAD
    let mut cues = Vec::new();

    let explicit = options.transcript_path.clone().filter(|p| !p.is_empty());
    let sidecar = find_sidecar_transcript(media_path);

    on_progress("transcript", "Buscando subtítulos…", 15.0);
    for (label, path) in [
        ("explicit", explicit.as_deref()),
        ("sidecar", sidecar.as_deref()),
    ] {
        if !cues.is_empty() {
            break;
        }
        let Some(p) = path else { continue };
        match load_transcript_cues(Path::new(p)) {
            Ok((c, kind)) => {
                transcript_source = kind;
                has_real = true;
                cues = c;
                if label == "sidecar" {
                    warnings.push(format!("Transcripción: {p}"));
                }
            }
            Err(e) => {
                if label == "explicit" {
                    warnings.push(format!("Transcript import failed ({e})"));
                }
            }
        }
    }

    if cues.is_empty() && options.prefer_whisper {
        on_progress("whisper", "Whisper (texto para clips)…", 35.0);
        match crate::pipeline::detectors::whisper_cli::try_generate_srt(media_path).await {
            Ok(Some(cap)) => match load_transcript_cues(&cap.srt_path) {
                Ok((c, _)) => {
                    transcript_source = TranscriptSourceKind::WhisperCli;
                    has_real = true;
                    cues = c;
                    warnings.push(format!("Whisper captions via {}", cap.method));
                }
                Err(e) => warnings.push(format!("Whisper SRT unreadable: {e}")),
            },
            Ok(None) => {}
            Err(e) => warnings.push(format!("Whisper failed: {e}")),
        }
    }

    if cues.is_empty() {
        // Prefer in-memory / disk analysis for same media (opened in Silencios first)
        let reused = reuse_analysis
            .filter(|ar| paths_equal(&ar.media_path, &path_str))
            .cloned()
            .or_else(|| find_cached_analysis_for_media(&path_str));

        if let Some(ar) = reused {
            on_progress("reuse", "Reutilizando análisis de silencios…", 50.0);
            cues = cues_from_speech_events(&ar.events);
            if cues.is_empty() {
                warnings.push("Análisis reutilizado sin bloques de habla".into());
            } else {
                warnings.push(
                    "Sin SRT: clips desde habla del análisis previo (importa .srt para mejor calidad)"
                        .into(),
                );
            }
        } else {
            on_progress("vad", "Detectando habla (sin SRT)…", 50.0);
            let policy = PolicyConfig {
                prefer_silero: true,
                prefer_whisper: false,
                ..PolicyConfig::default()
            };
            match run_silence_analysis_with_progress(media_path, &policy, on_progress).await {
                Ok(ar) => {
                    cues = cues_from_speech_events(&ar.events);
                    if cues.is_empty() {
                        warnings.push("No speech spans detected for clipping".into());
                    } else {
                        warnings.push(
                            "Sin SRT/VTT: candidatos desde bloques de habla (importa subtítulos para mejor calidad)"
                                .into(),
                        );
                    }
                }
                Err(e) => {
                    warnings.push(format!("Speech analysis failed: {e}"));
                }
            }
        }
    }

    on_progress("candidates", "Generando y puntuando clips…", 75.0);
    let (_, _ideal, max_d) = options.resolved_bounds();
    let units = cues_to_semantic_units(&cues, 0.9, max_d * 1.2);

    let mut candidates = generate_candidates(
        &run_id,
        &path_str,
        &units,
        source_duration,
        &options,
        has_real,
        framing,
    );
    candidates = dedupe_and_group(candidates);
    let dropped_weak = candidates
        .iter()
        .filter(|c| c.score < MIN_CLIP_SCORE)
        .count();
    candidates.retain(|c| c.score >= MIN_CLIP_SCORE);
    if dropped_weak > 0 {
        warnings.push(format!(
            "Omitidos {dropped_weak} clips con score < {MIN_CLIP_SCORE:.0}"
        ));
    }
    apply_preselection(&mut candidates, &options);
    candidates.retain(|c| c.score >= MIN_CLIP_SCORE);
    finalize_clip_titles(&mut candidates);

    on_progress("rank", "Clasificando candidatos…", 92.0);

    let preselected = candidates
        .iter()
        .filter(|c| c.status == ClipReviewStatus::Preselected)
        .count();
    let high_confidence = candidates
        .iter()
        .filter(|c| c.is_primary_variant && c.score >= 72.0 && c.confidence >= 0.55)
        .count();
    let needs_review = candidates
        .iter()
        .filter(|c| {
            matches!(
                c.status,
                ClipReviewStatus::Preselected | ClipReviewStatus::Suggested
            ) && c.confidence < 0.6
        })
        .count();
    let discarded = candidates
        .iter()
        .filter(|c| c.status == ClipReviewStatus::Discarded)
        .count();
    let best_score = candidates.iter().map(|c| c.score).fold(0.0_f64, f64::max);
    let selected_total_duration: f64 = candidates
        .iter()
        .filter(|c| c.status == ClipReviewStatus::Preselected && c.is_primary_variant)
        .map(|c| c.duration)
        .sum();

    let candidates_found = candidates
        .iter()
        .filter(|c| {
            c.is_primary_variant
                && c.score >= MIN_CLIP_SCORE
                && !matches!(c.status, ClipReviewStatus::Discarded)
        })
        .count();
    if candidates_found == 0 && !warnings.iter().any(|w| w.contains("score <")) {
        warnings.push(format!(
            "Ningún clip superó el score mínimo ({MIN_CLIP_SCORE:.0})"
        ));
    }

    let analysis_seconds = t0.elapsed().as_secs_f64();
    let summary = ClippingSummary {
        source_duration,
        analysis_seconds,
        candidates_found,
        preselected,
        high_confidence,
        needs_review,
        discarded,
        best_score,
        selected_total_duration,
        transcript_source,
        warnings,
    };

    on_progress("done", "Clips listos", 100.0);

    Ok(ClippingRun {
        id: run_id,
        media_path: path_str,
        source_duration,
        options,
        candidates,
        summary,
        created_at: Utc::now().to_rfc3339(),
    })
}

fn paths_equal(a: &str, b: &str) -> bool {
    let na = a.replace('\\', "/").to_lowercase();
    let nb = b.replace('\\', "/").to_lowercase();
    na == nb
}

/// Latest analysis run on disk for this media path (from silence open).
fn find_cached_analysis_for_media(media_path: &str) -> Option<AnalysisRun> {
    let dir = AppState::cache_dir().ok()?.join("runs");
    if !dir.is_dir() {
        return None;
    }
    let mut best: Option<(std::time::SystemTime, AnalysisRun)> = None;
    let entries = std::fs::read_dir(dir).ok()?;
    for ent in entries.flatten() {
        let path = ent.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let meta = ent.metadata().ok();
        let mtime = meta.and_then(|m| m.modified().ok());
        let data = std::fs::read_to_string(&path).ok()?;
        let run: AnalysisRun = serde_json::from_str(&data).ok()?;
        if !paths_equal(&run.media_path, media_path) {
            continue;
        }
        let t = mtime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &best {
            None => best = Some((t, run)),
            Some((bt, _)) if t > *bt => best = Some((t, run)),
            _ => {}
        }
    }
    best.map(|(_, r)| r)
}

fn find_sidecar_transcript(media_path: &Path) -> Option<String> {
    let stem = media_path.with_extension("");
    for ext in ["srt", "vtt", "SRT", "VTT"] {
        let p = stem.with_extension(ext);
        if p.is_file() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}
