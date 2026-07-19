//! Detector plugins — emit Events only (never edit decisions).
//!
//! Primary VAD (silence ranges) lives in the engine / silero module.
//! Secondary detectors share [`DetectorContext`] and are run via [`run_secondary`].

mod breath;
mod filler;
mod silero;
mod structure;
pub mod whisper_cli;

use std::path::Path;

use crate::error::AppResult;
use crate::models::edl::PolicyConfig;
use crate::models::event::Event;

/// Shared context for secondary detectors. Mutates the event list in place.
pub struct DetectorContext<'a> {
    pub run_id: &'a str,
    pub media_path: &'a Path,
    pub duration: f64,
    pub policy: &'a PolicyConfig,
    pub events: &'a mut Vec<Event>,
    /// Cached 16 kHz mono WAV if available
    pub wav_path: Option<&'a Path>,
    /// Optional captions SRT from Whisper
    pub srt_path: Option<&'a Path>,
}

/// Secondary detector contract — evidence only.
pub trait Detector: Send + Sync {
    fn id(&self) -> &'static str;
    /// Whether this detector should run given context (features present, etc.).
    fn enabled(&self, _ctx: &DetectorContext<'_>) -> bool {
        true
    }
    fn run(&self, ctx: &mut DetectorContext<'_>) -> AppResult<()>;
}

struct BreathDetector;
struct ChapterDetector;
struct ShortsDetector;
struct FillerFromSrtDetector;

impl Detector for BreathDetector {
    fn id(&self) -> &'static str {
        "breath@1"
    }
    fn run(&self, ctx: &mut DetectorContext<'_>) -> AppResult<()> {
        breath::detect_breaths(ctx.run_id, ctx.events);
        Ok(())
    }
}

impl Detector for ChapterDetector {
    fn id(&self) -> &'static str {
        "structure.chapters@1"
    }
    fn run(&self, ctx: &mut DetectorContext<'_>) -> AppResult<()> {
        structure::detect_chapters(ctx.run_id, ctx.duration, ctx.events);
        Ok(())
    }
}

impl Detector for ShortsDetector {
    fn id(&self) -> &'static str {
        "structure.shorts@1"
    }
    fn run(&self, ctx: &mut DetectorContext<'_>) -> AppResult<()> {
        structure::detect_short_candidates(ctx.run_id, ctx.duration, ctx.events);
        Ok(())
    }
}

impl Detector for FillerFromSrtDetector {
    fn id(&self) -> &'static str {
        "filler.srt@1"
    }
    fn enabled(&self, ctx: &DetectorContext<'_>) -> bool {
        ctx.srt_path.is_some()
    }
    fn run(&self, ctx: &mut DetectorContext<'_>) -> AppResult<()> {
        let Some(srt) = ctx.srt_path else {
            return Ok(());
        };
        match filler::detect_fillers_from_srt(ctx.run_id, srt) {
            Ok(fillers) => {
                tracing::info!("filler detector: {} events", fillers.len());
                ctx.events.extend(fillers);
            }
            Err(e) => tracing::warn!("filler detector failed: {e}"),
        }
        Ok(())
    }
}

fn secondary_detectors() -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(BreathDetector),
        Box::new(ChapterDetector),
        Box::new(ShortsDetector),
        Box::new(FillerFromSrtDetector),
    ]
}

/// Run all registered secondary detectors in stable order.
pub fn run_secondary(ctx: &mut DetectorContext<'_>) {
    for det in secondary_detectors() {
        if !det.enabled(ctx) {
            continue;
        }
        if let Err(e) = det.run(ctx) {
            tracing::warn!("detector {} failed: {e}", det.id());
        }
    }
}

/// Legacy entry used by older call sites.
pub fn enrich_events(run_id: &str, duration: f64, events: &mut Vec<Event>) {
    let policy = PolicyConfig::default();
    let mut ctx = DetectorContext {
        run_id,
        media_path: Path::new(""),
        duration,
        policy: &policy,
        events,
        wav_path: None,
        srt_path: None,
    };
    // Only breath + structure when no srt/path (compat)
    for det in secondary_detectors() {
        if det.id().starts_with("filler") {
            continue;
        }
        let _ = det.run(&mut ctx);
    }
}

pub use filler::{detect_fillers_from_srt, TYPE_SPEECH_FILLER};
pub use silero::detect_silences_silero;
pub use structure::{chapters_from_events, shorts_from_events};
