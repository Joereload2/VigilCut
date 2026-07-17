use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::preset::{ColorOptions, ExportOptions};
use crate::models::segment::{Segment, SegmentDecision};

#[derive(Debug, Clone)]
pub struct ExportPlan {
    pub keep_ranges: Vec<(f64, f64)>,
    pub estimated_duration: f64,
    pub filter_complex: Option<String>,
}

/// Build keep ranges from user-approved segments (decision == Keep).
pub fn keep_ranges_from_segments(segments: &[Segment]) -> Vec<(f64, f64)> {
    let mut ranges: Vec<(f64, f64)> = segments
        .iter()
        .filter(|s| s.decision == SegmentDecision::Keep)
        .map(|s| (s.start, s.end))
        .collect();

    ranges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // Merge adjacent/overlapping keep ranges
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (s, e) in ranges {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 + 0.02 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

pub fn build_export_filter(
    keep: &[(f64, f64)],
    has_audio: bool,
    color: &ColorOptions,
) -> AppResult<String> {
    if keep.is_empty() {
        return Err(AppError::Invalid(
            "No segments marked Keep — nothing to export".into(),
        ));
    }

    let mut parts = Vec::new();
    let mut v_labels = Vec::new();
    let mut a_labels = Vec::new();

    for (i, (start, end)) in keep.iter().enumerate() {
        let dur = end - start;
        if dur <= 0.0 {
            continue;
        }
        let v = format!("v{i}");
        parts.push(format!(
            "[0:v]trim=start={start}:end={end},setpts=PTS-STARTPTS[{v}]"
        ));
        v_labels.push(format!("[{v}]"));

        if has_audio {
            let a = format!("a{i}");
            parts.push(format!(
                "[0:a]atrim=start={start}:end={end},asetpts=PTS-STARTPTS[{a}]"
            ));
            a_labels.push(format!("[{a}]"));
        }
    }

    if v_labels.is_empty() {
        return Err(AppError::Invalid("All keep ranges empty".into()));
    }

    let n = v_labels.len();
    parts.push(format!(
        "{}concat=n={n}:v=1:a=0[vout_raw]",
        v_labels.join("")
    ));

    let mut color_chain = String::from("[vout_raw]");
    if color.enabled {
        color_chain.push_str(&format!(
            "eq=brightness={}:contrast={}:saturation={}:gamma={}",
            color.brightness, color.contrast, color.saturation, color.gamma
        ));
        color_chain.push_str("[vout]");
    } else {
        color_chain.push_str("null[vout]");
    }
    parts.push(color_chain);

    if has_audio && !a_labels.is_empty() {
        parts.push(format!(
            "{}concat=n={}:v=0:a=1[aout]",
            a_labels.join(""),
            a_labels.len()
        ));
    }

    Ok(parts.join(";"))
}

pub async fn export_with_cuts(
    input: &Path,
    output: &Path,
    segments: &[Segment],
    export_opts: &ExportOptions,
    color: &ColorOptions,
    has_audio: bool,
) -> AppResult<PathBuf> {
    let ffmpeg = Ffmpeg::new()?;
    let keep = keep_ranges_from_segments(segments);
    let estimated: f64 = keep.iter().map(|(s, e)| e - s).sum();

    if !export_opts.apply_cuts {
        // Simple remux / reencode full file
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
        args.push(output.to_string_lossy().into_owned());
        ffmpeg.run(&args).await?;
        return Ok(output.to_path_buf());
    }

    let filter = build_export_filter(&keep, has_audio, color)?;
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
    args.push(output.to_string_lossy().into_owned());

    tracing::info!(
        "Exporting {} keep ranges (~{:.1}s) -> {}",
        keep.len(),
        estimated,
        output.display()
    );

    ffmpeg.run(&args).await?;
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
