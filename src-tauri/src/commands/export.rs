use std::path::PathBuf;

use serde::Serialize;

use crate::error::AppResult;
use crate::models::preset::{ColorOptions, ExportOptions};
use crate::models::segment::Segment;
use crate::pipeline::export::{
    estimate_export as estimate_export_plan, export_with_cuts, keep_ranges_from_segments,
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

#[tauri::command]
pub async fn export_video(
    media_path: String,
    output_path: String,
    segments: Vec<Segment>,
    export_options: Option<ExportOptions>,
    color_options: Option<ColorOptions>,
    has_audio: Option<bool>,
) -> AppResult<ExportResult> {
    let export_opts = export_options.unwrap_or_default();
    let color = color_options.unwrap_or_default();
    let has_audio = has_audio.unwrap_or(true);

    let out = export_with_cuts(
        PathBuf::from(&media_path).as_path(),
        PathBuf::from(&output_path).as_path(),
        &segments,
        &export_opts,
        &color,
        has_audio,
    )
    .await?;

    let plan = estimate_export_plan(&segments);
    Ok(ExportResult {
        output_path: out.to_string_lossy().into_owned(),
        duration: plan.estimated_duration,
        keep_count: plan.keep_ranges.len(),
    })
}

/// Returns keep ranges so the UI preview can skip cut segments without re-encoding.
#[tauri::command]
pub fn preview_skip_cuts(segments: Vec<Segment>) -> AppResult<PreviewSkipPlan> {
    let keep = keep_ranges_from_segments(&segments);
    let estimated_duration = keep.iter().map(|(s, e)| e - s).sum();
    Ok(PreviewSkipPlan {
        keep_ranges: keep.into_iter().map(|(s, e)| [s, e]).collect(),
        estimated_duration,
    })
}

#[tauri::command]
pub fn estimate_export(segments: Vec<Segment>, source_duration: f64) -> AppResult<ExportEstimate> {
    let plan = estimate_export_plan(&segments);
    let keep_dur = plan.estimated_duration;
    Ok(ExportEstimate {
        estimated_duration: keep_dur,
        keep_ranges: plan
            .keep_ranges
            .into_iter()
            .map(|(s, e)| [s, e])
            .collect(),
        cut_duration: (source_duration - keep_dur).max(0.0),
        source_duration,
    })
}
