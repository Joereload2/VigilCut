//! Detector plugins — each emits Events only.
//!
//! Register new enrichers in `enrich_events`. Full async media detectors
//! (Silero, Whisper) plug in at the feature-extraction stage later.

mod breath;
mod structure;

use crate::models::event::Event;

/// Enrich base timeline events with secondary detectors.
pub fn enrich_events(run_id: &str, duration: f64, events: &mut Vec<Event>) {
    breath::detect_breaths(run_id, events);
    structure::detect_chapters(run_id, duration, events);
    structure::detect_short_candidates(run_id, duration, events);
}

pub use structure::{chapters_from_events, shorts_from_events};
