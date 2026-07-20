use serde::Serialize;
use tauri::State;

use crate::error::AppResult;
use crate::ffmpeg::Ffmpeg;
use crate::job_control::JobControl;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub os: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub available: bool,
    pub ffmpeg_path: Option<String>,
    pub ffprobe_path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspacePaths {
    pub app_data: String,
    pub projects: String,
    pub presets: String,
    pub cache: String,
    pub models: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "VigilCut".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        os: std::env::consts::OS.into(),
    }
}

#[tauri::command]
pub async fn check_ffmpeg() -> AppResult<FfmpegStatus> {
    match Ffmpeg::new() {
        Ok(ff) => {
            let paths = ff.paths().clone();
            match ff.version().await {
                Ok(version) => Ok(FfmpegStatus {
                    available: true,
                    ffmpeg_path: Some(paths.ffmpeg.to_string_lossy().into_owned()),
                    ffprobe_path: Some(paths.ffprobe.to_string_lossy().into_owned()),
                    version: Some(version),
                    error: None,
                }),
                Err(e) => Ok(FfmpegStatus {
                    available: false,
                    ffmpeg_path: Some(paths.ffmpeg.to_string_lossy().into_owned()),
                    ffprobe_path: Some(paths.ffprobe.to_string_lossy().into_owned()),
                    version: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        Err(e) => Ok(FfmpegStatus {
            available: false,
            ffmpeg_path: None,
            ffprobe_path: None,
            version: None,
            error: Some(e.to_string()),
        }),
    }
}

#[tauri::command]
pub fn get_workspace_paths(_state: State<'_, AppState>) -> AppResult<WorkspacePaths> {
    Ok(WorkspacePaths {
        app_data: AppState::app_data_dir()?.to_string_lossy().into_owned(),
        projects: AppState::projects_dir()?.to_string_lossy().into_owned(),
        presets: AppState::presets_dir()?.to_string_lossy().into_owned(),
        cache: AppState::cache_dir()?.to_string_lossy().into_owned(),
        models: AppState::models_dir()?.to_string_lossy().into_owned(),
    })
}

/// Cancel the current long job (analysis / clipping / export encode).
#[tauri::command]
pub fn cancel_job(jobs: State<'_, JobControl>) -> AppResult<()> {
    jobs.request_cancel();
    Ok(())
}
