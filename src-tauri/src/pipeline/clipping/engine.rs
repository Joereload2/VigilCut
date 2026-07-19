//! Orchestrate clipping analysis run.

use std::path::Path;
use std::time::Instant;

use chrono::Utc;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::models::clipping::{
    ClipReviewStatus, ClippingOptions, ClippingRun, ClippingSummary, TranscriptSourceKind,
};
use crate::pipeline::clipping::dedupe::dedupe_and_group;
use crate::pipeline::clipping::framing::default_framing_for_media;
use crate::pipeline::clipping::generate::generate_candidates;
use crate::pipeline::clipping::preselect::apply_preselection;
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

    let cues = if let Some(ref tp) = options.transcript_path {
        match load_transcript_cues(Path::new(tp)) {
            Ok((c, kind)) => {
                transcript_source = kind;
                has_real = true;
                c
            }
            Err(e) => {
                warnings.push(format!("Transcript import failed ({e}); using speech fallback"));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let cues = if cues.is_empty() {
        // Reuse silence engine speech spans as timing backbone
        let policy = PolicyConfig {
            prefer_silero: true,
            ..PolicyConfig::default()
        };
        match run_silence_analysis(media_path, &policy).await {
            Ok(ar) => {
                let c = cues_from_speech_events(&ar.events);
                if c.is_empty() {
                    warnings.push("No speech spans detected for clipping".into());
                } else {
                    warnings.push(
                        "Sin SRT/VTT: candidatos desde bloques de habla (importa subtítulos para mejor calidad)"
                            .into(),
                    );
                }
                c
            }
            Err(e) => {
                warnings.push(format!("Speech analysis failed: {e}"));
                Vec::new()
            }
        }
    } else {
        cues
    };

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
    apply_preselection(&mut candidates, &options);

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

    let analysis_seconds = t0.elapsed().as_secs_f64();
    let summary = ClippingSummary {
        source_duration,
        analysis_seconds,
        candidates_found: candidates.iter().filter(|c| c.is_primary_variant).count(),
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
