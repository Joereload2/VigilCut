use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentProjectRecord {
    pub id: String,
    pub title: String,
    pub client_label: Option<String>,
    pub privacy_mode: String,
    pub status: String,
    pub source_media_path: String,
    pub source_sha256: Option<String>,
    pub source_duration_s: Option<f64>,
    pub source_width: Option<i64>,
    pub source_height: Option<i64>,
    pub source_has_audio: Option<bool>,
    pub source_has_video: Option<bool>,
    pub source_container: Option<String>,
    pub work_dir: String,
    pub active_recipe_id: Option<String>,
    pub legacy_project_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl ContentProjectRecord {
    pub fn new_local(
        id: String,
        title: String,
        source_media_path: String,
        work_dir: String,
        now_rfc3339: String,
    ) -> Self {
        Self {
            id,
            title,
            client_label: None,
            privacy_mode: "local_only".into(),
            status: "active".into(),
            source_media_path,
            source_sha256: None,
            source_duration_s: None,
            source_width: None,
            source_height: None,
            source_has_audio: None,
            source_has_video: None,
            source_container: None,
            work_dir,
            active_recipe_id: None,
            legacy_project_id: None,
            created_at: now_rfc3339.clone(),
            updated_at: now_rfc3339,
        }
    }
}
