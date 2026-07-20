//! Intelligent clipping pipeline.
//!
//! Transcript (optional) → semantic units → candidates → score → dedupe → preselect.

mod dedupe;
mod engine;
mod export_clips;
mod framing;
mod generate;
mod preselect;
mod score;
mod titles;
mod transcript;

pub use engine::{run_clipping_analysis, run_clipping_analysis_with_progress};
pub use export_clips::{export_approved_clips, export_one_clip};
pub use framing::{compute_crop_filter, default_framing_for_media};
pub use transcript::{cues_to_semantic_units, load_transcript_cues};
