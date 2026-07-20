use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{AppError, AppResult};
use crate::models::media::MediaInfo;

/// On Windows, ffmpeg/ffprobe are console apps — without this flag a black
/// terminal window flashes for every probe/export/preview call.
fn cmd_hidden(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

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
        // Prefer a directory that actually contains ffmpeg — never return the
        // test/deps folder just because it exists (that broke cargo tests).
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("binaries"));
                candidates.push(dir.to_path_buf());
                // target/release or target/debug (parent of deps when running tests)
                if let Some(parent) = dir.parent() {
                    candidates.push(parent.join("binaries"));
                    candidates.push(parent.to_path_buf());
                }
            }
        }
        candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"));

        for c in &candidates {
            if Self::dir_has_ffmpeg(c) {
                return c.clone();
            }
        }
        // Last resort: project binaries even if missing (error message later)
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
    }

    fn dir_has_ffmpeg(dir: &Path) -> bool {
        #[cfg(windows)]
        {
            dir.join("ffmpeg.exe").is_file()
                || dir.join("ffmpeg-x86_64-pc-windows-msvc.exe").is_file()
        }
        #[cfg(not(windows))]
        {
            dir.join("ffmpeg").is_file()
        }
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
        let output = cmd_hidden(&self.paths.ffmpeg)
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
        let output = cmd_hidden(&self.paths.ffprobe)
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

    /// Run ffmpeg. Global quiet flags are prepended so progress spam does not
    /// fill the stderr pipe (which can deadlock long exports on Windows).
    pub async fn run(&self, args: &[String]) -> AppResult<std::process::Output> {
        self.run_expecting(args, None).await
    }

    /// Like [`run`], but if ffmpeg exits non-zero after a finished encode
    /// (e.g. SIGTERM / signal 15 while closing), accept a valid output file.
    pub async fn run_expecting(
        &self,
        args: &[String],
        expected_output: Option<&Path>,
    ) -> AppResult<std::process::Output> {
        self.run_expecting_tracked(args, expected_output, None)
            .await
    }

    /// Same as [`run_expecting`], optionally registering the PID for cooperative cancel.
    pub async fn run_expecting_tracked(
        &self,
        args: &[String],
        expected_output: Option<&Path>,
        job: Option<&crate::job_control::JobControl>,
    ) -> AppResult<std::process::Output> {
        if let Some(j) = job {
            j.check()?;
        }

        let mut full_args: Vec<String> = Vec::with_capacity(args.len() + 4);
        // Keep banner/stats off — UI already shows its own progress.
        full_args.extend([
            "-hide_banner".into(),
            "-nostats".into(),
            "-loglevel".into(),
            "error".into(),
        ]);
        full_args.extend(args.iter().cloned());

        // kill_on_drop(false): a cancelled Tauri invoke must not SIGTERM a
        // nearly-finished export (that was showing up as "signal 15").
        let mut child = cmd_hidden(&self.paths.ffmpeg)
            .args(&full_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .kill_on_drop(false)
            .spawn()
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        if let Some(j) = job {
            j.set_ffmpeg_pid(child.id());
        }

        let stdout_pipe = child.stdout.take();
        let stderr_pipe = child.stderr.take();

        let stdout_task = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            if let Some(mut r) = stdout_pipe {
                let _ = r.read_to_end(&mut buf).await;
            }
            buf
        });
        let stderr_task = tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            let mut buf = Vec::new();
            if let Some(mut r) = stderr_pipe {
                let _ = r.read_to_end(&mut buf).await;
            }
            buf
        });

        let status = child
            .wait()
            .await
            .map_err(|e| AppError::Ffmpeg(e.to_string()))?;

        if let Some(j) = job {
            j.set_ffmpeg_pid(None);
            if j.is_cancelled() {
                return Err(AppError::Cancelled);
            }
        }

        let stdout = stdout_task.await.unwrap_or_default();
        let stderr = stderr_task.await.unwrap_or_default();
        let output = std::process::Output {
            status,
            stdout,
            stderr,
        };

        if output.status.success() {
            return Ok(output);
        }

        let err_text = String::from_utf8_lossy(&output.stderr);
        let out_text = String::from_utf8_lossy(&output.stdout);
        let combined = format!("{err_text}\n{out_text}");

        if let Some(path) = expected_output {
            if output_looks_usable(path) {
                // Encode finished but process was signalled during teardown.
                if combined.contains("signal 15")
                    || combined.contains("muxing overhead")
                    || combined.contains("Lsize=")
                    || err_text.trim().is_empty()
                {
                    tracing::warn!(
                        path = %path.display(),
                        code = ?output.status.code(),
                        "ffmpeg exited non-zero but output file is usable — treating as success"
                    );
                    return Ok(output);
                }
            }
        }

        Err(AppError::Ffmpeg(summarize_ffmpeg_error(&combined)))
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
        let output = cmd_hidden(&self.paths.ffmpeg)
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

        let output = cmd_hidden(&self.paths.ffmpeg)
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

fn output_looks_usable(path: &Path) -> bool {
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && m.len() > 8_192,
        Err(_) => false,
    }
}

/// Keep UI errors short — full ffmpeg dumps are useless for the user.
fn summarize_ffmpeg_error(log: &str) -> String {
    let mut useful: Vec<&str> = log
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            !lower.starts_with("ffmpeg version")
                && !lower.starts_with("built with")
                && !lower.starts_with("configuration:")
                && !lower.starts_with("libav")
                && !lower.starts_with("libsw")
                && !lower.starts_with("frame=")
                && !lower.starts_with("press [q]")
                && !lower.contains("copyright (c)")
                && !lower.contains("http://")
                && !lower.contains("https://")
        })
        .collect();

    // Prefer lines that look like real errors.
    let errors: Vec<&str> = useful
        .iter()
        .copied()
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            lower.contains("error")
                || lower.contains("failed")
                || lower.contains("invalid")
                || lower.contains("no such")
                || lower.contains("does not exist")
                || lower.contains("permission")
                || lower.contains("signal ")
                || lower.starts_with('[')
        })
        .collect();

    if !errors.is_empty() {
        useful = errors;
    }

    if useful.is_empty() {
        return "FFmpeg falló sin mensaje útil. Revisa el archivo de salida o vuelve a intentar."
            .into();
    }

    // Last ~8 meaningful lines are usually the real failure.
    let start = useful.len().saturating_sub(8);
    let msg = useful[start..].join("\n");
    if msg.len() > 1200 {
        format!("{}…", &msg[..1200])
    } else {
        msg
    }
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
