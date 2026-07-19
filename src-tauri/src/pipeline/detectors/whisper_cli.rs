//! Optional captions via external Whisper CLI if installed on PATH.
//! Supports: `whisper` (openai-whisper) or `whisper-cli` (whisper.cpp).

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{AppError, AppResult};
use crate::pipeline::features::ensure_audio_16k;
use crate::state::AppState;

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
pub struct CaptionResult {
    pub srt_path: PathBuf,
    pub method: String,
}

/// Try to generate SRT for media. Returns None if no whisper binary found.
pub async fn try_generate_srt(media_path: &Path) -> AppResult<Option<CaptionResult>> {
    let whisper = which::which("whisper-cli")
        .or_else(|_| which::which("whisper"))
        .ok();
    let Some(bin) = whisper else {
        return Ok(None);
    };

    let wav = ensure_audio_16k(media_path).await?;
    let out_dir = AppState::cache_dir()?.join("captions");
    std::fs::create_dir_all(&out_dir)?;
    let stem = media_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cap");
    let out_base = out_dir.join(stem);

    let bin_name = bin.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let status = if bin_name.contains("whisper-cli") || bin_name == "main" {
        // whisper.cpp style
        cmd_hidden(&bin)
            .args([
                "-f",
                &wav.to_string_lossy(),
                "-of",
                &out_base.to_string_lossy(),
                "-osrt",
                "-l",
                "auto",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| AppError::Message(e.to_string()))?
    } else {
        // openai-whisper python CLI
        cmd_hidden(&bin)
            .args([
                wav.to_string_lossy().as_ref(),
                "--model",
                "base",
                "--output_dir",
                &out_dir.to_string_lossy(),
                "--output_format",
                "srt",
                "--verbose",
                "False",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .status()
            .await
            .map_err(|e| AppError::Message(e.to_string()))?
    };

    if !status.success() {
        return Err(AppError::Message("whisper CLI failed".into()));
    }

    // Find srt
    let candidates = [
        out_base.with_extension("srt"),
        out_dir.join(format!("{stem}.srt")),
        out_dir.join(
            wav.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("audio_16k"),
        )
        .with_extension("srt"),
    ];
    for c in candidates {
        if c.is_file() {
            return Ok(Some(CaptionResult {
                srt_path: c,
                method: bin_name.to_string(),
            }));
        }
    }
    // scan dir for newest srt
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(&out_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("srt") {
                if let Ok(m) = e.metadata() {
                    if let Ok(t) = m.modified() {
                        if newest.as_ref().map(|(ot, _)| t > *ot).unwrap_or(true) {
                            newest = Some((t, p));
                        }
                    }
                }
            }
        }
    }
    if let Some((_, p)) = newest {
        return Ok(Some(CaptionResult {
            srt_path: p,
            method: bin_name.to_string(),
        }));
    }

    Ok(None)
}
