use std::path::Path;

use crate::error::AppResult;
use crate::models::segment::{SilenceDetectionOptions, SilenceDetectionResult};
use crate::pipeline::engine::detect_and_build_segments_legacy;

/// Back-compat entry: silence detection via engine (events + policy under the hood).
pub async fn detect_and_build_segments(
    media_path: &Path,
    options: &SilenceDetectionOptions,
) -> AppResult<SilenceDetectionResult> {
    detect_and_build_segments_legacy(media_path, options).await
}
