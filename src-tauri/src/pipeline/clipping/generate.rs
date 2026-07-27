//! Generate clip candidates from semantic units + duration profile.

use uuid::Uuid;

use crate::models::clipping::{
    ClipCandidate, ClipFraming, ClipReviewStatus, ClippingOptions, SemanticUnit,
};
use crate::pipeline::clipping::score::score_unit;

/// Slide / expand units into duration band and score each window.
pub fn generate_candidates(
    run_id: &str,
    media_path: &str,
    units: &[SemanticUnit],
    source_duration: f64,
    options: &ClippingOptions,
    has_real_transcript: bool,
    framing: ClipFraming,
) -> Vec<ClipCandidate> {
    let (min_d, ideal, max_d) = options.resolved_bounds();
    let mut out = Vec::new();

    if units.is_empty() {
        return out;
    }

    // Strategy A: each unit that already fits
    for u in units {
        let d = u.span.duration();
        if d >= min_d * 0.85 && d <= max_d * 1.1 {
            let start = (u.span.start - options.pad_before).max(0.0);
            let end = (u.span.end + options.pad_after).min(source_duration);
            if end - start < min_d * 0.8 {
                continue;
            }
            out.push(build_candidate(
                run_id,
                media_path,
                start,
                end,
                &u.text,
                u.energy,
                ideal,
                min_d,
                max_d,
                has_real_transcript,
                framing.clone(),
            ));
        }
    }

    // Strategy B: merge consecutive units until ideal window
    let mut i = 0;
    while i < units.len() {
        let mut j = i;
        let start0 = units[i].span.start;
        let mut end = units[i].span.end;
        let mut texts = vec![units[i].text.clone()];
        let mut energy = units[i].energy;
        while j + 1 < units.len() {
            let next = &units[j + 1];
            let gap = next.span.start - end;
            if gap > 2.5 {
                break;
            }
            let trial_end = next.span.end;
            if trial_end - start0 > max_d {
                break;
            }
            j += 1;
            end = trial_end;
            texts.push(next.text.clone());
            energy = energy.max(next.energy);
            if end - start0 >= ideal * 0.9 {
                break;
            }
        }
        let start = (start0 - options.pad_before).max(0.0);
        let end = (end + options.pad_after).min(source_duration);
        let dur = end - start;
        if dur >= min_d && dur <= max_d {
            let text = texts.join(" ");
            out.push(build_candidate(
                run_id,
                media_path,
                start,
                end,
                &text,
                energy,
                ideal,
                min_d,
                max_d,
                has_real_transcript,
                framing.clone(),
            ));
        }
        i = j + 1;
    }

    // Strategy C: sliding windows over speech timeline when few candidates
    if out.len() < 3 {
        let speech_start = units.first().map(|u| u.span.start).unwrap_or(0.0);
        let speech_end = units.last().map(|u| u.span.end).unwrap_or(source_duration);
        let mut t = speech_start;
        while t + min_d <= speech_end && out.len() < options.max_candidates {
            let end = (t + ideal).min(speech_end);
            if end - t >= min_d {
                let text = units
                    .iter()
                    .filter(|u| u.span.start < end && u.span.end > t)
                    .map(|u| u.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ");
                let energy = units
                    .iter()
                    .filter(|u| u.span.start < end && u.span.end > t)
                    .map(|u| u.energy)
                    .fold(0.5_f64, f64::max);
                out.push(build_candidate(
                    run_id,
                    media_path,
                    t,
                    end,
                    if text.is_empty() {
                        "[segmento de habla]"
                    } else {
                        &text
                    },
                    energy,
                    ideal,
                    min_d,
                    max_d,
                    has_real_transcript,
                    framing.clone(),
                ));
            }
            t += ideal * 0.65;
        }
    }

    out.truncate(options.max_candidates);
    out
}

fn build_candidate(
    run_id: &str,
    media_path: &str,
    start: f64,
    end: f64,
    text: &str,
    energy: f64,
    ideal: f64,
    min_d: f64,
    max_d: f64,
    has_real_transcript: bool,
    framing: ClipFraming,
) -> ClipCandidate {
    let duration = end - start;
    let unit = SemanticUnit {
        id: Uuid::new_v4().to_string(),
        span: crate::models::event::Span::new(start, end),
        text: text.to_string(),
        cue_ids: vec![],
        energy,
    };
    let scored = score_unit(&unit, duration, ideal, min_d, max_d, has_real_transcript);
    let id = Uuid::new_v4().to_string();
    ClipCandidate {
        id: id.clone(),
        analysis_run_id: run_id.to_string(),
        source_media_path: media_path.to_string(),
        start,
        end,
        duration,
        transcript: text.to_string(),
        title: scored.title,
        summary: scored.summary,
        score: scored.score,
        confidence: scored.confidence,
        breakdown: scored.breakdown,
        reasons: scored.reasons,
        warnings: Vec::new(),
        strengths: scored.strengths,
        risks: scored.risks,
        status: ClipReviewStatus::Suggested,
        variant_group_id: id,
        is_primary_variant: true,
        framing,
        original_start: start,
        original_end: end,
        export_path: None,
        error: None,
    }
}
