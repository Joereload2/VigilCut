use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::media::MediaInfo;
use super::preset::ProcessingPreset;
use super::segment::Segment;
use super::subtitle::SubtitleTrack;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub media_path: String,
    pub media: Option<MediaInfo>,
    pub segments: Vec<Segment>,
    pub preset: ProcessingPreset,
    pub subtitles: Option<SubtitleTrack>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Working directory for caches (waveforms, thumbs)
    pub work_dir: Option<String>,
    pub notes: Option<String>,
    /// Mode: silence_cut | clip_select | full
    pub mode: ProjectMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMode {
    #[default]
    SilenceCut,
    ClipSelect,
    Full,
}

impl Project {
    pub fn new(name: impl Into<String>, media_path: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            media_path: media_path.into(),
            media: None,
            segments: Vec::new(),
            preset: ProcessingPreset::default(),
            subtitles: None,
            created_at: now,
            updated_at: now,
            work_dir: None,
            notes: None,
            mode: ProjectMode::SilenceCut,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub media_path: String,
    pub updated_at: DateTime<Utc>,
    pub mode: ProjectMode,
}
