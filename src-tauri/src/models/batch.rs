use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    Queued,
    Running,
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
    /// Factory mode: unresolved exceptions are auto-accepted as cuts
    pub auto_accept_exceptions: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BatchJob {
    pub fn new(
        media_paths: Vec<String>,
        preset_id: String,
        output_dir: String,
        auto_accept_exceptions: bool,
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
            auto_accept_exceptions,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}
