use crate::models::event::{Event, TYPE_AUDIO_SILENCE};

pub use crate::models::event::TYPE_AUDIO_BREATH;

/// Tag short mid-speech silences as breath / micro-pause candidates.
/// Does not invent new spans — re-emits refined events from short silences.
pub fn detect_breaths(run_id: &str, events: &mut Vec<Event>) {
    let short_silences: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == TYPE_AUDIO_SILENCE)
        .filter(|e| {
            let d = e.span.duration();
            d >= 0.12 && d < 0.45
        })
        .cloned()
        .collect();

    for ev in short_silences {
        let d = ev.span.duration();
        let score = (0.55 + (0.45 - d)).clamp(0.5, 0.75);
        events.push(
            Event::new(
                run_id,
                TYPE_AUDIO_BREATH,
                "breath@1.0.0",
                ev.span,
                score,
                serde_json::json!({
                    "duration": d,
                    "parentSilenceId": ev.id,
                    "note": "micro-pause / possible breath",
                }),
            )
            .with_tag("breath")
            .with_tag("optional_trim"),
        );
    }
}
