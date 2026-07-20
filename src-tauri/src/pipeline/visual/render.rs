//! Render accepted visual placements as image overlays via FFmpeg.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::visual::VisualPlan;
use crate::pipeline::safe_paths::{
    cleanup_temp, finalize_atomic, temp_export_path, unique_output_path, validate_export_output,
    validate_export_request,
};
use crate::pipeline::visual::library::{list_assets, record_usage};

/// Overlay placements onto an already-cut longform video (output timeline).
pub async fn render_visual_plan(
    cut_video: &Path,
    plan: &VisualPlan,
    output: &Path,
    media_path_for_usage: &str,
) -> AppResult<PathBuf> {
    let final_out = unique_output_path(output);
    validate_export_request(cut_video, &final_out)?;

    let active: Vec<_> = plan
        .placements
        .iter()
        .filter(|p| p.status == "active")
        .cloned()
        .collect();

    if active.is_empty() {
        // Nothing to overlay — refuse silent "copy as visual render" without labeling
        return Err(AppError::Invalid(
            "VisualPlan sin placements activos".into(),
        ));
    }

    // Resolve asset paths
    let all = list_assets(None, 500)?;
    let mut inputs: Vec<PathBuf> = vec![cut_video.to_path_buf()];
    let mut filter_parts = Vec::new();
    let mut last = "[0:v]".to_string();

    for (i, pl) in active.iter().enumerate() {
        let asset = all
            .iter()
            .find(|a| a.id == pl.asset_id)
            .ok_or_else(|| AppError::NotFound(format!("asset {}", pl.asset_id)))?;
        let img = PathBuf::from(&asset.managed_path);
        if !img.is_file() {
            return Err(AppError::NotFound(format!(
                "Imagen administrada faltante: {}",
                asset.managed_path
            )));
        }
        inputs.push(img);
        let idx = i + 1;
        let start = pl.output_start;
        let end = pl.output_end;
        let enable = format!("between(t\\,{start:.3}\\,{end:.3})");
        // Scale image to cover 1080p-ish height, overlay center
        let scaled = format!("[{idx}:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,format=rgba,colorchannelmixer=aa=0.92[ov{i}]");
        filter_parts.push(scaled);
        let overlay = format!(
            "{last}[ov{i}]overlay=(W-w)/2:(H-h)/2:enable='{enable}'[v{i}]"
        );
        filter_parts.push(overlay);
        last = format!("[v{i}]");
    }

    let filter = filter_parts.join(";");
    let temp = temp_export_path(&final_out);
    let ffmpeg = Ffmpeg::new()?;

    let mut args: Vec<String> = vec!["-y".into()];
    for inp in &inputs {
        // loop still image
        if inp != cut_video {
            args.push("-loop".into());
            args.push("1".into());
        }
        args.push("-i".into());
        args.push(inp.to_string_lossy().into_owned());
    }
    args.extend([
        "-filter_complex".into(),
        filter,
        "-map".into(),
        last,
        "-map".into(),
        "0:a?".into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "20".into(),
        "-c:a".into(),
        "aac".into(),
        "-shortest".into(),
        "-movflags".into(),
        "+faststart".into(),
        temp.to_string_lossy().into_owned(),
    ]);

    let result = ffmpeg.run_expecting(&args, Some(&temp)).await;
    if let Err(e) = result {
        cleanup_temp(&temp);
        return Err(e);
    }
    if let Err(e) = validate_export_output(&temp, plan.placements.iter().map(|p| p.output_end).fold(0.0, f64::max)) {
        cleanup_temp(&temp);
        return Err(e);
    }
    if let Err(e) = finalize_atomic(&temp, &final_out) {
        cleanup_temp(&temp);
        return Err(e);
    }

    // Record usage only after success
    for pl in &active {
        let _ = record_usage(
            &pl.asset_id,
            media_path_for_usage,
            Some(&plan.run_id),
            pl.output_start,
            pl.output_end,
        );
    }

    Ok(final_out)
}
