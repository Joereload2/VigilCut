//! Lightweight local factory metrics (no telemetry).
//!
//! Base for `human_seconds_per_media_minute`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct JobMetrics {
    pub source_duration_secs: f64,
    pub wall_clock_secs: f64,
    /// Time spent with job waiting on human (if measured).
    pub human_wait_secs: f64,
    pub exceptions_total: usize,
    pub exceptions_pending: usize,
    pub human_actions: usize,
    pub removed_duration_secs: f64,
    pub exception_mode: String,
    pub artifacts_written: usize,
    pub failures: usize,
    pub retries: usize,
    pub engine_version: String,
}

impl JobMetrics {
    pub fn human_seconds_per_media_minute(&self) -> Option<f64> {
        if self.source_duration_secs <= 0.0 {
            return None;
        }
        let minutes = self.source_duration_secs / 60.0;
        if minutes <= 0.0 {
            return None;
        }
        Some(self.human_wait_secs / minutes)
    }
}
