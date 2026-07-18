use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::state::AppState;

/// Stable cache key from path + size + mtime (good enough for factory).
pub fn media_cache_key(path: &Path) -> AppResult<String> {
    let meta = std::fs::metadata(path)?;
    let modified = meta
        .modified()
        .unwrap_or(SystemTime::UNIX_EPOCH)
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut h = DefaultHasher::new();
    path.to_string_lossy().hash(&mut h);
    meta.len().hash(&mut h);
    modified.hash(&mut h);
    Ok(format!("{:016x}", h.finish()))
}

/// Ensure 16 kHz mono WAV exists under cache; returns path.
pub async fn ensure_audio_16k(media_path: &Path) -> AppResult<PathBuf> {
    let key = media_cache_key(media_path)?;
    let dir = AppState::cache_dir()?.join("features").join(&key);
    std::fs::create_dir_all(&dir)?;
    let wav = dir.join("audio_16k.wav");
    if wav.is_file() && wav.metadata().map(|m| m.len() > 44).unwrap_or(false) {
        return Ok(wav);
    }
    let ffmpeg = Ffmpeg::new()?;
    ffmpeg.extract_audio_wav(media_path, &wav).await?;
    Ok(wav)
}
