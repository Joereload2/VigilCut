use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::error::{AppError, AppResult};
use crate::models::batch::BatchJob;
use crate::models::project::Project;

#[derive(Default)]
pub struct AppState {
    pub projects: Mutex<HashMap<String, Project>>,
    pub batch_jobs: Mutex<HashMap<String, BatchJob>>,
}

impl AppState {
    pub fn app_data_dir() -> AppResult<PathBuf> {
        let base = dirs::data_dir()
            .ok_or_else(|| AppError::Message("Cannot resolve data directory".into()))?;
        Ok(base.join("VigilCut"))
    }

    pub fn ensure_dirs(&self) -> AppResult<()> {
        let root = Self::app_data_dir()?;
        for sub in ["projects", "presets", "cache", "exports", "temp", "models"] {
            std::fs::create_dir_all(root.join(sub))?;
        }
        Ok(())
    }

    pub fn projects_dir() -> AppResult<PathBuf> {
        Ok(Self::app_data_dir()?.join("projects"))
    }

    pub fn presets_dir() -> AppResult<PathBuf> {
        Ok(Self::app_data_dir()?.join("presets"))
    }

    pub fn cache_dir() -> AppResult<PathBuf> {
        Ok(Self::app_data_dir()?.join("cache"))
    }

    pub fn temp_dir() -> AppResult<PathBuf> {
        Ok(Self::app_data_dir()?.join("temp"))
    }

    pub fn models_dir() -> AppResult<PathBuf> {
        Ok(Self::app_data_dir()?.join("models"))
    }
}
