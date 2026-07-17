use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{AppError, AppResult};
use crate::models::media::MediaInfo;

#[derive(Debug, Clone)]
pub struct FfmpegPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

impl FfmpegPaths {
    /// Resolve bundled sidecars first, then system PATH.
    pub fn resolve() -> AppResult<Self> {
        let sidecar_dir = Self::sidecar_dir();

        let ffmpeg = Self::find_binary("ffmpeg", &sidecar_dir)?;
        let ffprobe = Self::find_binary("ffprobe", &sidecar_dir)?;

        Ok(Self { ffmpeg, ffprobe })
    }

    fn sidecar_dir() -> PathBuf {
        // In Tauri, externalBin is next to the executable under binaries/
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let candidates = [
                    dir.join("binaries"),
                    dir.to_path_buf(),
                    // Dev layout: src-tauri/binaries
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
                ];
                for c in candidates {
                    if c.exists() {
                        return c;
                    }
                }
            }
        }
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
    }

    fn find_binary(name: &str, sidecar_dir: &Path) -> AppResult<PathBuf> {
        #[cfg(windows)]
        let exe_name = format!("{name}.exe");
        #[cfg(not(windows))]
        let exe_name = name.to_string();

        // Target-triple suffix used by Tauri externalBin in some layouts
        let triple = std::env::var("TAURI_ENV_TARGET_TRIPLE").ok();
        let mut candidates: Vec<PathBuf> = vec![sidecar_dir.join(&exe_name)];

        if let Some(t) = &triple {
            candidates.push(sidecar_dir.join(format!("{name}-{t}")));
            #[cfg(windows)]
            candidates.push(sidecar_dir.join(format!("{name}-{t}.exe")));
        }

        // Common Tauri externalBin naming
        #[cfg(windows)]
        {
            candidates.push(sidecar_dir.join(format!("{name}-x86_64-pc-windows-msvc.exe")));
        }
        #[cfg(target_os = "macos")]
        {
            candidates.push(sidecar_dir.join(format!("{name}-x86_64-apple-darwin")));
            candidates.push(sidecar_dir.join(format!("{name}-aarch64-apple-darwin")));
        }
        #[cfg(target_os = "linux")]
        {
            candidates.push(sidecar_dir.join(format!("{name}-x86_64-unknown-linux-gnu")));
        }

        for c in &candidates {
            if c.is_file() {
                return Ok(c.clone());
            }
        }

        // Fall back to PATH
        which::which(name).map_err(|_| {
            AppError::Ffmpeg(format!(
                "{name} not found. Place a binary in src-tauri/binaries/ or install system-wide. Run: npm run setup:ffmpeg"
            ))
        })
    }
}

pub struct Ffmpeg {
    paths: FfmpegPaths,
}

impl Ffmpeg {
    pub fn new() -> AppResult<Self> {
        Ok(Self {
            paths: FfmpegPaths::resolve()?,
        })
    }

    pub fn paths(&self) -> &FfmpegPaths {
        &self.paths
    }

    pub async fn version(&self) -> AppResult<String> {
        let output = Command::new(&self.paths.ffmpeg)
            .arg("-version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.lines().next().unwrap_or("ffmpeg").to_string())
    }

    pub async fn probe(&self, path: &Path) -> AppResult<MediaInfo> {
        let output = Command::new(&self.paths.ffprobe)
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
            ])
            .arg(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::Ffmpeg(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        parse_probe_json(path, &json)
    }

    pub async fn run(&self, args: &[String]) -> AppResult<std::process::Output> {
        let output = Command::new(&self.paths.ffmpeg)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::Ffmpeg(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        }
        Ok(output)
    }

    /// Extract mono 16kHz PCM WAV for VAD / analysis.
    pub async fn extract_audio_wav(&self, input: &Path, output: &Path) -> AppResult<()> {
        let args = vec![
            "-y".into(),
            "-i".into(),
            input.to_string_lossy().into_owned(),
            "-vn".into(),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            "16000".into(),
            "-c:a".into(),
            "pcm_s16le".into(),
            output.to_string_lossy().into_owned(),
        ];
        self.run(&args).await?;
        Ok(())
    }

    /// FFmpeg silencedetect fallback (when Silero model is missing).
    pub async fn detect_silences_ffmpeg(
        &self,
        input: &Path,
        noise_db: f64,
        min_duration: f64,
    ) -> AppResult<Vec<(f64, f64)>> {
        let filter = format!("silencedetect=noise={noise_db}dB:d={min_duration}");
        let output = Command::new(&self.paths.ffmpeg)
            .args([
                "-i",
                &input.to_string_lossy(),
                "-af",
                &filter,
                "-f",
                "null",
                "-",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(parse_silencedetect_log(&stderr))
    }

    pub async fn extract_waveform_peaks(
        &self,
        input: &Path,
        peaks_per_second: u32,
    ) -> AppResult<(Vec<f32>, f64)> {
        let info = self.probe(input).await?;
        let duration = info.duration;
        if duration <= 0.0 || !info.has_audio {
            return Ok((Vec::new(), duration));
        }

        // Downmix to mono f32 and sample roughly at peaks_per_second
        let target_rate = peaks_per_second.max(20);
        let args = vec![
            "-i".into(),
            input.to_string_lossy().into_owned(),
            "-vn".into(),
            "-ac".into(),
            "1".into(),
            "-ar".into(),
            target_rate.to_string(),
            "-f".into(),
            "f32le".into(),
            "-".into(),
        ];

        let output = Command::new(&self.paths.ffmpeg)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        if !output.status.success() {
            return Err(AppError::Ffmpeg("waveform extraction failed".into()));
        }

        let bytes = output.stdout;
        let mut peaks = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            peaks.push(sample.abs().clamp(0.0, 1.0));
        }
        Ok((peaks, duration))
    }

    pub async fn thumbnail(&self, input: &Path, time: f64, output: &Path) -> AppResult<()> {
        let args = vec![
            "-y".into(),
            "-ss".into(),
            format!("{time:.3}"),
            "-i".into(),
            input.to_string_lossy().into_owned(),
            "-frames:v".into(),
            "1".into(),
            "-q:v".into(),
            "3".into(),
            output.to_string_lossy().into_owned(),
        ];
        self.run(&args).await?;
        Ok(())
    }
}

fn parse_probe_json(path: &Path, json: &serde_json::Value) -> AppResult<MediaInfo> {
    let format = json.get("format").cloned().unwrap_or(serde_json::json!({}));
    let streams = json
        .get("streams")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();

    let video = streams.iter().find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("video"));
    let audio = streams.iter().find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("audio"));

    let duration = format
        .get("duration")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| {
            video
                .and_then(|v| v.get("duration"))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(0.0);

    let fps = video
        .and_then(|v| v.get("avg_frame_rate"))
        .and_then(|v| v.as_str())
        .and_then(parse_frame_rate)
        .unwrap_or(0.0);

    let size_bytes = format
        .get("size")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok())
        .or_else(|| std::fs::metadata(path).ok().map(|m| m.len()))
        .unwrap_or(0);

    Ok(MediaInfo {
        path: path.to_string_lossy().into_owned(),
        duration,
        width: video
            .and_then(|v| v.get("width"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        height: video
            .and_then(|v| v.get("height"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        fps,
        video_codec: video
            .and_then(|v| v.get("codec_name"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        audio_codec: audio
            .and_then(|v| v.get("codec_name"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        sample_rate: audio
            .and_then(|v| v.get("sample_rate"))
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok()),
        channels: audio
            .and_then(|v| v.get("channels"))
            .and_then(|v| v.as_u64())
            .map(|n| n as u32),
        bitrate: format
            .get("bit_rate")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok()),
        has_audio: audio.is_some(),
        has_video: video.is_some(),
        format_name: format
            .get("format_name")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        size_bytes,
    })
}

fn parse_frame_rate(s: &str) -> Option<f64> {
    if let Some((n, d)) = s.split_once('/') {
        let n: f64 = n.parse().ok()?;
        let d: f64 = d.parse().ok()?;
        if d != 0.0 {
            return Some(n / d);
        }
    }
    s.parse().ok()
}

/// Parse pairs (silence_start, silence_end) from ffmpeg silencedetect stderr.
pub fn parse_silencedetect_log(log: &str) -> Vec<(f64, f64)> {
    let mut starts: Vec<f64> = Vec::new();
    let mut ranges: Vec<(f64, f64)> = Vec::new();

    let re_start = regex::Regex::new(r"silence_start:\s*([0-9.]+)").unwrap();
    let re_end = regex::Regex::new(r"silence_end:\s*([0-9.]+)").unwrap();

    for line in log.lines() {
        if let Some(c) = re_start.captures(line) {
            if let Ok(t) = c[1].parse::<f64>() {
                starts.push(t);
            }
        }
        if let Some(c) = re_end.captures(line) {
            if let Ok(end) = c[1].parse::<f64>() {
                let start = starts.pop().unwrap_or(0.0);
                ranges.push((start, end));
            }
        }
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_silence_log() {
        let log = r#"
[silencedetect @ 0x] silence_start: 1.2
[silencedetect @ 0x] silence_end: 2.5 | silence_duration: 1.3
[silencedetect @ 0x] silence_start: 10.0
[silencedetect @ 0x] silence_end: 11.1 | silence_duration: 1.1
"#;
        let ranges = parse_silencedetect_log(log);
        assert_eq!(ranges.len(), 2);
        assert!((ranges[0].0 - 1.2).abs() < 0.001);
        assert!((ranges[0].1 - 2.5).abs() < 0.001);
    }
}
