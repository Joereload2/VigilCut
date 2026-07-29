//! Atomic vertical short render from VerticalRenderPlanV1.
//! UI never builds FFmpeg args — only this adapter does.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::pipeline::clipping::compute_crop_filter;
use crate::pipeline::safe_paths::{
    cleanup_temp, finalize_atomic, temp_export_path, unique_output_path, validate_export_output,
    validate_export_request,
};
use crate::vnext::domain::{
    ass_force_style_safe_center_bottom, VerticalRenderPlanV1, SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1,
};

#[derive(Debug, Clone)]
pub struct VerticalRenderResult {
    pub final_path: PathBuf,
    pub temp_was_used: bool,
    pub burned_subtitles: bool,
    pub duration_s: f64,
    pub byte_size: u64,
}

/// Write SRT (UTF-8) for burn-in. Times relative to short output timeline.
pub fn write_srt_for_cues(
    path: &Path,
    cues: &[crate::vnext::domain::SubtitleCueV1],
) -> AppResult<()> {
    let mut body = String::new();
    for (i, c) in cues.iter().enumerate() {
        body.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            srt_ts(c.start_s),
            srt_ts(c.end_s),
            c.text.replace('\n', " ")
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)?;
    Ok(())
}

fn srt_ts(secs: f64) -> String {
    let ms_total = (secs.max(0.0) * 1000.0).round() as u64;
    let h = ms_total / 3_600_000;
    let m = (ms_total % 3_600_000) / 60_000;
    let s = (ms_total % 60_000) / 1000;
    let ms = ms_total % 1000;
    format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

/// Escape path for FFmpeg `subtitles=` filter (best-effort on Windows).
fn escape_subtitles_path(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    // Escape characters special to the subtitles filter
    s.replace(':', "\\:")
        .replace('\'', "\\'")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

/// Render plan → temp → validate → atomic finalize.
/// `work_dir` holds temps and optional SRT; must be dedicated to this job.
pub async fn render_vertical_plan(
    plan: &VerticalRenderPlanV1,
    final_output: &Path,
    work_dir: &Path,
    src_w: u32,
    src_h: u32,
) -> AppResult<VerticalRenderResult> {
    plan.validate()
        .map_err(|e| AppError::Invalid(e.to_string()))?;

    let input = PathBuf::from(&plan.source_media_path);
    let final_out = unique_output_path(final_output);
    validate_export_request(&input, &final_out)?;
    std::fs::create_dir_all(work_dir)?;

    let temp = temp_export_path(&final_out);
    // Prefer temp next to final; also ensure work_dir sibling temp for cleanup scope
    let temp = if temp.starts_with(work_dir) {
        temp
    } else {
        work_dir.join(
            temp.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("render-{}.mp4", plan.id)),
        )
    };

    let crop = compute_crop_filter(&plan.framing, src_w, src_h);
    let mut burned = false;
    let mut vf = crop;

    if plan.subtitles.enabled && !plan.subtitles.cues.is_empty() {
        if plan.subtitles.preset_id != SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1 {
            return Err(AppError::Invalid(
                "Preset de subtítulos no aprobado para burn-in".into(),
            ));
        }
        let srt = work_dir.join("burn.srt");
        write_srt_for_cues(&srt, &plan.subtitles.cues)?;
        let esc = escape_subtitles_path(&srt);
        let style = ass_force_style_safe_center_bottom();
        vf = format!("{vf},subtitles='{esc}':force_style='{style}'");
        burned = true;
    }

    let start = plan.source_start_s.max(0.0);
    let dur = plan.duration_s();
    let spec = &plan.output_spec;

    let mut args = vec![
        "-y".into(),
        "-ss".into(),
        format!("{start:.3}"),
        "-i".into(),
        input.to_string_lossy().into_owned(),
        "-t".into(),
        format!("{dur:.3}"),
        "-vf".into(),
        vf,
        "-c:v".into(),
        spec.video_codec.clone(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        spec.crf.to_string(),
        "-c:a".into(),
        spec.audio_codec.clone(),
        "-b:a".into(),
        format!("{}k", spec.audio_bitrate_kbps),
        "-pix_fmt".into(),
        spec.pixel_format.clone(),
    ];
    if spec.faststart {
        args.push("-movflags".into());
        args.push("+faststart".into());
    }
    args.push(temp.to_string_lossy().into_owned());

    let ffmpeg = Ffmpeg::new()?;
    match ffmpeg.run_expecting(&args, Some(&temp)).await {
        Ok(_) => {}
        Err(e) if burned => {
            // Fallback: re-encode without subtitles filter (Windows path issues)
            tracing::warn!("subtitle burn failed, retry without burn-in: {e}");
            cleanup_temp(&temp);
            burned = false;
            let vf_only = compute_crop_filter(&plan.framing, src_w, src_h);
            let args2 = vec![
                "-y".into(),
                "-ss".into(),
                format!("{start:.3}"),
                "-i".into(),
                input.to_string_lossy().into_owned(),
                "-t".into(),
                format!("{dur:.3}"),
                "-vf".into(),
                vf_only,
                "-c:v".into(),
                spec.video_codec.clone(),
                "-preset".into(),
                "veryfast".into(),
                "-crf".into(),
                spec.crf.to_string(),
                "-c:a".into(),
                spec.audio_codec.clone(),
                "-b:a".into(),
                format!("{}k", spec.audio_bitrate_kbps),
                "-movflags".into(),
                "+faststart".into(),
                "-pix_fmt".into(),
                spec.pixel_format.clone(),
                temp.to_string_lossy().into_owned(),
            ];
            ffmpeg.run_expecting(&args2, Some(&temp)).await?;
        }
        Err(e) => {
            cleanup_temp(&temp);
            return Err(e);
        }
    }

    if let Err(e) = validate_export_output(&temp, dur) {
        cleanup_temp(&temp);
        return Err(e);
    }

    // Soft geometry check via probe when possible
    if let Ok(info) = ffmpeg.probe(&temp).await {
        if info.width > 0
            && info.height > 0
            && (info.width != spec.width || info.height != spec.height)
            && (info.width < 1000 || info.height < 1800)
        {
            cleanup_temp(&temp);
            return Err(AppError::Ffmpeg(format!(
                "Geometría inesperada {}x{} (esperado {}x{})",
                info.width, info.height, spec.width, spec.height
            )));
        }
    }

    finalize_atomic(&temp, &final_out)?;
    let meta = std::fs::metadata(&final_out)?;
    Ok(VerticalRenderResult {
        final_path: final_out,
        temp_was_used: true,
        burned_subtitles: burned,
        duration_s: dur,
        byte_size: meta.len(),
    })
}

/// Cleanup only files under `work_dir` (never touch inputs or foreign paths).
pub fn cleanup_job_work_dir(work_dir: &Path) {
    if !work_dir.exists() {
        return;
    }
    // Only remove known temp patterns inside work_dir
    if let Ok(rd) = std::fs::read_dir(work_dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.starts_with('.') && name.contains("vigilcut-tmp")
                || name.ends_with(".srt")
                || name.starts_with("render-") && name.ends_with(".mp4")
            {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vnext::domain::SubtitleCueV1;

    #[test]
    fn srt_timestamp_format() {
        assert_eq!(srt_ts(0.0), "00:00:00,000");
        assert_eq!(srt_ts(65.5), "00:01:05,500");
    }

    #[test]
    fn write_srt_roundtrip() {
        let dir = std::env::temp_dir().join(format!("vc-srt-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.srt");
        write_srt_for_cues(
            &p,
            &[SubtitleCueV1 {
                id: "1".into(),
                start_s: 0.0,
                end_s: 1.5,
                text: "hola".into(),
            }],
        )
        .unwrap();
        let s = std::fs::read_to_string(&p).unwrap();
        assert!(s.contains("hola"));
        assert!(s.contains("-->"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
