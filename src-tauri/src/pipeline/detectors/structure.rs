use uuid::Uuid;

use crate::models::artifacts::{ChapterMarker, ShortCandidate};
use crate::models::event::{
    Event, Span, TYPE_AUDIO_SILENCE, TYPE_AUDIO_SPEECH, TYPE_STRUCTURE_SHORT,
};

pub use crate::models::event::TYPE_STRUCTURE_CHAPTER;
/// Canonical short-candidate event type (alias of model constant).
pub const TYPE_SHORT_CANDIDATE: &str = TYPE_STRUCTURE_SHORT;

/// Chapter candidates: long silence gaps between speech blocks → topic breaks.
pub fn detect_chapters(run_id: &str, _duration: f64, events: &mut Vec<Event>) {
    let mut chapter_idx = 0usize;
    // Opening chapter at 0 if there is speech
    if events.iter().any(|e| e.event_type == TYPE_AUDIO_SPEECH) {
        events.push(
            Event::new(
                run_id,
                TYPE_STRUCTURE_CHAPTER,
                "structure@1.0.0",
                Span::new(0.0, 0.0),
                0.7,
                serde_json::json!({ "index": 0, "title": "Inicio" }),
            )
            .with_tag("chapter"),
        );
        chapter_idx = 1;
    }

    for ev in events.clone() {
        if ev.event_type != TYPE_AUDIO_SILENCE {
            continue;
        }
        // Long pause ≈ possible topic change
        if ev.span.duration() >= 1.4 && ev.score >= 0.75 {
            let title = format!("Parte {}", chapter_idx + 1);
            events.push(
                Event::new(
                    run_id,
                    TYPE_STRUCTURE_CHAPTER,
                    "structure@1.0.0",
                    Span::new(ev.span.end, ev.span.end),
                    (0.55 + ev.span.duration() * 0.05).min(0.9),
                    serde_json::json!({
                        "index": chapter_idx,
                        "title": title,
                        "gap": ev.span.duration(),
                    }),
                )
                .with_tag("chapter"),
            );
            chapter_idx += 1;
        }
    }
}

/// Short candidates: continuous speech blocks 12–60s (vertical clip density).
pub fn detect_short_candidates(run_id: &str, _duration: f64, events: &mut Vec<Event>) {
    let speech: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == TYPE_AUDIO_SPEECH)
        .cloned()
        .collect();
    for ev in speech {
        let d = ev.span.duration();
        if !(12.0..=75.0).contains(&d) {
            continue;
        }
        // Prefer denser mid-length talking segments
        let score = if (20.0..45.0).contains(&d) {
            0.78
        } else if (12.0..20.0).contains(&d) {
            0.65
        } else {
            0.6
        };
        events.push(
            Event::new(
                run_id,
                TYPE_SHORT_CANDIDATE,
                "shorts@1.0.0",
                ev.span,
                score,
                serde_json::json!({
                    "duration": d,
                    "reason": "speech_block_length",
                }),
            )
            .with_tag("short"),
        );
    }
}

/// Map chapter events → markers on *output* timeline given keep ranges.
pub fn chapters_from_events(events: &[Event], keep: &[(f64, f64)]) -> Vec<ChapterMarker> {
    let mut markers = Vec::new();
    for ev in events
        .iter()
        .filter(|e| e.event_type == TYPE_STRUCTURE_CHAPTER)
    {
        let at_source = ev.span.start;
        let Some(at_output) = source_to_output(at_source, keep) else {
            continue;
        };
        let title = ev
            .payload
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Capítulo")
            .to_string();
        let index = ev
            .payload
            .get("index")
            .and_then(|v| v.as_u64())
            .unwrap_or(markers.len() as u64) as usize;
        markers.push(ChapterMarker {
            index,
            title,
            at_output,
            at_source,
        });
    }
    markers.sort_by(|a, b| {
        a.at_output
            .partial_cmp(&b.at_output)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // reindex
    for (i, m) in markers.iter_mut().enumerate() {
        m.index = i;
    }
    markers
}

pub fn shorts_from_events(events: &[Event]) -> Vec<ShortCandidate> {
    let mut out: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == TYPE_SHORT_CANDIDATE)
        .map(|e| ShortCandidate {
            id: e.id.clone(),
            start: e.span.start,
            end: e.span.end,
            score: e.score,
            reason: e
                .payload
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("speech_block")
                .to_string(),
        })
        .collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Stable ids if empty from clone — use uuid for safety
    for s in &mut out {
        if s.id.is_empty() {
            s.id = Uuid::new_v4().to_string();
        }
    }
    out
}

fn source_to_output(source: f64, keep: &[(f64, f64)]) -> Option<f64> {
    let mut acc = 0.0;
    for (s, e) in keep {
        if source < *s {
            return Some(acc);
        }
        if source <= *e {
            return Some(acc + (source - s));
        }
        acc += e - s;
    }
    // past end
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::TYPE_STRUCTURE_CHAPTER;

    #[test]
    fn maps_chapter_into_output_timeline() {
        let events = vec![
            Event::new(
                "r",
                TYPE_STRUCTURE_CHAPTER,
                "t",
                Span::new(0.0, 0.0),
                0.8,
                serde_json::json!({ "index": 0, "title": "Inicio" }),
            ),
            Event::new(
                "r",
                TYPE_STRUCTURE_CHAPTER,
                "t",
                Span::new(5.0, 5.0),
                0.8,
                serde_json::json!({ "index": 1, "title": "Parte 2" }),
            ),
        ];
        // Keep [0,2] and [4,6] → source 5.0 maps into second keep at output 2+(5-4)=3
        let keep = vec![(0.0, 2.0), (4.0, 6.0)];
        let chapters = chapters_from_events(&events, &keep);
        assert_eq!(chapters.len(), 2);
        assert!((chapters[0].at_output - 0.0).abs() < 0.01);
        assert!((chapters[1].at_output - 3.0).abs() < 0.01);
        assert_eq!(chapters[1].title, "Parte 2");
    }

    #[test]
    fn drops_chapter_inside_removed_span_maps_to_boundary() {
        // Chapter at 3.0 is inside removed gap (2,4); maps to start of next keep → output 2.0
        let events = vec![Event::new(
            "r",
            TYPE_STRUCTURE_CHAPTER,
            "t",
            Span::new(3.0, 3.0),
            0.8,
            serde_json::json!({ "title": "Mid" }),
        )];
        let keep = vec![(0.0, 2.0), (4.0, 6.0)];
        let chapters = chapters_from_events(&events, &keep);
        assert_eq!(chapters.len(), 1);
        assert!((chapters[0].at_output - 2.0).abs() < 0.01);
    }
}
