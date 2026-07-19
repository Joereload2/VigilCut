//! Filler / muletilla detection from SRT captions (when available).

use std::path::Path;

use crate::error::AppResult;
use crate::models::event::{Event, Span};

pub use crate::models::event::TYPE_SPEECH_FILLER;

const FILLERS_ES: &[&str] = &[
    "eh", "ehh", "mmm", "mm", "este", "esto", "o sea", "tipo", "digamos", "bueno", "vale",
    "entonces", "como que", "la verdad", "en plan", "pues", "a ver",
];
const FILLERS_EN: &[&str] = &[
    "uh", "um", "uhm", "like", "you know", "i mean", "sort of", "kind of", "basically",
    "actually", "right", "so",
];

/// Parse SRT and emit filler events where a cue is mostly a filler token.
pub fn detect_fillers_from_srt(run_id: &str, srt_path: &Path) -> AppResult<Vec<Event>> {
    let text = std::fs::read_to_string(srt_path)?;
    let cues = parse_srt_cues(&text);
    let mut events = Vec::new();

    for (start, end, line) in cues {
        let norm = normalize(&line);
        if norm.is_empty() {
            continue;
        }
        if is_filler_line(&norm) {
            let conf = if norm.split_whitespace().count() <= 2 {
                0.88
            } else {
                0.72
            };
            events.push(
                Event::new(
                    run_id,
                    TYPE_SPEECH_FILLER,
                    "filler@1.0.0",
                    Span::new(start, end),
                    conf,
                    serde_json::json!({ "text": line, "normalized": norm }),
                )
                .with_tag("filler")
                .with_tag("removable_candidate"),
            );
        }
    }
    Ok(events)
}

fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c.is_whitespace() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_filler_line(norm: &str) -> bool {
    if FILLERS_ES.iter().any(|f| norm == *f || norm.starts_with(&format!("{f} "))) {
        return true;
    }
    if FILLERS_EN.iter().any(|f| norm == *f || norm.starts_with(&format!("{f} "))) {
        return true;
    }
    // line is only filler words
    let words: Vec<_> = norm.split_whitespace().collect();
    if words.is_empty() || words.len() > 4 {
        return false;
    }
    words.iter().all(|w| {
        FILLERS_ES.contains(w)
            || FILLERS_EN.contains(w)
            || w.chars().all(|c| c == 'e' || c == 'h' || c == 'm' || c == 'u')
    })
}

fn parse_srt_cues(text: &str) -> Vec<(f64, f64, String)> {
    let mut out = Vec::new();
    let normalized = text.replace("\r\n", "\n");
    for block in normalized.split("\n\n") {
        let lines: Vec<&str> = block.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if lines.len() < 2 {
            continue;
        }
        let timing_idx = lines.iter().position(|l| l.contains("-->")).unwrap_or(0);
        let timing = lines[timing_idx];
        let Some((a, b)) = timing.split_once("-->") else {
            continue;
        };
        let start = parse_ts(a.trim());
        let end = parse_ts(b.trim().split_whitespace().next().unwrap_or(""));
        let (Some(start), Some(end)) = (start, end) else {
            continue;
        };
        let text_body = lines[timing_idx + 1..].join(" ");
        out.push((start, end, text_body));
    }
    out
}

fn parse_ts(s: &str) -> Option<f64> {
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
    fn detects_spanish_filler() {
        assert!(is_filler_line("este"));
        assert!(is_filler_line("o sea"));
        assert!(!is_filler_line("vamos a hablar del producto"));
    }
}
