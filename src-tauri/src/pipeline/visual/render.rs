//! Render accepted visual placements as image overlays via FFmpeg.
//! Geometry comes from [`layout::PlacementGeom`] — same contract as live preview.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::visual::VisualPlan;
use crate::pipeline::safe_paths::{
    cleanup_temp, finalize_atomic, temp_export_path, unique_output_path, validate_export_output,
    validate_export_request,
};
use crate::pipeline::visual::layout::{
    geom_json, PlacementGeom, DEFAULT_FRAME_H, DEFAULT_FRAME_W,
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
        .filter(|p| !plan.is_protected(p.output_start, p.output_end))
        .cloned()
        .collect();

    if active.is_empty() {
        return Err(AppError::Invalid(
            "VisualPlan sin placements activos".into(),
        ));
    }

    let frame_w = DEFAULT_FRAME_W;
    let frame_h = DEFAULT_FRAME_H;

    let all = list_assets(None, 500)?;
    let mut inputs: Vec<PathBuf> = vec![cut_video.to_path_buf()];
    let mut filter_parts = Vec::new();
    // Normalize main video to canonical frame so overlay math matches layout.rs
    filter_parts.push(format!(
        "[0:v]scale={frame_w}:{frame_h}:force_original_aspect_ratio=decrease,pad={frame_w}:{frame_h}:(ow-iw)/2:(oh-ih)/2,setsar=1[base0]"
    ));
    let mut last = "[base0]".to_string();

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

        let geom = PlacementGeom::from_placement(pl);
        let scale = geom.ffmpeg_scale_filter(idx, i, frame_w, frame_h);
        let (ox, oy) = geom.ffmpeg_overlay_xy(frame_w, frame_h);
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
    if let Err(e) = validate_export_output(
        &temp,
        plan.placements
            .iter()
            .map(|p| p.output_end)
            .fold(0.0, f64::max),
    ) {
        cleanup_temp(&temp);
        return Err(e);
    }
    if let Err(e) = finalize_atomic(&temp, &final_out) {
        cleanup_temp(&temp);
        return Err(e);
    }

    for pl in &active {
        let _ = record_usage(
            &pl.asset_id,
            media_path_for_usage,
            Some(&plan.run_id),
            pl.output_start,
            pl.output_end,
        );
    }

    let manifest_path = final_out.with_extension("visual-manifest.json");
    let manifest = serde_json::json!({
        "kind": "visual_render_manifest",
        "version": 2,
        "layoutContract": "center_norm_v1",
        "frame": { "w": frame_w, "h": frame_h },
        "cutVideo": cut_video.to_string_lossy(),
        "output": final_out.to_string_lossy(),
        "sourceMedia": media_path_for_usage,
        "planId": plan.id,
        "runId": plan.run_id,
        "edlFingerprint": plan.edl_fingerprint,
        "planVersion": plan.version,
        "placements": active.iter().map(|p| {
            let mut o = serde_json::json!({
                "id": p.id,
                "assetId": p.asset_id,
                "outputStart": p.output_start,
                "outputEnd": p.output_end,
                "mode": p.mode,
                "fit": p.fit,
                "provenance": p.provenance,
            });
            if let Some(map) = o.as_object_mut() {
                map.insert("layout".into(), geom_json(p));
            }
            o
        }).collect::<Vec<_>>(),
        "renderedAt": chrono::Utc::now().to_rfc3339(),
        "note": "Overlays use PlacementGeom center-normalized layout. Original media not modified.",
    });
    let _ = std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    );
    let plan_beside = final_out.with_extension("visual-plan.json");
    let _ = std::fs::write(
        &plan_beside,
        serde_json::to_string_pretty(plan).unwrap_or_default(),
    );

    Ok(final_out)
}
