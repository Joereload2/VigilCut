use std::path::PathBuf;

use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::subtitle::{SubtitleCue, SubtitleSource, SubtitleTrack, WhisperOptions};

/// Import SRT/VTT subtitles for burn-in or side-by-side review.
#[tauri::command]
pub fn import_subtitles(path: String, language: Option<String>) -> AppResult<SubtitleTrack> {
    let p = PathBuf::from(&path);
    if !p.is_file() {
        return Err(AppError::NotFound(path));
    }
    let text = std::fs::read_to_string(&p)?;
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let cues = match ext.as_str() {
        "srt" => parse_srt(&text),
        "vtt" => parse_vtt(&text),
        _ => {
            // Try SRT first
            let c = parse_srt(&text);
            if c.is_empty() {
                parse_vtt(&text)
            } else {
                c
            }
        }
    };

    Ok(SubtitleTrack {
        language: language.unwrap_or_else(|| "und".into()),
        source: SubtitleSource::Upload,
        cues,
        path: Some(path),
    })
}

/// Whisper auto-subtitles — architecture stub for local model runner.
#[tauri::command]
pub fn generate_subtitles_whisper(
    _media_path: String,
    options: Option<WhisperOptions>,
) -> AppResult<SubtitleTrack> {
    let opts = options.unwrap_or(WhisperOptions {
        model: "base".into(),
        language: None,
        translate_to_english: false,
    });

    Err(AppError::Message(format!(
        "Whisper integration planned (model: {}). Place models under app data /models and enable in a future release. For now, import SRT/VTT via import_subtitles.",
        opts.model
    )))
}

fn parse_srt(text: &str) -> Vec<SubtitleCue> {
    let mut cues = Vec::new();
    let normalized = text.replace("\r\n", "\n");
    let blocks: Vec<&str> = normalized.split("\n\n").map(str::trim).collect();

    for block in blocks {
        if block.is_empty() {
            continue;
        }
        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 2 {
            continue;
        }
        // Find timing line
        let timing_idx = lines.iter().position(|l| l.contains("-->")).unwrap_or(1);
        let timing = lines.get(timing_idx).copied().unwrap_or("");
        if let Some((start, end)) = parse_srt_times(timing) {
            let text_lines: Vec<&str> = lines.iter().skip(timing_idx + 1).copied().collect();
            if text_lines.is_empty() {
                continue;
            }
            cues.push(SubtitleCue {
                id: Uuid::new_v4().to_string(),
                start,
                end,
                text: text_lines.join("\n"),
            });
        }
    }
    cues
}

fn parse_vtt(text: &str) -> Vec<SubtitleCue> {
    let cleaned = text
        .replace("\r\n", "\n")
        .lines()
        .filter(|l| !l.starts_with("WEBVTT") && !l.starts_with("NOTE") && !l.starts_with("STYLE"))
        .collect::<Vec<_>>()
        .join("\n");
    parse_srt(&cleaned)
}

fn parse_srt_times(line: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = line.split("-->").map(str::trim).collect();
    if parts.len() < 2 {
        return None;
    }
    let start = parse_timestamp(parts[0])?;
    // VTT may have settings after end time
    let end_raw = parts[1].split_whitespace().next()?;
    let end = parse_timestamp(end_raw)?;
    Some((start, end))
}

fn parse_timestamp(s: &str) -> Option<f64> {
    // 00:00:01,000 or 00:00:01.000 or 00:01.000
    let s = s.replace(',', ".");
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        3 => {
            let h: f64 = parts[0].parse().ok()?;
            let m: f64 = parts[1].parse().ok()?;
            let sec: f64 = parts[2].parse().ok()?;
            Some(h * 3600.0 + m * 60.0 + sec)
        }
        2 => {
            let m: f64 = parts[0].parse().ok()?;
            let sec: f64 = parts[1].parse().ok()?;
            Some(m * 60.0 + sec)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_srt() {
        let srt = "1\n00:00:01,000 --> 00:00:03,500\nHello world\n\n2\n00:00:04,000 --> 00:00:05,000\nBye\n";
        let cues = parse_srt(srt);
        assert_eq!(cues.len(), 2);
        assert!((cues[0].start - 1.0).abs() < 0.001);
        assert_eq!(cues[0].text, "Hello world");
    }
}
