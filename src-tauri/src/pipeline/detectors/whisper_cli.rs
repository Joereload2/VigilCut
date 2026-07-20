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
/// `verbose=True` so we can parse progress from stderr/stdout.
fn openai_whisper_args(wav: &Path, out_dir: &Path) -> Vec<String> {
    vec![
        wav.to_string_lossy().into_owned(),
        "--model".into(),
        "base".into(),
        "--output_dir".into(),
        out_dir.to_string_lossy().into_owned(),
        "--output_format".into(),
        "srt".into(),
        "--verbose".into(),
        "True".into(),
        "--language".into(),
        "es".into(),
        "--device".into(),
        "cpu".into(),
        "--fp16".into(),
        "False".into(),
    ]
}

/// Progress callback: (stage, message, percent 0..100)
pub type WhisperProgressFn<'a> = dyn FnMut(&str, &str, f64) + Send + 'a;

/// Try to generate SRT for media. Returns None if no whisper engine found.
pub async fn try_generate_srt(media_path: &Path) -> AppResult<Option<CaptionResult>> {
    try_generate_srt_with_progress(media_path, &mut |_, _, _| {}).await
}

pub async fn try_generate_srt_with_progress(
    media_path: &Path,
    on_progress: &mut WhisperProgressFn<'_>,
) -> AppResult<Option<CaptionResult>> {
    let Some(engine) = resolve_engine() else {
        return Ok(None);
    };

    on_progress("audio", "Extrayendo audio 16 kHz…", 8.0);
    let wav = ensure_audio_16k(media_path).await?;
    on_progress("audio", "Audio listo", 18.0);

    // Approximate duration from wav size (16k mono s16le ≈ 32000 bytes/s)
    let wav_bytes = std::fs::metadata(&wav).map(|m| m.len()).unwrap_or(0);
    let duration_est = if wav_bytes > 44 {
        ((wav_bytes - 44) as f64 / 32000.0).max(1.0)
    } else {
        60.0
    };

    let out_dir = AppState::cache_dir()?.join("captions");
    std::fs::create_dir_all(&out_dir)?;
    let stem = media_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("cap");
    let out_base = out_dir.join(stem);

    on_progress("whisper", "Iniciando Whisper (modelo base)…", 22.0);

    let mut cmd = match &engine {
        WhisperEngine::Binary { path, name } => {
            let mut c = cmd_hidden(path);
            if name.contains("whisper-cli") || name == "main" {
                c.args([
                    "-f",
                    &wav.to_string_lossy(),
                    "-of",
                    &out_base.to_string_lossy(),
                    "-osrt",
                    "-l",
                    "es",
                ]);
            } else {
                c.args(openai_whisper_args(&wav, &out_dir));
            }
            c
        }
        WhisperEngine::PythonModule { python, via } => {
            let mut c = cmd_hidden(python);
            if via.starts_with("py ") {
                c.arg("-3");
            }
            c.arg("-m").arg("whisper");
            c.args(openai_whisper_args(&wav, &out_dir));
            c
        }
    };

    let method = match &engine {
        WhisperEngine::Binary { name, .. } => name.clone(),
        WhisperEngine::PythonModule { via, .. } => via.clone(),
    };

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::Message(format!("No se pudo lanzar whisper: {e}")))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut err_buf = String::new();
    let mut out_buf = String::new();
    let mut last_pct = 22.0_f64;
    let started = std::time::Instant::now();

    // Merge stderr+stdout line reading
    use tokio::io::{AsyncBufReadExt, BufReader};
    let mut stdout_reader = stdout.map(|s| BufReader::new(s).lines());
    let mut stderr_reader = stderr.map(|s| BufReader::new(s).lines());

    loop {
        tokio::select! {
            line = async {
                if let Some(r) = stderr_reader.as_mut() {
                    r.next_line().await
                } else {
                    Ok(None)
                }
            } => {
                match line {
                    Ok(Some(l)) => {
                        err_buf.push_str(&l);
                        err_buf.push('\n');
                        if let Some((msg, pct)) = parse_whisper_progress_line(&l, duration_est, last_pct) {
                            last_pct = pct;
                            on_progress("whisper", &msg, pct);
                        }
                    }
                    Ok(None) => {
                        stderr_reader = None;
                        if stdout_reader.is_none() { break; }
                    }
                    Err(_) => {
                        stderr_reader = None;
                        if stdout_reader.is_none() { break; }
                    }
                }
            }
            line = async {
                if let Some(r) = stdout_reader.as_mut() {
                    r.next_line().await
                } else {
                    Ok(None)
                }
            } => {
                match line {
                    Ok(Some(l)) => {
                        out_buf.push_str(&l);
                        out_buf.push('\n');
                        if let Some((msg, pct)) = parse_whisper_progress_line(&l, duration_est, last_pct) {
                            last_pct = pct;
                            on_progress("whisper", &msg, pct);
                        }
                    }
                    Ok(None) => {
                        stdout_reader = None;
                        if stderr_reader.is_none() { break; }
                    }
                    Err(_) => {
                        stdout_reader = None;
                        if stderr_reader.is_none() { break; }
                    }
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(800)), if stdout_reader.is_some() || stderr_reader.is_some() => {
                // Heartbeat while silent: crawl toward 88% based on elapsed time
                let elapsed = started.elapsed().as_secs_f64();
                // Rough: ~0.4–1.0× realtime on CPU for base
                let est_total = (duration_est * 0.7).max(15.0);
                let crawl = 25.0 + (elapsed / est_total) * 60.0;
                let crawl = crawl.min(88.0);
                if crawl > last_pct + 0.5 {
                    last_pct = crawl;
                    on_progress(
                        "whisper",
                        &format!("Transcribiendo… {:.0}s · ~{:.0}%", elapsed, last_pct),
                        last_pct,
                    );
                }
            }
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| AppError::Message(format!("whisper wait: {e}")))?;

    if !status.success() {
        let err = tail_err(err_buf.as_bytes(), 1200);
        let out = tail_err(out_buf.as_bytes(), 400);
        let hint = if err.contains("WinError 2")
            || err.contains("cannot find the file")
            || err.to_lowercase().contains("ffmpeg")
        {
            " (FFmpeg no visible para Whisper — reinicia la app)"
        } else {
            ""
        };
        return Err(AppError::Message(format!(
            "Whisper falló ({method}){hint}.\n{err}\n{out}"
        )));
    }

    on_progress("whisper", "Leyendo subtítulos…", 92.0);

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
        out_dir.join("audio_16k.srt"),
    ];
    for c in &candidates {
        if c.is_file() && c.metadata().map(|m| m.len() > 0).unwrap_or(false) {
            on_progress("whisper", "Transcripción lista", 100.0);
            return Ok(Some(CaptionResult {
                srt_path: c.clone(),
                method: method.clone(),
            }));
        }
    }
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
        on_progress("whisper", "Transcripción lista", 100.0);
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

/// Parse tqdm percent or segment timestamps from whisper verbose output.
fn parse_whisper_progress_line(line: &str, duration_est: f64, last_pct: f64) -> Option<(String, f64)> {
    // Download / tqdm: " 45%|████..."
    if let Some(idx) = line.find('%') {
        let head = &line[..idx];
        let num: String = head
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit() || *c == ' ' || *c == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        let num = num.trim();
        if let Ok(p) = num.parse::<f64>() {
            if (0.0..=100.0).contains(&p) {
                // Map download 0-100 into UI 22-35
                let mapped = 22.0 + (p / 100.0) * 13.0;
                return Some((format!("Descargando / cargando modelo… {p:.0}%"), mapped.max(last_pct)));
            }
        }
    }

    // Segment lines: [00:12.340 --> 00:15.120] text
    // or 00:00:12,340 --> 00:00:15,120
    if let Some(end_t) = parse_timestamp_end(line) {
        if duration_est > 0.0 {
            let ratio = (end_t / duration_est).clamp(0.0, 1.0);
            let mapped = 35.0 + ratio * 52.0; // 35..87
            let mapped = mapped.max(last_pct);
            return Some((
                format!("Transcribiendo… {:.0}s / {:.0}s", end_t, duration_est),
                mapped,
            ));
        }
    }
    None
}

fn parse_timestamp_end(line: &str) -> Option<f64> {
    // Find "-->" and parse the time after it
    let idx = line.find("-->")?;
    let after = line[idx + 3..].trim();
    let token = after.split_whitespace().next()?;
    // 00:12.340 or 00:00:12,340 or 00:00:12.340
    let token = token.replace(',', ".");
    let parts: Vec<&str> = token.split(':').collect();
    match parts.len() {
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let s: f64 = parts[1].parse().ok()?;
            Some(m * 60.0 + s)
        }
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let s: f64 = parts[2].parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + s)
        }
        _ => None,
    }
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
