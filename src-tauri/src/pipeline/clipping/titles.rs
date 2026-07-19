//! Human-readable clip titles: transcript snippet or numbered fallback.

use crate::models::clipping::ClipCandidate;

/// After scoring/dedupe: give each primary clip a stable label.
/// - Real speech text → `01. primeras palabras…`
/// - Speech-fallback placeholders → `Clip 01`
pub fn finalize_clip_titles(candidates: &mut [ClipCandidate]) {
    let mut primaries: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_primary_variant)
        .map(|(i, _)| i)
        .collect();
    primaries.sort_by(|&a, &b| {
        candidates[a]
            .start
            .partial_cmp(&candidates[b].start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (n, &idx) in primaries.iter().enumerate() {
        let num = n + 1;
        let c = &mut candidates[idx];
        if is_placeholder_text(&c.transcript) || is_placeholder_text(&c.title) {
            c.title = format!("Clip {num:02}");
            c.summary = format!(
                "{}–{} · {:.0}s",
                format_mmss(c.start),
                format_mmss(c.end),
                c.duration
            );
        } else {
            let phrase = content_title(&c.transcript);
            c.title = format!("{num:02}. {phrase}");
            if c.summary.trim().is_empty() || is_placeholder_text(&c.summary) {
                c.summary = snippet(&c.transcript, 140);
            }
        }
    }

    // Variants: same number family as primary when possible
    for i in 0..candidates.len() {
        if candidates[i].is_primary_variant {
            continue;
        }
        let gid = candidates[i].variant_group_id.clone();
        let parent_title = candidates
            .iter()
            .find(|p| p.is_primary_variant && p.variant_group_id == gid)
            .map(|p| p.title.clone())
            .unwrap_or_else(|| "Clip".into());
        let base = parent_title.split(" · ").next().unwrap_or(&parent_title);
        candidates[i].title = format!("{base} · var");
    }
}

fn is_placeholder_text(s: &str) -> bool {
    let t = s.trim().to_lowercase();
    if t.is_empty() {
        return true;
    }
    t.contains("[habla")
        || t.contains("[segmento")
        || t.starts_with("habla ")
        || t.chars().filter(|c| c.is_alphabetic()).count() < 3
}

fn content_title(text: &str) -> String {
    let clean = text
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let clean = clean
        .trim_matches(|c: char| c == '"' || c == '\'' || c == '[' || c == ']' || c == '«' || c == '»');
    // Prefer up to first sentence end, max ~8 words
    let cut = clean
        .find(['.', '?', '!', '…'])
        .map(|i| &clean[..=i])
        .unwrap_or(clean);
    let words: Vec<&str> = cut.split_whitespace().take(8).collect();
    if words.is_empty() {
        return "Momento".into();
    }
    let mut t = words.join(" ");
    if clean.split_whitespace().count() > 8 && !t.ends_with(['.', '?', '!', '…']) {
        t.push('…');
    }
    // Capitalize first letter
    let mut chars = t.chars();
    match chars.next() {
        None => "Momento".into(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn snippet(text: &str, max: usize) -> String {
    let clean = text.replace(['\n', '\r'], " ");
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() <= max {
        return clean;
    }
    let t: String = clean.chars().take(max.saturating_sub(1)).collect();
    format!("{t}…")
}

fn format_mmss(t: f64) -> String {
    let s = t.max(0.0).floor() as u64;
    let m = s / 60;
    let sec = s % 60;
    if m >= 60 {
        let h = m / 60;
        let m = m % 60;
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::clipping::{
        ClipFraming, ClipReviewStatus, ClipScoreBreakdown,
    };

    fn cand(id: &str, start: f64, text: &str, primary: bool) -> ClipCandidate {
        ClipCandidate {
            id: id.into(),
            analysis_run_id: "r".into(),
            source_media_path: "m".into(),
            start,
            end: start + 20.0,
            duration: 20.0,
            transcript: text.into(),
            title: text.into(),
            summary: text.into(),
            score: 60.0,
            confidence: 0.5,
            breakdown: ClipScoreBreakdown::default(),
            reasons: vec![],
            warnings: vec![],
            strengths: vec![],
            risks: vec![],
            status: ClipReviewStatus::Suggested,
            variant_group_id: id.into(),
            is_primary_variant: primary,
            framing: ClipFraming::default(),
            original_start: start,
            original_end: start + 20.0,
            export_path: None,
            error: None,
        }
    }

    #[test]
    fn numbers_placeholder_speech() {
        let mut list = vec![
            cand("a", 10.0, "[habla 10.0s–30.0s]", true),
            cand("b", 40.0, "[habla 40.0s–60.0s] [habla 61.0s–70.0s]", true),
        ];
        finalize_clip_titles(&mut list);
        assert_eq!(list[0].title, "Clip 01");
        assert_eq!(list[1].title, "Clip 02");
        assert!(list[0].summary.contains("0:10"));
    }

    #[test]
    fn uses_transcript_words() {
        let mut list = vec![cand(
            "a",
            0.0,
            "La clave del marketing es escuchar al cliente antes de vender.",
            true,
        )];
        finalize_clip_titles(&mut list);
        assert!(list[0].title.starts_with("01. "));
        assert!(list[0].title.to_lowercase().contains("clave") || list[0].title.to_lowercase().contains("marketing"));
    }
}
