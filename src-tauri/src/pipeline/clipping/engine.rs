//! Orchestrate clipping analysis run.

use std::path::Path;
use std::time::Instant;

use chrono::Utc;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::models::clipping::{
    ClipReviewStatus, ClippingOptions, ClippingRun, ClippingSummary, TranscriptSourceKind,
    MIN_CLIP_SCORE,
};
use crate::pipeline::clipping::dedupe::dedupe_and_group;
use crate::pipeline::clipping::framing::default_framing_for_media;
use crate::pipeline::clipping::generate::generate_candidates;
use crate::pipeline::clipping::preselect::apply_preselection;
use crate::pipeline::clipping::titles::finalize_clip_titles;
use crate::pipeline::clipping::transcript::{
    cues_from_speech_events, cues_to_semantic_units, load_transcript_cues,
};
use crate::pipeline::engine::run_silence_analysis;
use crate::models::edl::PolicyConfig;

pub async fn run_clipping_analysis(
    media_path: &Path,
    options: ClippingOptions,
) -> AppResult<ClippingRun> {
    let t0 = Instant::now();
    let run_id = ClippingRun::new_id();
    let path_str = media_path.to_string_lossy().into_owned();

    let ffmpeg = Ffmpeg::new()?;
    let info = ffmpeg.probe(media_path).await?;
    let source_duration = info.duration;
    let framing = default_framing_for_media(&info);

    let mut warnings = Vec::new();
    let mut transcript_source = TranscriptSourceKind::AnalysisSpeechFallback;
    let mut has_real = false;

    // 1) Explicit transcript path
    // 2) Sidecar .srt / .vtt next to the media
    // 3) Optional Whisper CLI when prefer_whisper
    // 4) Speech-event fallback from silence engine
    let mut cues = Vec::new();

    let explicit = options.transcript_path.clone().filter(|p| !p.is_empty());
    let sidecar = find_sidecar_transcript(media_path);

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
            Ok(None) => {
                // quiet — no binary
            }
            Err(e) => warnings.push(format!("Whisper failed: {e}")),
        }
    }

    if cues.is_empty() {
        let policy = PolicyConfig {
            prefer_silero: true,
            ..PolicyConfig::default()
        };
        match run_silence_analysis(media_path, &policy).await {
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
    // Drop weak windows before preselect — they never leave the factory.
    let dropped_weak = candidates.iter().filter(|c| c.score < MIN_CLIP_SCORE).count();
    candidates.retain(|c| c.score >= MIN_CLIP_SCORE);
    if dropped_weak > 0 {
        warnings.push(format!(
            "Omitidos {dropped_weak} clips con score < {MIN_CLIP_SCORE:.0}"
        ));
    }
    apply_preselection(&mut candidates, &options);
    // Belt: never return discarded-below-floor leftovers (variants, etc.)
    candidates.retain(|c| c.score >= MIN_CLIP_SCORE);
    // Readable names: transcript phrase or Clip 01, Clip 02…
    finalize_clip_titles(&mut candidates);

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
    let best_score = candidates
        .iter()
        .map(|c| c.score)
        .fold(0.0_f64, f64::max);
    let selected_total_duration: f64 = candidates
        .iter()
        .filter(|c| c.status == ClipReviewStatus::Preselected && c.is_primary_variant)
        .map(|c| c.duration)
        .sum();

    // Only count clips that survive the score floor (what the user actually sees).
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
