use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::job_control::JobControl;
use crate::models::edl::Edl;
use crate::models::preset::{AudioEnhanceOptions, ColorOptions, ExportOptions};
use crate::models::segment::{Segment, SegmentDecision};

#[derive(Debug, Clone)]
pub struct ExportPlan {
    pub keep_ranges: Vec<(f64, f64)>,
    pub estimated_duration: f64,
    pub filter_complex: Option<String>,
}

/// Merge overlapping / adjacent keep ranges (fewer cuts → stabler export).
pub fn merge_keep_ranges(ranges: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut ranges = ranges;
    ranges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (s, e) in ranges {
        if e - s <= 0.001 {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if s <= last.1 + 0.08 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

/// Canonical path: keep ranges from EDL (engine truth).
pub fn keep_ranges_from_edl(edl: &Edl) -> Vec<(f64, f64)> {
    merge_keep_ranges(edl.keep_ranges())
}

/// UI / supervision path: keep ranges from segment decisions.
/// Pending (unresolved exceptions) are treated as Keep (conservative).
pub fn keep_ranges_from_segments(segments: &[Segment]) -> Vec<(f64, f64)> {
    let ranges: Vec<(f64, f64)> = segments
        .iter()
        .filter(|s| matches!(s.decision, SegmentDecision::Keep | SegmentDecision::Pending))
        .map(|s| (s.start, s.end))
        .collect();
    merge_keep_ranges(ranges)
}

/// Resolve keep ranges with EDL preferred over segments when provided.
pub fn resolve_keep_ranges(
    edl: Option<&Edl>,
    segments: Option<&[Segment]>,
    explicit: Option<Vec<(f64, f64)>>,
) -> AppResult<Vec<(f64, f64)>> {
    if let Some(k) = explicit {
        let m = merge_keep_ranges(k);
        if m.is_empty() {
            return Err(AppError::Invalid(
                "No keep ranges — nothing to export".into(),
            ));
        }
        return Ok(m);
    }
    if let Some(edl) = edl {
        let m = keep_ranges_from_edl(edl);
        if !m.is_empty() {
            return Ok(m);
        }
    }
    if let Some(segs) = segments {
        let m = keep_ranges_from_segments(segs);
        if !m.is_empty() {
            return Ok(m);
        }
    }
    Err(AppError::Invalid(
        "No keep ranges — nothing to export".into(),
    ))
}

/// Build audio enhance chain after aselect/asetpts (comma-separated FFmpeg filters).
pub fn audio_enhance_filters(audio: &AudioEnhanceOptions) -> Vec<String> {
    if !audio.enabled {
        return Vec::new();
    }
    let mut filters = Vec::new();
    if let Some(hz) = audio.highpass_hz {
        filters.push(format!("highpass=f={hz}"));
    }
    if audio.denoise {
        let nr = (audio.denoise_strength * 20.0).clamp(1.0, 30.0);
        filters.push(format!("afftdn=nr={nr:.1}"));
    }
    if audio.compress {
        filters.push("acompressor=threshold=-18dB:ratio=3:attack=20:release=250".into());
    }
    if audio.normalize {
        filters.push(format!("loudnorm=I={}:TP=-1.5:LRA=11", audio.target_lufs));
    }
    filters
}

/// Build a single-pass select/aselect filter (stable with dozens of cuts).
pub fn build_export_filter(
    keep: &[(f64, f64)],
    has_audio: bool,
    color: &ColorOptions,
    audio: Option<&AudioEnhanceOptions>,
) -> AppResult<String> {
    if keep.is_empty() {
        return Err(AppError::Invalid(
            "No segments marked Keep — nothing to export".into(),
        ));
    }

    let ranges: Vec<(f64, f64)> = keep
        .iter()
        .copied()
        .filter(|(s, e)| e - s > 0.001)
        .collect();
    if ranges.is_empty() {
        return Err(AppError::Invalid("All keep ranges empty".into()));
    }

    // Commas inside select expressions must be escaped for filter_complex.
    let expr = ranges
        .iter()
        .map(|(s, e)| format!("between(t\\,{s:.3}\\,{e:.3})"))
        .collect::<Vec<_>>()
        .join("+");

    let mut parts = Vec::new();
    if color.enabled {
        parts.push(format!(
            "[0:v]select='{expr}',setpts=N/FRAME_RATE/TB,eq=brightness={}:contrast={}:saturation={}:gamma={}[vout]",
            color.brightness, color.contrast, color.saturation, color.gamma
        ));
    } else {
        parts.push(format!("[0:v]select='{expr}',setpts=N/FRAME_RATE/TB[vout]"));
    }

    if has_audio {
        let mut chain = format!("[0:a]aselect='{expr}',asetpts=N/SR/TB");
        if let Some(a) = audio {
            for f in audio_enhance_filters(a) {
                chain.push(',');
                chain.push_str(&f);
            }
        }
        chain.push_str("[aout]");
        parts.push(chain);
    }

    Ok(parts.join(";"))
}

/// Primary export API — EDL / keep-ranges based (not Segment-centric).
pub async fn export_keep_ranges(
    input: &Path,
    output: &Path,
    keep: &[(f64, f64)],
    export_opts: &ExportOptions,
    color: &ColorOptions,
    has_audio: bool,
) -> AppResult<PathBuf> {
    export_keep_ranges_with_audio(
        input,
        output,
        keep,
        export_opts,
        color,
        None,
        has_audio,
        None,
    )
    .await
}

pub async fn export_keep_ranges_with_audio(
    input: &Path,
    output: &Path,
    keep: &[(f64, f64)],
    export_opts: &ExportOptions,
    color: &ColorOptions,
    audio: Option<&AudioEnhanceOptions>,
    has_audio: bool,
    job: Option<&JobControl>,
) -> AppResult<PathBuf> {
    use crate::pipeline::safe_paths::{
        cleanup_temp, finalize_atomic, temp_export_path, unique_output_path,
        validate_export_output, validate_export_request,
    };

    if let Some(j) = job {
        j.check()?;
    }

    // Never write over the original; avoid silent overwrites of existing destinations.
    let final_out = unique_output_path(output);
    validate_export_request(input, &final_out)?;

    let ffmpeg = Ffmpeg::new()?;
    let keep = merge_keep_ranges(keep.to_vec());
    let estimated: f64 = keep.iter().map(|(s, e)| e - s).sum();

    if keep.is_empty() {
        return Err(AppError::Invalid(
            "No keep ranges — nothing to export".into(),
        ));
    }

    let temp_out = temp_export_path(&final_out);

    let run_result = async {
        if !export_opts.apply_cuts {
            let mut args = vec![
                "-y".into(),
                "-i".into(),
                input.to_string_lossy().into_owned(),
            ];
            if export_opts.reencode {
                args.extend([
                    "-c:v".into(),
                    export_opts.video_codec.clone(),
                    "-crf".into(),
                    export_opts.crf.to_string(),
                    "-preset".into(),
                    export_opts.preset.clone(),
                    "-c:a".into(),
                    export_opts.audio_codec.clone(),
                    "-b:a".into(),
                    format!("{}k", export_opts.audio_bitrate_k),
                ]);
            } else {
                args.extend(["-c".into(), "copy".into()]);
            }
            args.push(temp_out.to_string_lossy().into_owned());
            ffmpeg
                .run_expecting_tracked(&args, Some(&temp_out), job)
                .await?;
        } else {
            let filter = build_export_filter(&keep, has_audio, color, audio)?;
            let mut args = vec![
                "-y".into(),
                "-i".into(),
                input.to_string_lossy().into_owned(),
                "-filter_complex".into(),
                filter,
                "-map".into(),
                "[vout]".into(),
            ];

            if has_audio {
                args.extend(["-map".into(), "[aout]".into()]);
            }

            args.extend([
                "-c:v".into(),
                export_opts.video_codec.clone(),
                "-crf".into(),
                export_opts.crf.to_string(),
                "-preset".into(),
                export_opts.preset.clone(),
            ]);

            if has_audio {
                args.extend([
                    "-c:a".into(),
                    export_opts.audio_codec.clone(),
                    "-b:a".into(),
                    format!("{}k", export_opts.audio_bitrate_k),
                ]);
            }

            args.extend(["-movflags".into(), "+faststart".into()]);
            args.push(temp_out.to_string_lossy().into_owned());

            tracing::info!(
                "Exporting {} keep ranges (~{:.1}s) -> temp {} (audio_enhance={})",
                keep.len(),
                estimated,
                temp_out.display(),
                audio.map(|a| a.enabled).unwrap_or(false)
            );

            ffmpeg
                .run_expecting_tracked(&args, Some(&temp_out), job)
                .await?;
        }

        validate_export_output(&temp_out, estimated)?;
        finalize_atomic(&temp_out, &final_out)?;
        Ok::<PathBuf, AppError>(final_out.clone())
    }
    .await;

    match run_result {
        Ok(p) => {
            tracing::info!("Export finalized (original untouched) → {}", p.display());
            Ok(p)
        }
        Err(e) => {
            cleanup_temp(&temp_out);
            Err(e)
        }
    }
}

/// Convenience: export from segment decisions (UI path).
pub async fn export_with_cuts(
    input: &Path,
    output: &Path,
    segments: &[Segment],
    export_opts: &ExportOptions,
    color: &ColorOptions,
    has_audio: bool,
) -> AppResult<PathBuf> {
    let keep = keep_ranges_from_segments(segments);
    export_keep_ranges(input, output, &keep, export_opts, color, has_audio).await
}

/// Export from EDL (factory / batch / CLI path).
pub async fn export_from_edl(
    input: &Path,
    output: &Path,
    edl: &Edl,
    export_opts: &ExportOptions,
    color: &ColorOptions,
    has_audio: bool,
) -> AppResult<PathBuf> {
    let keep = keep_ranges_from_edl(edl);
    export_keep_ranges(input, output, &keep, export_opts, color, has_audio).await
}

/// Export a single source span as a standalone clip (e.g. Short).
pub async fn export_clip(
    input: &Path,
    output: &Path,
    start: f64,
    end: f64,
    export_opts: &ExportOptions,
) -> AppResult<PathBuf> {
    let ffmpeg = Ffmpeg::new()?;
    let start = start.max(0.0);
    let end = end.max(start + 0.1);
    let args = vec![
        "-y".into(),
        "-ss".into(),
        format!("{start:.3}"),
        "-to".into(),
        format!("{end:.3}"),
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-c:v".into(),
        export_opts.video_codec.clone(),
        "-crf".into(),
        export_opts.crf.to_string(),
        "-preset".into(),
        export_opts.preset.clone(),
        "-c:a".into(),
        export_opts.audio_codec.clone(),
        "-b:a".into(),
        format!("{}k", export_opts.audio_bitrate_k),
        "-movflags".into(),
        "+faststart".into(),
        output.to_string_lossy().into_owned(),
    ];
    ffmpeg.run_expecting(&args, Some(output)).await?;
    Ok(output.to_path_buf())
}

pub fn estimate_export(segments: &[Segment]) -> ExportPlan {
    let keep_ranges = keep_ranges_from_segments(segments);
    let estimated_duration = keep_ranges.iter().map(|(s, e)| e - s).sum();
    ExportPlan {
        keep_ranges,
        estimated_duration,
        filter_complex: None,
    }
}

pub fn estimate_from_keep(keep: &[(f64, f64)]) -> ExportPlan {
    let keep_ranges = merge_keep_ranges(keep.to_vec());
    let estimated_duration = keep_ranges.iter().map(|(s, e)| e - s).sum();
    ExportPlan {
        keep_ranges,
        estimated_duration,
        filter_complex: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edl::Edl;
    use crate::models::segment::{Segment, SegmentDecision, SegmentKind};

    #[test]
    fn merge_adjacent() {
        let m = merge_keep_ranges(vec![(0.0, 1.0), (1.05, 2.0), (5.0, 6.0)]);
        assert_eq!(m.len(), 2);
        assert!((m[0].1 - 2.0).abs() < 0.01);
    }

    #[test]
    fn filter_escapes_commas() {
        let f = build_export_filter(
            &[(1.0, 2.0), (5.0, 6.0)],
            true,
            &ColorOptions::default(),
            None,
        )
        .unwrap();
        assert!(f.contains("between(t\\,"));
        assert!(f.contains("[vout]"));
        assert!(f.contains("[aout]"));
    }

    #[test]
    fn audio_enhance_appended_to_chain() {
        let mut audio = AudioEnhanceOptions::default();
        audio.enabled = true;
        audio.denoise = true;
        audio.normalize = true;
        audio.highpass_hz = Some(80);
        let f = build_export_filter(&[(0.0, 5.0)], true, &ColorOptions::default(), Some(&audio))
            .unwrap();
        assert!(f.contains("highpass=f=80"));
        assert!(f.contains("afftdn="));
        assert!(f.contains("loudnorm="));
        assert!(f.contains("[aout]"));
    }

    #[test]
    fn resolve_prefers_explicit_then_edl_then_segments() {
        let edl = Edl::from_remove_spans("x.mp4", 10.0, &[(2.0, 3.0)]);
        let segs = vec![Segment::new(
            0.0,
            10.0,
            SegmentKind::Speech,
            SegmentDecision::Keep,
        )];

        let k = resolve_keep_ranges(None, None, Some(vec![(0.0, 1.0), (2.0, 3.0)])).unwrap();
        assert_eq!(k.len(), 2);

        let k2 = resolve_keep_ranges(Some(&edl), Some(&segs), None).unwrap();
        assert!(!k2.is_empty());
        let dur: f64 = k2.iter().map(|(s, e)| e - s).sum();
        assert!((dur - 9.0).abs() < 0.1);

        let k3 = resolve_keep_ranges(None, Some(&segs), None).unwrap();
        assert_eq!(k3.len(), 1);
    }

    #[test]
    fn pending_segments_count_as_keep() {
        let segs = vec![
            Segment::new(0.0, 1.0, SegmentKind::Speech, SegmentDecision::Keep),
            Segment::new(1.0, 2.0, SegmentKind::Silence, SegmentDecision::Pending),
            Segment::new(2.0, 3.0, SegmentKind::Speech, SegmentDecision::Keep),
            Segment::new(3.0, 4.0, SegmentKind::Silence, SegmentDecision::Cut),
        ];
        let k = keep_ranges_from_segments(&segs);
        // keep+pending merge → [0,2] then [2,3]? 0-1 keep, 1-2 pending, 2-3 keep → merge to [0,3]
        assert_eq!(k.len(), 1);
        assert!((k[0].0 - 0.0).abs() < 0.01);
        assert!((k[0].1 - 3.0).abs() < 0.01);
    }
}
