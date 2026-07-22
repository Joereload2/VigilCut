//! Build canonical Transcript from SRT path, sidecar, or Whisper CLI.

use std::path::Path;

use crate::error::{AppError, AppResult};
use crate::models::transcript::{Transcript, TranscriptSegment, TranscriptStatus};
use crate::pipeline::clipping::{load_transcript_cues, parse_srt_cues};
use crate::pipeline::detectors::whisper_cli;

/// Build transcript: explicit path → sidecar → optional whisper → empty with warning.
pub async fn build_transcript(
    media_path: &Path,
    explicit_srt: Option<&Path>,
    prefer_whisper: bool,
    run_id: Option<String>,
) -> AppResult<Transcript> {
    build_transcript_with_progress(
        media_path,
        explicit_srt,
        prefer_whisper,
        run_id,
        &mut |_, _, _| {},
    )
    .await
}

pub async fn build_transcript_with_progress(
    media_path: &Path,
    explicit_srt: Option<&Path>,
    prefer_whisper: bool,
    run_id: Option<String>,
    on_progress: &mut whisper_cli::WhisperProgressFn<'_>,
) -> AppResult<Transcript> {
    let mut tr = Transcript::new(media_path.to_string_lossy(), "none");
    tr.run_id = run_id;

    // 1) Explicit
    if let Some(p) = explicit_srt {
        if p.is_file() {
            on_progress("transcript", "Leyendo SRT…", 30.0);
            match load_from_path(p) {
                Ok(mut t) => {
                    t.media_path = media_path.to_string_lossy().into_owned();
                    t.run_id = tr.run_id.clone();
                    t.engine = format!("file:{}", p.display());
                    on_progress("transcript", "SRT cargado", 100.0);
                    return Ok(t);
                }
                Err(e) => tr.warnings.push(format!("Explicit transcript failed: {e}")),
            }
        }
    }

    // 2) Sidecar
    if let Some(side) = find_sidecar(media_path) {
        on_progress("transcript", "Leyendo subtítulos del video…", 30.0);
        match load_from_path(&side) {
            Ok(mut t) => {
                t.media_path = media_path.to_string_lossy().into_owned();
                t.run_id = tr.run_id.clone();
                t.engine = format!("sidecar:{}", side.display());
                t.warnings
                    .push(format!("Loaded sidecar {}", side.display()));
                on_progress("transcript", "Subtítulos listos", 100.0);
                return Ok(t);
            }
            Err(e) => tr.warnings.push(format!("Sidecar failed: {e}")),
        }
    }

    // 3) Whisper
    if prefer_whisper {
        match whisper_cli::try_generate_srt_with_progress(media_path, on_progress).await {
            Ok(Some(cap)) => match load_from_path(&cap.srt_path) {
                Ok(mut t) => {
                    t.media_path = media_path.to_string_lossy().into_owned();
                    t.run_id = tr.run_id.clone();
                    t.engine = format!("whisper:{}", cap.method);
                    on_progress("transcript", "Texto listo", 100.0);
                    return Ok(t);
                }
                Err(e) => tr.warnings.push(format!("Whisper SRT unreadable: {e}")),
            },
            Ok(None) => tr.warnings.push(
                "Whisper no está en PATH. Instálalo o importa un .srt para transcripción.".into(),
            ),
            Err(e) => tr.warnings.push(format!("Whisper falló: {e}")),
        }
    } else {
        tr.warnings.push(
            "Sin SRT/VTT y Whisper desactivado — no hay transcripción. El corte de silencios sigue disponible."
                .into(),
        );
    }

    tr.status = TranscriptStatus::Empty;
    tr.engine = "none".into();
    Ok(tr)
}

fn load_from_path(path: &Path) -> AppResult<Transcript> {
    let text = std::fs::read_to_string(path)?;
    let cues = if path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("vtt"))
        .unwrap_or(false)
    {
        // reuse SRT parser after strip via load_transcript_cues
        let (c, _) = load_transcript_cues(path)?;
        c
    } else {
        let c = parse_srt_cues(&text);
        if c.is_empty() {
            let (c2, _) = load_transcript_cues(path)?;
            c2
        } else {
            c
        }
    };
    if cues.is_empty() {
        return Err(AppError::Invalid("No timed cues in transcript".into()));
    }
    let mut tr = Transcript::new("pending", "file");
    for c in cues {
        tr.segments.push(TranscriptSegment::new(c.span, c.text));
    }
    tr.status = TranscriptStatus::Ready;
    tr.language = "auto".into();
    Ok(tr)
}

fn find_sidecar(media: &Path) -> Option<std::path::PathBuf> {
    let stem = media.with_extension("");
    for ext in ["srt", "vtt", "SRT", "VTT"] {
        let p = stem.with_extension(ext);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// Write projections next to a directory.
pub fn write_transcript_artifacts(
    tr: &Transcript,
    out_dir: &Path,
    stem: &str,
) -> AppResult<Vec<(String, String)>> {
    std::fs::create_dir_all(out_dir)?;
    let mut arts = Vec::new();
    let json_path = out_dir.join(format!("{stem}.transcript.json"));
    std::fs::write(&json_path, serde_json::to_string_pretty(tr)?)?;
    arts.push((
        "transcript_json".into(),
        json_path.to_string_lossy().into_owned(),
    ));
    let srt_path = out_dir.join(format!("{stem}.transcript.srt"));
    std::fs::write(&srt_path, tr.to_srt())?;
    arts.push((
        "transcript_srt".into(),
        srt_path.to_string_lossy().into_owned(),
    ));
    let txt_path = out_dir.join(format!("{stem}.transcript.txt"));
    std::fs::write(&txt_path, tr.to_txt_timed())?;
    arts.push((
        "transcript_txt".into(),
        txt_path.to_string_lossy().into_owned(),
    ));
    Ok(arts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_srt_fixture() {
        let dir = std::env::temp_dir().join(format!("vc-tr-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let srt = dir.join("t.srt");
        std::fs::write(
            &srt,
            "1\n00:00:01,000 --> 00:00:03,000\nHola inflación\n\n2\n00:00:04,000 --> 00:00:06,000\nEn los mercados\n",
        )
        .unwrap();
        let tr = load_from_path(&srt).unwrap();
        assert_eq!(tr.segments.len(), 2);
        assert!(tr.full_text().contains("inflación"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
