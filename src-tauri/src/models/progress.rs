//! Lightweight job progress for UI (analysis / clipping / export).

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    /// analysis | clipping | export | batch
    pub job: String,
    /// Machine stage id: probe, audio, vad, whisper, policy, candidates, encode, done
    pub stage: String,
    pub message: String,
    /// 0..100
    pub percent: f64,
}

impl JobProgress {
    pub fn new(job: impl Into<String>, stage: impl Into<String>, message: impl Into<String>, percent: f64) -> Self {
        Self {
            job: job.into(),
            stage: stage.into(),
            message: message.into(),
            percent: percent.clamp(0.0, 100.0),
        }
    }
}

pub const PROGRESS_EVENT: &str = "vigilcut://progress";

pub fn emit_progress(app: &AppHandle, progress: JobProgress) {
    let _ = app.emit(PROGRESS_EVENT, progress);
}

pub fn emit(
    app: &AppHandle,
    job: &str,
    stage: &str,
    message: &str,
    percent: f64,
) {
    emit_progress(app, JobProgress::new(job, stage, message, percent));
}
