use serde::{Deserialize, Serialize};

/// Published or intermediate factory output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub kind: String,
    pub path: String,
    pub label: Option<String>,
}

/// Chapter marker on the *output* timeline (after cuts).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterMarker {
    pub index: usize,
    pub title: String,
    /// Time in the exported (cut) video
    pub at_output: f64,
    /// Source media time where chapter starts
    pub at_source: f64,
}

/// Vertical short candidate on source timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortCandidate {
    pub id: String,
    pub start: f64,
    pub end: f64,
    pub score: f64,
    pub reason: String,
}

pub const ART_LONGFORM: &str = "longform_mp4";
pub const ART_MANIFEST: &str = "manifest_json";
pub const ART_CHAPTERS: &str = "chapters_json";
pub const ART_SHORTS: &str = "shorts_json";
pub const ART_EVENTS: &str = "events_json";
pub const ART_EDL: &str = "edl_json";
