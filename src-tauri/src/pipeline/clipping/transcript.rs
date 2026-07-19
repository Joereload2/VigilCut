//! Transcript providers: SRT/VTT import + speech-event fallback.

use std::path::Path;

use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::clipping::{SemanticUnit, TranscriptCue, TranscriptSourceKind};
use crate::models::event::{Event, Span, TYPE_AUDIO_SPEECH};

/// Load cues from SRT or VTT path.
pub fn load_transcript_cues(path: &Path) -> AppResult<(Vec<TranscriptCue>, TranscriptSourceKind)> {
    if !path.is_file() {
        return Err(AppError::NotFound(path.display().to_string()));
    }
    let text = std::fs::read_to_string(path)?;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let kind = match ext.as_str() {
        "vtt" => TranscriptSourceKind::VttFile,
        _ => TranscriptSourceKind::SrtFile,
    };
    let cues = if ext == "vtt" {
        parse_vtt_cues(&text)
    } else {
        let c = parse_srt_cues(&text);
        if c.is_empty() {
            parse_vtt_cues(&text)
        } else {
            c
        }
    };
    if cues.is_empty() {
        return Err(AppError::Invalid(
            "No timed cues found in transcript file".into(),
        ));
    }
    Ok((cues, kind))
}

/// Build cues from silence-analysis speech events when no transcript is available.
pub fn cues_from_speech_events(events: &[Event]) -> Vec<TranscriptCue> {
    events
        .iter()
        .filter(|e| e.event_type == TYPE_AUDIO_SPEECH)
        .map(|e| {
            let text = format!(
                "[habla {:.1}s–{:.1}s]",
                e.span.start, e.span.end
            );
            TranscriptCue {
                id: e.id.clone(),
                span: e.span,
                text,
            }
        })
        .collect()
}

/// Merge cues into semantic units using pause gaps and max length.
pub fn cues_to_semantic_units(
    cues: &[TranscriptCue],
    max_gap: f64,
    max_unit_duration: f64,
) -> Vec<SemanticUnit> {
    if cues.is_empty() {
        return Vec::new();
    }
    let mut units = Vec::new();
    let mut cur_start = cues[0].span.start;
    let mut cur_end = cues[0].span.end;
    let mut texts: Vec<String> = vec![cues[0].text.clone()];
    let mut ids: Vec<String> = vec![cues[0].id.clone()];

    for c in cues.iter().skip(1) {
        let gap = c.span.start - cur_end;
        let would = c.span.end - cur_start;
        if gap > max_gap || would > max_unit_duration {
            units.push(make_unit(cur_start, cur_end, &texts, &ids));
            cur_start = c.span.start;
            cur_end = c.span.end;
            texts = vec![c.text.clone()];
            ids = vec![c.id.clone()];
        } else {
            cur_end = cur_end.max(c.span.end);
            texts.push(c.text.clone());
            ids.push(c.id.clone());
        }
    }
    units.push(make_unit(cur_start, cur_end, &texts, &ids));
    units
}

fn make_unit(start: f64, end: f64, texts: &[String], ids: &[String]) -> SemanticUnit {
    let text = texts.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    let energy = (text.chars().filter(|c| c.is_alphanumeric()).count() as f64
        / text.len().max(1) as f64)
        .clamp(0.2, 1.0);
    SemanticUnit {
        id: Uuid::new_v4().to_string(),
        span: Span::new(start, end),
        text,
        cue_ids: ids.to_vec(),
        energy,
    }
}

pub fn parse_srt_cues(text: &str) -> Vec<TranscriptCue> {
    let mut cues = Vec::new();
    let normalized = text.replace("\r\n", "\n");
    for block in normalized.split("\n\n").map(str::trim) {
        if block.is_empty() {
            continue;
        }
        let lines: Vec<&str> = block.lines().collect();
        if lines.len() < 2 {
            continue;
        }
        let timing_idx = lines.iter().position(|l| l.contains("-->")).unwrap_or(1);
        let timing = lines.get(timing_idx).copied().unwrap_or("");
        if let Some((start, end)) = parse_times(timing) {
            let body: Vec<&str> = lines.iter().skip(timing_idx + 1).copied().collect();
            if body.is_empty() {
                continue;
            }
            cues.push(TranscriptCue::new(Span::new(start, end), body.join(" ")));
        }
    }
    cues
}

fn parse_vtt_cues(text: &str) -> Vec<TranscriptCue> {
    let cleaned = text
        .lines()
        .filter(|l| !l.starts_with("WEBVTT") && !l.starts_with("NOTE") && !l.starts_with("STYLE"))
        .collect::<Vec<_>>()
        .join("\n");
    parse_srt_cues(&cleaned)
}

fn parse_times(line: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = line.split("-->").map(str::trim).collect();
    if parts.len() < 2 {
        return None;
    }
    let start = parse_ts(parts[0].split_whitespace().next()?)?;
    let end = parse_ts(parts[1].split_whitespace().next()?)?;
    Some((start, end))
}

fn parse_ts(s: &str) -> Option<f64> {
    // 00:00:01,000 or 00:01.000 or 00:01:02.500
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
        _ => s.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_srt_and_merges_units() {
        let srt = r#"1
00:00:00,000 --> 00:00:02,000
Hola a todos

2
00:00:02,200 --> 00:00:05,000
hoy les cuento un truco

3
00:00:12,000 --> 00:00:15,000
otro bloque
"#;
        let cues = parse_srt_cues(srt);
        assert_eq!(cues.len(), 3);
        let units = cues_to_semantic_units(&cues, 1.0, 30.0);
        assert_eq!(units.len(), 2);
        assert!(units[0].text.contains("Hola"));
    }
}
