use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::error::AppResult;
use crate::job_control::JobControl;
use crate::models::preset::{AudioEnhanceOptions, ColorOptions, ExportOptions};
use crate::models::progress;
use crate::models::segment::Segment;
use crate::pipeline::export::{
    estimate_export as estimate_export_plan, estimate_from_keep, export_keep_ranges_with_audio,
    keep_ranges_from_segments, merge_keep_ranges, resolve_keep_ranges,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub output_path: String,
    pub duration: f64,
    pub keep_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportEstimate {
    pub estimated_duration: f64,
    pub keep_ranges: Vec<[f64; 2]>,
    pub cut_duration: f64,
    pub source_duration: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSkipPlan {
    /// Ordered keep ranges for the frontend preview player to jump cuts
    pub keep_ranges: Vec<[f64; 2]>,
    pub estimated_duration: f64,
}

/// Export video. Prefer explicit `keep_ranges` (EDL / factory truth).
/// Falls back to segment decisions for manual UI overrides.
#[tauri::command]
pub async fn export_video(
    app: AppHandle,
    jobs: State<'_, JobControl>,
    media_path: String,
    output_path: String,
    segments: Option<Vec<Segment>>,
    keep_ranges: Option<Vec<[f64; 2]>>,
    export_options: Option<ExportOptions>,
    color_options: Option<ColorOptions>,
    audio_options: Option<AudioEnhanceOptions>,
    has_audio: Option<bool>,
) -> AppResult<ExportResult> {
    jobs.begin();
    progress::emit(&app, "export", "prepare", "Preparando export…", 5.0);

    let export_opts = export_options.unwrap_or_default();
    let color = color_options.unwrap_or_default();
    let audio = audio_options.unwrap_or_default();
    let has_audio = has_audio.unwrap_or(true);

    let explicit = keep_ranges.map(|ranges| {
        ranges
            .into_iter()
            .map(|r| (r[0], r[1]))
            .collect::<Vec<_>>()
    });

    jobs.check()?;
    progress::emit(&app, "export", "ranges", "Resolviendo tramos a conservar…", 15.0);
    let keep = resolve_keep_ranges(None, segments.as_deref(), explicit)?;

    jobs.check()?;
    let msg = if audio.enabled {
        "Codificando con cortes + audio enhance…"
    } else {
        "Codificando con cortes…"
    };
    progress::emit(&app, "export", "encode", msg, 35.0);

    let out = export_keep_ranges_with_audio(
        PathBuf::from(&media_path).as_path(),
        PathBuf::from(&output_path).as_path(),
        &keep,
        &export_opts,
        &color,
        Some(&audio),
        has_audio,
        Some(&jobs),
    )
    .await?;

    progress::emit(&app, "export", "done", "Export listo", 100.0);

    let plan = estimate_from_keep(&keep);
    Ok(ExportResult {
        output_path: out.to_string_lossy().into_owned(),
        duration: plan.estimated_duration,
        keep_count: plan.keep_ranges.len(),
    })
}

/// Returns keep ranges so the UI preview can skip cut segments without re-encoding.
#[tauri::command]
pub fn preview_skip_cuts(
    segments: Option<Vec<Segment>>,
    keep_ranges: Option<Vec<[f64; 2]>>,
) -> AppResult<PreviewSkipPlan> {
    let keep = if let Some(kr) = keep_ranges {
        merge_keep_ranges(kr.into_iter().map(|r| (r[0], r[1])).collect())
    } else if let Some(segs) = segments {
        keep_ranges_from_segments(&segs)
    } else {
        Vec::new()
    };
    let estimated_duration = keep.iter().map(|(s, e)| e - s).sum();
    Ok(PreviewSkipPlan {
        keep_ranges: keep.into_iter().map(|(s, e)| [s, e]).collect(),
        estimated_duration,
    })
}

#[tauri::command]
pub fn estimate_export(
    segments: Option<Vec<Segment>>,
    keep_ranges: Option<Vec<[f64; 2]>>,
    source_duration: f64,
) -> AppResult<ExportEstimate> {
    let plan = if let Some(kr) = keep_ranges {
        let keep = merge_keep_ranges(kr.into_iter().map(|r| (r[0], r[1])).collect());
        estimate_from_keep(&keep)
    } else if let Some(segs) = segments {
        estimate_export_plan(&segs)
    } else {
        estimate_from_keep(&[])
    };
    Ok(ExportEstimate {
        estimated_duration: plan.estimated_duration,
        keep_ranges: plan
            .keep_ranges
            .into_iter()
            .map(|(s, e)| [s, e])
            .collect(),
        cut_duration: (source_duration - plan.estimated_duration).max(0.0),
        source_duration,
    })
}
