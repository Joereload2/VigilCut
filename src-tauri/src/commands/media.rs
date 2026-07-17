use std::path::PathBuf;

use tauri::State;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::models::media::{MediaInfo, ThumbnailResult, WaveformData};
use crate::state::AppState;

#[tauri::command]
pub async fn probe_media(path: String) -> AppResult<MediaInfo> {
    let ffmpeg = Ffmpeg::new()?;
    ffmpeg.probe(PathBuf::from(path).as_path()).await
}

#[tauri::command]
pub async fn extract_waveform(
    path: String,
    peaks_per_second: Option<u32>,
) -> AppResult<WaveformData> {
    let ffmpeg = Ffmpeg::new()?;
    let p = PathBuf::from(&path);
    let pps = peaks_per_second.unwrap_or(100);
    let (peaks, duration) = ffmpeg.extract_waveform_peaks(&p, pps).await?;
    Ok(WaveformData {
        path,
        sample_rate: pps,
        peaks,
        duration,
    })
}

#[tauri::command]
pub async fn generate_thumbnail(
    path: String,
    time: f64,
    _state: State<'_, AppState>,
) -> AppResult<ThumbnailResult> {
    let ffmpeg = Ffmpeg::new()?;
    let cache = AppState::cache_dir()?.join("thumbs");
    std::fs::create_dir_all(&cache)?;

    let hash = simple_hash(&format!("{path}:{time:.2}"));
    let out = cache.join(format!("{hash}.jpg"));

    if !out.exists() {
        ffmpeg
            .thumbnail(PathBuf::from(&path).as_path(), time, &out)
            .await?;
    }

    Ok(ThumbnailResult {
        path,
        time,
        image_path: out.to_string_lossy().into_owned(),
    })
}

fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:x}", h.finish())
}
