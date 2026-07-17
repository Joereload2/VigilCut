use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::models::preset::{builtin_presets, ProcessingPreset};
use crate::state::AppState;

#[tauri::command]
pub fn list_presets() -> AppResult<Vec<ProcessingPreset>> {
    let mut presets = builtin_presets();
    let dir = AppState::presets_dir()?;
    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            if let Ok(data) = std::fs::read_to_string(&path) {
                if let Ok(p) = serde_json::from_str::<ProcessingPreset>(&data) {
                    // User presets override same id
                    if let Some(pos) = presets.iter().position(|x| x.id == p.id) {
                        presets[pos] = p;
                    } else {
                        presets.push(p);
                    }
                }
            }
        }
    }
    Ok(presets)
}

#[tauri::command]
pub fn save_preset(mut preset: ProcessingPreset) -> AppResult<ProcessingPreset> {
    if preset.id.is_empty() {
        preset.id = uuid::Uuid::new_v4().to_string();
    }
    preset.is_builtin = false;
    let dir = AppState::presets_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", preset.id));
    std::fs::write(path, serde_json::to_string_pretty(&preset)?)?;
    Ok(preset)
}

#[tauri::command]
pub fn delete_preset(id: String) -> AppResult<()> {
    let path: PathBuf = AppState::presets_dir()?.join(format!("{id}.json"));
    if !path.exists() {
        return Err(AppError::NotFound(format!("Preset {id}")));
    }
    // Do not delete builtins from disk (they aren't stored)
    std::fs::remove_file(path)?;
    Ok(())
}
