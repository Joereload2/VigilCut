//! L2 — Policies convert Events into EditOps + Exceptions.
//!
//! Detectors never decide cuts. Every auto-cut / exception path goes through
//! a policy registered by `event_type`. New event types scale by adding a
//! policy here — not by branching in the engine.

use crate::models::edl::{
    EditOp, EditOpKind, ExceptionItem, ExceptionReason, ExceptionResolution, PolicyConfig,
};
use crate::models::event::{Event, TYPE_AUDIO_SILENCE};

use super::detectors::TYPE_SPEECH_FILLER;

/// Result of applying one or more policies.
#[derive(Debug, Default)]
pub struct PolicyOutcome {
    pub ops: Vec<EditOp>,
    pub exceptions: Vec<ExceptionItem>,
}

impl PolicyOutcome {
    pub fn merge(mut self, other: PolicyOutcome) -> Self {
        self.ops.extend(other.ops);
        self.exceptions.extend(other.exceptions);
        self
    }
}

/// Policy for a single namespaced event type.
pub trait EventPolicy: Send + Sync {
    fn event_type(&self) -> &'static str;
    fn apply(&self, events: &[Event], config: &PolicyConfig) -> PolicyOutcome;
}

struct SilencePolicy;
struct FillerPolicy;

impl EventPolicy for SilencePolicy {
    fn event_type(&self) -> &'static str {
        TYPE_AUDIO_SILENCE
    }

    fn apply(&self, events: &[Event], config: &PolicyConfig) -> PolicyOutcome {
        let mut out = PolicyOutcome::default();
        for ev in events.iter().filter(|e| e.event_type == TYPE_AUDIO_SILENCE) {
            if ev.score >= config.auto_approve_min_score {
                out.ops.push(EditOp::remove(
                    ev.span,
                    vec![ev.id.clone()],
                    format!(
                        "Auto-cut silence (score heurístico {:.0}, {:.2}s)",
                        ev.score * 100.0,
                        ev.span.duration()
                    ),
                    true,
                ));
            } else {
                out.exceptions.push(ExceptionItem::new(
                    vec![ev.id.clone()],
                    ExceptionReason::LowConfidence,
                    ev.span,
                    ev.score,
                    format!(
                        "Silencio dudoso (score {:.0} < umbral {:.0}). ¿Cortar? (estimación operativa, no probabilidad científica)",
                        ev.score * 100.0,
                        config.auto_approve_min_score * 100.0
                    ),
                ));
            }
        }
        out
    }
}

impl EventPolicy for FillerPolicy {
    fn event_type(&self) -> &'static str {
        TYPE_SPEECH_FILLER
    }

    fn apply(&self, events: &[Event], config: &PolicyConfig) -> PolicyOutcome {
        let mut out = PolicyOutcome::default();
        // Slightly higher bar for removing speech-like content
        let thr = (config.auto_approve_min_score + 0.05).min(0.95);
        for ev in events.iter().filter(|e| e.event_type == TYPE_SPEECH_FILLER) {
            if ev.score >= thr {
                out.ops.push(EditOp::remove(
                    ev.span,
                    vec![ev.id.clone()],
                    format!(
                        "Auto-cut filler ({:.0}%): {}",
                        ev.score * 100.0,
                        ev.payload
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?")
                    ),
                    true,
                ));
            }
            // Low-confidence fillers: leave in (no exception spam for speech)
        }
        out
    }
}

/// Built-in factory policies. Add new event types here — one place.
fn builtin_policies() -> Vec<Box<dyn EventPolicy>> {
    vec![Box::new(SilencePolicy), Box::new(FillerPolicy)]
}

/// Apply all registered policies to the event set.
pub fn apply_policies(events: &[Event], config: &PolicyConfig) -> PolicyOutcome {
    let mut outcome = PolicyOutcome::default();
    for policy in builtin_policies() {
        // Only run if events of that type exist (cheap filter)
        let t = policy.event_type();
        if events.iter().any(|e| e.event_type == t) {
            outcome = outcome.merge(policy.apply(events, config));
        }
    }
    outcome
}

/// Effective remove spans = auto ops + human-accepted exceptions.
/// Pending and rejected exceptions do **not** cut (conservative factory default).
pub fn effective_removes(ops: &[EditOp], exceptions: &[ExceptionItem]) -> Vec<(f64, f64)> {
    let mut remove: Vec<(f64, f64)> = ops
        .iter()
        .filter(|o| o.op == EditOpKind::RemoveSpan)
        .map(|o| (o.span.start, o.span.end))
        .collect();

    for ex in exceptions {
        if matches!(ex.resolution, ExceptionResolution::Accepted) {
            remove.push((ex.span.start, ex.span.end));
        }
    }

    remove
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::{Span, TYPE_AUDIO_SPEECH};

    #[test]
    fn silence_auto_and_exception() {
        let events = vec![
            Event::new(
                "r",
                TYPE_AUDIO_SILENCE,
                "t",
                Span::new(1.0, 2.0),
                0.95,
                serde_json::json!({}),
            ),
            Event::new(
                "r",
                TYPE_AUDIO_SILENCE,
                "t",
                Span::new(3.0, 3.5),
                0.60,
                serde_json::json!({}),
            ),
            Event::new(
                "r",
                TYPE_AUDIO_SPEECH,
                "t",
                Span::new(0.0, 1.0),
                0.9,
                serde_json::json!({}),
            ),
        ];
        let out = apply_policies(&events, &PolicyConfig::default());
        assert_eq!(out.ops.len(), 1);
        assert_eq!(out.exceptions.len(), 1);
    }

    #[test]
    fn pending_exception_does_not_remove() {
        let ops = vec![EditOp::remove(
            Span::new(1.0, 2.0),
            vec!["a".into()],
            "auto",
            true,
        )];
        let mut ex = ExceptionItem::new(
            vec!["b".into()],
            ExceptionReason::LowConfidence,
            Span::new(5.0, 6.0),
            0.5,
            "?",
        );
        // still pending
        let rem = effective_removes(&ops, &[ex.clone()]);
        assert_eq!(rem.len(), 1);
        assert!((rem[0].0 - 1.0).abs() < 0.01);

        ex.resolution = ExceptionResolution::Accepted;
        let rem2 = effective_removes(&ops, &[ex]);
        assert_eq!(rem2.len(), 2);
    }
}
