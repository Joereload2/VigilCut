//! Optional captions via external Whisper if installed.
//! Supports: `whisper-cli` (whisper.cpp), `whisper` (openai-whisper CLI),
//! and `python -m whisper` / `py -3 -m whisper`.
//!
//! openai-whisper shells out to `ffmpeg` on PATH to load audio — we inject the
//! app-bundled FFmpeg directory into PATH for child processes.

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::pipeline::features::ensure_audio_16k;
use crate::state::AppState;

fn cmd_hidden(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    // openai-whisper calls `ffmpeg` by name — put our sidecar first on PATH
    if let Ok(ff) = Ffmpeg::new() {
        if let Some(dir) = ff.paths().ffmpeg.parent() {
            let sep = if cfg!(windows) { ";" } else { ":" };
            let cur = std::env::var_os("PATH").unwrap_or_default();
            let mut new_path = dir.as_os_str().to_os_string();
            new_path.push(sep);
            new_path.push(&cur);
            cmd.env("PATH", new_path);
        }
    }
    cmd
}

#[derive(Debug, Clone)]
pub struct CaptionResult {
    pub srt_path: PathBuf,
    pub method: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhisperStatus {
    pub available: bool,
    pub kind: String,
    pub detail: String,
    pub install_hint: String,
}

#[derive(Debug, Clone)]
enum WhisperEngine {
    /// Direct binary: whisper-cli or whisper.exe
    Binary { path: PathBuf, name: String },
    /// python -m whisper / py -3 -m whisper
    PythonModule { python: PathBuf, via: String },
}

/// Probe whether any Whisper engine is available (does not run transcription).
pub fn whisper_status() -> WhisperStatus {
    match resolve_engine() {
        Some(WhisperEngine::Binary { path, name }) => WhisperStatus {
            available: true,
            kind: name,
            detail: path.display().to_string(),
            install_hint: String::new(),
        },
        Some(WhisperEngine::PythonModule { python, via }) => WhisperStatus {
            available: true,
            kind: via.clone(),
            detail: format!("{} -m whisper", python.display()),
            install_hint: String::new(),
        },
        None => WhisperStatus {
            available: false,
            kind: "none".into(),
            detail: "No se encontró Whisper en PATH ni como módulo de Python.".into(),
            install_hint: "npm run setup:whisper   o   python -m pip install -U openai-whisper"
                .into(),
        },
    }
}

fn resolve_engine() -> Option<WhisperEngine> {
    if let Ok(p) = std::env::var("VIGILCUT_WHISPER") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Some(WhisperEngine::Binary {
                name: pb
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("whisper")
                    .into(),
                path: pb,
            });
        }
    }

    if let Ok(p) = which::which("whisper-cli") {
        return Some(WhisperEngine::Binary {
            name: "whisper-cli".into(),
            path: p,
        });
    }
    if let Ok(p) = which::which("whisper") {
        return Some(WhisperEngine::Binary {
            name: "whisper".into(),
            path: p,
        });
    }

    for (label, prog, args_prefix) in [
        ("python -m whisper", "python", vec![] as Vec<&str>),
        ("py -3 -m whisper", "py", vec!["-3"]),
        ("python3 -m whisper", "python3", vec![]),
    ] {
        if let Ok(py) = which::which(prog) {
            if python_module_available(&py, &args_prefix) {
                return Some(WhisperEngine::PythonModule {
                    python: py,
                    via: label.into(),
                });
            }
        }
    }

    None
}

fn python_module_available(python: &Path, prefix: &[&str]) -> bool {
    let mut cmd = std::process::Command::new(python);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    for a in prefix {
        cmd.arg(a);
    }
    cmd.args(["-c", "import whisper"]);
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    matches!(cmd.status(), Ok(s) if s.success())
}

fn tail_err(bytes: &[u8], max: usize) -> String {
    let s = String::from_utf8_lossy(bytes);
    let t = s.trim();
    if t.len() <= max {
        t.to_string()
    } else {
        t.chars().rev().take(max).collect::<String>().chars().rev().collect()
    }
}

/// Shared openai-whisper argument list (CPU-friendly).
fn openai_whisper_args(wav: &Path, out_dir: &Path) -> Vec<String> {
    vec![
        wav.to_string_lossy().into_owned(),
        "--model".into(),
        // tiny/base: faster on CPU; first run downloads weights
        "base".into(),
        "--output_dir".into(),
        out_dir.to_string_lossy().into_owned(),
        "--output_format".into(),
        "srt".into(),
        "--verbose".into(),
        "False".into(),
        "--language".into(),
        "es".into(),
        "--device".into(),
        "cpu".into(),
        // Avoid half-precision issues on CPU
        "--fp16".into(),
        "False".into(),
    ]
}

/// Try to generate SRT for media. Returns None if no whisper engine found.
pub async fn try_generate_srt(media_path: &Path) -> AppResult<Option<CaptionResult>> {
    let Some(engine) = resolve_engine() else {
        return Ok(None);
    };

    // Prefetch 16 kHz wav (also validates media has audio path)
    let wav = ensure_audio_16k(media_path).await?;
    let out_dir = AppState::cache_dir()?.join("captions");
    std::fs::create_dir_all(&out_dir)?;
    let stem = media_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cap");
    let out_base = out_dir.join(stem);

    let (output, method) = match &engine {
        WhisperEngine::Binary { path, name } => {
            let mut cmd = cmd_hidden(path);
            if name.contains("whisper-cli") || name == "main" {
                cmd.args([
                    "-f",
                    &wav.to_string_lossy(),
                    "-of",
                    &out_base.to_string_lossy(),
                    "-osrt",
                    "-l",
                    "es",
                ]);
            } else {
                cmd.args(openai_whisper_args(&wav, &out_dir));
            }
            let out = cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| AppError::Message(format!("No se pudo lanzar whisper: {e}")))?;
            (out, name.clone())
        }
        WhisperEngine::PythonModule { python, via } => {
            let mut cmd = cmd_hidden(python);
            if via.starts_with("py ") {
                cmd.arg("-3");
            }
            cmd.arg("-m").arg("whisper");
            cmd.args(openai_whisper_args(&wav, &out_dir));
            let out = cmd
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| AppError::Message(format!("No se pudo lanzar python -m whisper: {e}")))?;
            (out, via.clone())
        }
    };

    if !output.status.success() {
        let err = tail_err(&output.stderr, 1200);
        let out = tail_err(&output.stdout, 400);
        let hint = if err.contains("WinError 2")
            || err.contains("cannot find the file")
            || err.to_lowercase().contains("ffmpeg")
        {
            " (Whisper necesita FFmpeg en PATH; VigilCut debería inyectarlo automáticamente — reinicia la app)"
        } else {
            ""
        };
        return Err(AppError::Message(format!(
            "Whisper falló ({method}){hint}.\n{err}\n{out}"
        )));
    }

    // Find srt — openai-whisper names file after input stem (audio_16k.srt)
    let candidates = [
        out_base.with_extension("srt"),
        out_dir.join(format!("{stem}.srt")),
        out_dir
            .join(
                wav.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("audio_16k"),
            )
            .with_extension("srt"),
        // sometimes includes full name
        out_dir.join("audio_16k.srt"),
    ];
    for c in &candidates {
        if c.is_file() && c.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            return Ok(Some(CaptionResult {
                srt_path: c.clone(),
                method: method.clone(),
            }));
        }
    }
    // newest non-empty srt in out_dir
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(&out_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("srt") {
                if let Ok(m) = e.metadata() {
                    if m.len() == 0 {
                        continue;
                    }
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
            method,
        }));
    }

    Err(AppError::Message(format!(
        "Whisper terminó pero no se encontró .srt en {}",
        out_dir.display()
    )))
}

/// Install openai-whisper via pip (local, no cloud account). Long-running.
pub async fn install_openai_whisper() -> AppResult<String> {
    let python = which::which("python")
        .or_else(|_| which::which("py"))
        .map_err(|_| {
            AppError::Invalid(
                "No hay Python en PATH. Instala Python 3 desde python.org e inténtalo de nuevo."
                    .into(),
            )
        })?;

    let mut cmd = cmd_hidden(&python);
    if python
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("py"))
        .unwrap_or(false)
    {
        cmd.arg("-3");
    }
    cmd.args(["-m", "pip", "install", "-U", "openai-whisper", "--quiet"]);
    let out = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| AppError::Message(format!("pip: {e}")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(AppError::Message(format!(
            "No se pudo instalar openai-whisper:\n{}",
            err.chars().take(800).collect::<String>()
        )));
    }
    let st = whisper_status();
    if !st.available {
        return Err(AppError::Message(
            "pip reportó éxito pero aún no se detecta whisper. Reinicia la app.".into(),
        ));
    }
    Ok(format!("Whisper listo ({})", st.kind))
}
