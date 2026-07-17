use std::path::PathBuf;

use crate::error::AppResult;
use crate::models::segment::{SilenceDetectionOptions, SilenceDetectionResult};
use crate::pipeline::detect_and_build_segments;

#[tauri::command]
pub async fn detect_silences(
    path: String,
    options: Option<SilenceDetectionOptions>,
) -> AppResult<SilenceDetectionResult> {
    let opts = options.unwrap_or_default();
    detect_and_build_segments(PathBuf::from(path).as_path(), &opts).await
}

/// Alias used by clip-select mode: same pipeline, auto_cut disabled preferred.
#[tauri::command]
pub async fn analyze_speech_segments(
    path: String,
    options: Option<SilenceDetectionOptions>,
) -> AppResult<SilenceDetectionResult> {
    let mut opts = options.unwrap_or_default();
    opts.auto_cut_silence = false;
    detect_and_build_segments(PathBuf::from(path).as_path(), &opts).await
}
