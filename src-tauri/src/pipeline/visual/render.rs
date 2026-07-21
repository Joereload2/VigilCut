//! Render accepted visual placements as image overlays via FFmpeg.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::visual::{PlacementMode, VisualPlan};
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
        .filter(|p| {
            // Skip if fully inside a protected range
            !plan.is_protected(p.output_start, p.output_end)
        })
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
        let alpha = pl.layout.opacity.clamp(0.05, 1.0);
        let (scale, ox, oy) = match pl.mode {
            PlacementMode::Fullframe => {
                // Cover full frame
                (
                    format!(
                        "[{idx}:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
                    ),
                    "(W-w)/2".to_string(),
                    "(H-h)/2".to_string(),
                )
            }
            PlacementMode::PictureInPicture => {
                let wf = (pl.layout.w * 1280.0).round().max(80.0) as i32;
                let xf = pl.layout.x.clamp(0.0, 1.0);
                let yf = pl.layout.y.clamp(0.0, 1.0);
                (
                    format!(
                        "[{idx}:v]scale={wf}:-1:force_original_aspect_ratio=decrease,format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
                    ),
                    format!("(W-w)*{xf:.4}"),
                    format!("(H-h)*{yf:.4}"),
                )
            }
            PlacementMode::LowerThird => {
                let wf = (pl.layout.w * 1280.0).round().max(120.0) as i32;
                let yf = pl.layout.y.clamp(0.0, 1.0);
                (
                    format!(
                        "[{idx}:v]scale={wf}:-1:force_original_aspect_ratio=decrease,format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
                    ),
                    "(W-w)/2".to_string(),
                    format!("(H-h)*{yf:.4}"),
                )
            }
        };
        filter_parts.push(scale);
        let overlay = format!("{last}[ov{i}]overlay={ox}:{oy}:enable='{enable}'[v{i}]");
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

    // Manifest next to output (traceable, reproducible)
    let manifest_path = final_out.with_extension("visual-manifest.json");
    let manifest = serde_json::json!({
        "kind": "visual_render_manifest",
        "version": 1,
        "cutVideo": cut_video.to_string_lossy(),
        "output": final_out.to_string_lossy(),
        "sourceMedia": media_path_for_usage,
        "planId": plan.id,
        "runId": plan.run_id,
        "edlFingerprint": plan.edl_fingerprint,
        "planVersion": plan.version,
        "placements": active.iter().map(|p| serde_json::json!({
            "id": p.id,
            "assetId": p.asset_id,
            "outputStart": p.output_start,
            "outputEnd": p.output_end,
            "mode": p.mode,
            "provenance": p.provenance,
        })).collect::<Vec<_>>(),
        "renderedAt": chrono::Utc::now().to_rfc3339(),
        "note": "Overlays applied on cut timeline. Original media was not modified.",
    });
    let _ = std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    );
    // Also persist the full plan beside the output
    let plan_beside = final_out.with_extension("visual-plan.json");
    let _ = std::fs::write(
        &plan_beside,
        serde_json::to_string_pretty(plan).unwrap_or_default(),
    );

    Ok(final_out)
}
