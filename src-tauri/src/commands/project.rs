use std::path::PathBuf;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::project::{Project, ProjectSummary};
use crate::state::AppState;

#[tauri::command]
pub async fn create_project(
    name: String,
    media_path: String,
    state: State<'_, AppState>,
) -> AppResult<Project> {
    let mut project = Project::new(name, &media_path);

    // Probe media if FFmpeg available
    if let Ok(ffmpeg) = Ffmpeg::new() {
        if let Ok(info) = ffmpeg.probe(PathBuf::from(&media_path).as_path()).await {
            project.media = Some(info);
        }
    }

    let dir = AppState::projects_dir()?.join(&project.id);
    std::fs::create_dir_all(&dir)?;
    project.work_dir = Some(dir.to_string_lossy().into_owned());

    let path = dir.join("project.json");
    std::fs::write(&path, serde_json::to_string_pretty(&project)?)?;

    state
        .projects
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .insert(project.id.clone(), project.clone());

    Ok(project)
}

#[tauri::command]
pub fn load_project(id: String, state: State<'_, AppState>) -> AppResult<Project> {
    {
        let cache = state
            .projects
            .lock()
            .map_err(|e| AppError::Message(e.to_string()))?;
        if let Some(p) = cache.get(&id) {
            return Ok(p.clone());
        }
    }

    let path = AppState::projects_dir()?.join(&id).join("project.json");
    if !path.exists() {
        return Err(AppError::NotFound(format!("Project {id}")));
    }
    let data = std::fs::read_to_string(path)?;
    let project: Project = serde_json::from_str(&data)?;

    state
        .projects
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .insert(project.id.clone(), project.clone());

    Ok(project)
}

#[tauri::command]
pub fn save_project(mut project: Project, state: State<'_, AppState>) -> AppResult<Project> {
    project.touch();
    let dir = AppState::projects_dir()?.join(&project.id);
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("project.json");
    std::fs::write(&path, serde_json::to_string_pretty(&project)?)?;

    state
        .projects
        .lock()
        .map_err(|e| AppError::Message(e.to_string()))?
        .insert(project.id.clone(), project.clone());

    Ok(project)
}

#[tauri::command]
pub fn list_recent_projects() -> AppResult<Vec<ProjectSummary>> {
    let dir = AppState::projects_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let project_file = entry.path().join("project.json");
        if !project_file.is_file() {
            continue;
        }
        if let Ok(data) = std::fs::read_to_string(project_file) {
            if let Ok(p) = serde_json::from_str::<Project>(&data) {
                list.push(ProjectSummary {
                    id: p.id,
                    name: p.name,
                    media_path: p.media_path,
                    updated_at: p.updated_at,
                    mode: p.mode,
                });
            }
        }
    }

    list.sort_by_key(|a| std::cmp::Reverse(a.updated_at));
    Ok(list)
}
