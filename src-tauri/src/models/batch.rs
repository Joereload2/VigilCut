use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::exception_mode::ExceptionHandlingMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Queued,
    Running,
    /// One or more files need human review before final export (Supervised mode).
    NeedsReview,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchFileResult {
    pub media_path: String,
    pub ok: bool,
    pub output_path: Option<String>,
    pub auto_cuts: usize,
    pub exceptions_pending: usize,
    pub exceptions_forced: usize,
    pub source_duration: f64,
    pub output_duration: f64,
    pub error: Option<String>,
    /// safe | supervised | aggressive
    #[serde(default)]
    pub exception_mode: String,
    /// true if export skipped because supervised + pending exceptions
    #[serde(default)]
    pub needs_review: bool,
    /// true if pending exceptions were kept (conservative export)
    #[serde(default)]
    pub conservative_export: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchJob {
    pub id: String,
    pub media_paths: Vec<String>,
    pub preset_id: String,
    pub output_dir: String,
    pub status: BatchStatus,
    pub progress: f64,
    pub current_file: Option<String>,
    pub completed: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    pub results: Vec<BatchFileResult>,
    /// Legacy flag — prefer `exception_mode`. If present without mode, maps via from_auto_accept.
    #[serde(default)]
    pub auto_accept_exceptions: bool,
    #[serde(default)]
    pub exception_mode: ExceptionHandlingMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BatchJob {
    pub fn new(
        media_paths: Vec<String>,
        preset_id: String,
        output_dir: String,
        exception_mode: ExceptionHandlingMode,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            media_paths,
            preset_id,
            output_dir,
            status: BatchStatus::Queued,
            progress: 0.0,
            current_file: None,
            completed: 0,
            failed: 0,
            errors: Vec::new(),
            results: Vec::new(),
            auto_accept_exceptions: exception_mode.is_aggressive(),
            exception_mode,
            created_at: now,
            updated_at: now,
        }
    }

    /// Resolve effective mode (legacy boolean + new enum).
    pub fn effective_mode(&self) -> ExceptionHandlingMode {
        // Prefer explicit non-default mode, else map legacy flag for old clients
        if self.exception_mode != ExceptionHandlingMode::Safe {
            return self.exception_mode;
        }
        if self.auto_accept_exceptions {
            ExceptionHandlingMode::Aggressive
        } else {
            ExceptionHandlingMode::Safe
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
