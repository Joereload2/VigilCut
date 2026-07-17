use std::path::Path;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::models::segment::{
    Segment, SegmentDecision, SegmentKind, SilenceDetectionOptions, SilenceDetectionResult,
};
use crate::state::AppState;

/// Detect speech/silence and build reviewable segments.
/// MVP: FFmpeg silencedetect. Silero VAD hooks in when ONNX model is present.
pub async fn detect_and_build_segments(
    media_path: &Path,
    options: &SilenceDetectionOptions,
) -> AppResult<SilenceDetectionResult> {
    let ffmpeg = Ffmpeg::new()?;
    let info = ffmpeg.probe(media_path).await?;
    let duration = info.duration;

    let method;
    let silence_ranges: Vec<(f64, f64)>;

    let silero_model = AppState::models_dir()
        .ok()
        .map(|d| d.join("silero_vad.onnx"));
    let silero_available = silero_model
        .as_ref()
        .map(|p| p.is_file())
        .unwrap_or(false);

    if options.prefer_silero && silero_available {
        // Placeholder for Silero ONNX inference (see docs/ARCHITECTURE.md).
        // Falls through to FFmpeg until model runner is wired.
        method = "silero_vad+ffmpeg_fallback".to_string();
        let noise_db = threshold_to_noise_db(options.threshold);
        silence_ranges = ffmpeg
            .detect_silences_ffmpeg(media_path, noise_db, options.min_silence_duration)
            .await?;
    } else {
        method = "ffmpeg_silencedetect".to_string();
        let noise_db = threshold_to_noise_db(options.threshold);
        silence_ranges = ffmpeg
            .detect_silences_ffmpeg(media_path, noise_db, options.min_silence_duration)
            .await?;
    }

    let segments = silences_to_segments(
        duration,
        &silence_ranges,
        options.padding,
        options.min_silence_duration,
        options.auto_cut_silence,
    );

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

    Ok(SilenceDetectionResult {
        media_path: media_path.to_string_lossy().into_owned(),
        duration,
        segments,
        method,
        speech_duration,
        silence_duration,
        cut_duration,
    })
}

fn threshold_to_noise_db(threshold: f64) -> f64 {
    // Map UI 0..1 threshold to roughly -50..-20 dB for silencedetect
    let t = threshold.clamp(0.05, 0.95);
    -50.0 + t * 30.0
}

/// Convert silence ranges into alternating speech/silence segments covering [0, duration].
pub fn silences_to_segments(
    duration: f64,
    silences: &[(f64, f64)],
    padding: f64,
    min_silence: f64,
    auto_cut_silence: bool,
) -> Vec<Segment> {
    let mut cleaned: Vec<(f64, f64)> = silences
        .iter()
        .map(|(s, e)| {
            // Shrink silence by padding so speech edges are preserved
            let start = (s + padding).min(*e);
            let end = (e - padding).max(start);
            (start.max(0.0), end.min(duration))
        })
        .filter(|(s, e)| e - s >= min_silence)
        .collect();

    cleaned.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Merge overlapping
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

    let mut segments = Vec::new();
    let mut cursor = 0.0_f64;

    for (s, e) in merged {
        if s > cursor + 0.01 {
            segments.push(Segment::new(
                cursor,
                s,
                SegmentKind::Speech,
                SegmentDecision::Keep,
            ));
        }
        let decision = if auto_cut_silence {
            SegmentDecision::Cut
        } else {
            SegmentDecision::Pending
        };
        segments.push(Segment::new(s, e, SegmentKind::Silence, decision));
        cursor = e;
    }

    if cursor < duration - 0.01 {
        segments.push(Segment::new(
            cursor,
            duration,
            SegmentKind::Speech,
            SegmentDecision::Keep,
        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_speech_and_silence() {
        let segs = silences_to_segments(10.0, &[(2.0, 3.5), (7.0, 8.0)], 0.1, 0.3, true);
        assert!(segs.len() >= 3);
        assert!(segs.iter().any(|s| s.kind == SegmentKind::Silence));
        assert!(segs.iter().any(|s| s.kind == SegmentKind::Speech));
        let total: f64 = segs.iter().map(|s| s.duration()).sum();
        assert!((total - 10.0).abs() < 0.2);
    }
}
