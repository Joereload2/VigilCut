//! Export approved clips as vertical 9:16 MP4 via FFmpeg.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::clipping::{ClipCandidate, ClipExportResult, ClipFraming, ClipReviewStatus};
use crate::pipeline::clipping::framing::compute_crop_filter;

pub async fn export_one_clip(
    media_path: &Path,
    candidate: &ClipCandidate,
    output_path: &Path,
    framing: &ClipFraming,
    src_w: u32,
    src_h: u32,
) -> AppResult<PathBuf> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let ffmpeg = Ffmpeg::new()?;
    let vf = compute_crop_filter(framing, src_w, src_h);
    let start = candidate.start.max(0.0);
    let dur = (candidate.end - candidate.start).max(0.15);

    let args = vec![
        "-y".into(),
        "-ss".into(),
        format!("{start:.3}"),
        "-i".into(),
        media_path.to_string_lossy().into_owned(),
        "-t".into(),
        format!("{dur:.3}"),
        "-vf".into(),
        vf,
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        "veryfast".into(),
        "-crf".into(),
        "20".into(),
        "-c:a".into(),
        "aac".into(),
        "-b:a".into(),
        "160k".into(),
        "-movflags".into(),
        "+faststart".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
        output_path.to_string_lossy().into_owned(),
    ];
    ffmpeg.run_expecting(&args, Some(output_path)).await?;
    if !output_path.is_file() {
        return Err(AppError::Ffmpeg("clip export produced no file".into()));
    }
    Ok(output_path.to_path_buf())
}

/// Export selected candidates (by id). If `ids` is empty, export Approved + Preselected primaries.
pub async fn export_approved_clips(
    media_path: &Path,
    candidates: &mut [ClipCandidate],
    ids: &[String],
    output_dir: &Path,
    framing_override: Option<&ClipFraming>,
    src_w: u32,
    src_h: u32,
) -> AppResult<Vec<ClipExportResult>> {
    let clips_dir = output_dir.join("clips");
    std::fs::create_dir_all(&clips_dir)?;

    let mut results = Vec::new();
    let mut index = 1usize;

    for c in candidates.iter_mut() {
        let selected = if ids.is_empty() {
            c.is_primary_variant
                && matches!(
                    c.status,
                    ClipReviewStatus::Approved
                        | ClipReviewStatus::Preselected
                        | ClipReviewStatus::Modified
                )
        } else {
            ids.contains(&c.id)
        };
        if !selected {
            continue;
        }

        c.status = ClipReviewStatus::Exporting;
        let slug = sanitize_filename(&c.title);
        let name = format!("{index:03}_{slug}.mp4");
        let out = clips_dir.join(&name);
        let framing = framing_override.unwrap_or(&c.framing);

        match export_one_clip(media_path, c, &out, framing, src_w, src_h).await {
            Ok(p) => {
                c.status = ClipReviewStatus::Exported;
                c.export_path = Some(p.to_string_lossy().into_owned());
                c.error = None;
                results.push(ClipExportResult {
                    candidate_id: c.id.clone(),
                    ok: true,
                    output_path: Some(p.to_string_lossy().into_owned()),
                    error: None,
                });
                index += 1;
            }
            Err(e) => {
                c.status = ClipReviewStatus::Error;
                c.error = Some(e.to_string());
                results.push(ClipExportResult {
                    candidate_id: c.id.clone(),
                    ok: false,
                    output_path: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    let meta = serde_json::json!({
        "source": media_path.to_string_lossy(),
        "exported": results,
        "engine": "vigilcut-clipping@1",
    });
    std::fs::write(
        output_dir.join("metadata.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

    let report = serde_json::json!({
        "ok": results.iter().filter(|r| r.ok).count(),
        "failed": results.iter().filter(|r| !r.ok).count(),
        "results": results,
    });
    std::fs::write(
        output_dir.join("clipping-report.json"),
        serde_json::to_string_pretty(&report)?,
    )?;

    Ok(results)
}

fn sanitize_filename(s: &str) -> String {
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_lowercase();
    if s.is_empty() {
        "clip".into()
    } else {
        s.chars().take(48).collect()
    }
}
