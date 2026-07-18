//! Detector plugins — each emits Events only.
//!
//! Add a new detector by implementing `StructureEnricher` (or a full async
//! media detector later) and registering it in `enrich_events`.

mod structure;

use crate::models::event::Event;

/// Enrich base timeline events with structure detectors (chapters, shorts…).
pub fn enrich_events(run_id: &str, duration: f64, events: &mut Vec<Event>) {
    structure::detect_chapters(run_id, duration, events);
    structure::detect_short_candidates(run_id, duration, events);
}

pub use structure::{chapters_from_events, shorts_from_events};
