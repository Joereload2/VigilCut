//! Optional captions via external Whisper if installed.
//! Supports: `whisper-cli` (whisper.cpp), `whisper` (openai-whisper CLI),
//! and `python -m whisper` / `py -3 -m whisper`.

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
    // 1) Explicit override
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

    // 2) whisper-cli / whisper on PATH
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

    // 3) python -m whisper
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

/// Try to generate SRT for media. Returns None if no whisper engine found.
pub async fn try_generate_srt(media_path: &Path) -> AppResult<Option<CaptionResult>> {
    let Some(engine) = resolve_engine() else {
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

    let (ok, method) = match &engine {
        WhisperEngine::Binary { path, name } => {
            let status = if name.contains("whisper-cli") || name == "main" {
                cmd_hidden(path)
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
                    .map_err(|e| AppError::Message(format!("whisper-cli: {e}")))?
            } else {
                // openai-whisper CLI entrypoint
                cmd_hidden(path)
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
                        "--language",
                        "es",
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .status()
                    .await
                    .map_err(|e| AppError::Message(format!("whisper: {e}")))?
            };
            (status.success(), name.clone())
        }
        WhisperEngine::PythonModule { python, via } => {
            let mut cmd = cmd_hidden(python);
            // py -3 -m whisper ...
            if via.starts_with("py ") {
                cmd.arg("-3");
            }
            cmd.args([
                "-m",
                "whisper",
                wav.to_string_lossy().as_ref(),
                "--model",
                "base",
                "--output_dir",
                &out_dir.to_string_lossy(),
                "--output_format",
                "srt",
                "--verbose",
                "False",
                "--language",
                "es",
            ]);
            let status = cmd
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .status()
                .await
                .map_err(|e| AppError::Message(format!("python whisper: {e}")))?;
            (status.success(), via.clone())
        }
    };

    if !ok {
        return Err(AppError::Message(
            "Whisper falló al transcribir. Revisa el audio o prueba con un .srt.".into(),
        ));
    }

    // Find srt
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
    ];
    for c in candidates {
        if c.is_file() {
            return Ok(Some(CaptionResult {
                srt_path: c,
                method: method.clone(),
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
            method,
        }));
    }

    Err(AppError::Message(
        "Whisper terminó pero no se encontró el archivo .srt de salida.".into(),
    ))
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
    // If launcher is `py`, need -3
    if python
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("py"))
        .unwrap_or(false)
    {
        cmd.arg("-3");
    }
    cmd.args([
        "-m",
        "pip",
        "install",
        "-U",
        "openai-whisper",
        "--quiet",
    ]);
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
    // Verify
    let st = whisper_status();
    if !st.available {
        return Err(AppError::Message(
            "pip reportó éxito pero aún no se detecta whisper. Reinicia la app.".into(),
        ));
    }
    Ok(format!("Whisper listo ({})", st.kind))
}
