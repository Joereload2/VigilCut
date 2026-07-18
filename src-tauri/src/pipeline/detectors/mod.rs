//! Detector plugins — each emits Events only (or raw silence ranges for VAD).

mod breath;
mod filler;
mod silero;
mod structure;
pub mod whisper_cli;

use crate::models::event::Event;

/// Enrich base timeline events with secondary detectors.
pub fn enrich_events(run_id: &str, duration: f64, events: &mut Vec<Event>) {
    breath::detect_breaths(run_id, events);
    structure::detect_chapters(run_id, duration, events);
    structure::detect_short_candidates(run_id, duration, events);
}

pub use filler::{detect_fillers_from_srt, TYPE_SPEECH_FILLER};
pub use silero::detect_silences_silero;
pub use structure::{chapters_from_events, shorts_from_events};
