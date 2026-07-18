use serde::Serialize;
use tauri::State;

use crate::commands::analyze::AnalysisCache;
use crate::error::{AppError, AppResult};
use crate::models::artifacts::ArtifactRef;
use crate::models::policy_pack::{builtin_policy_packs, PolicyPack};
use crate::pipeline::artifacts::write_run_artifacts;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryPaths {
    pub app_data: String,
    pub inbox: String,
    pub outbox: String,
    pub exports: String,
    pub models: String,
    pub cache: String,
}

#[tauri::command]
pub fn get_factory_paths() -> AppResult<FactoryPaths> {
    let root = AppState::app_data_dir()?;
    for sub in ["inbox", "outbox", "exports", "models", "cache"] {
        std::fs::create_dir_all(root.join(sub))?;
    }
    Ok(FactoryPaths {
        app_data: root.to_string_lossy().into_owned(),
        inbox: root.join("inbox").to_string_lossy().into_owned(),
        outbox: root.join("outbox").to_string_lossy().into_owned(),
        exports: root.join("exports").to_string_lossy().into_owned(),
        models: root.join("models").to_string_lossy().into_owned(),
        cache: root.join("cache").to_string_lossy().into_owned(),
    })
}

/// After a single-file export, write chapters/shorts/events/EDL/manifest next to the mp4.
#[tauri::command]
pub async fn write_export_artifacts(
    run_id: String,
    output_path: String,
    cache: State<'_, AnalysisCache>,
) -> AppResult<Vec<ArtifactRef>> {
    let run = {
        let map = cache
            .runs
            .lock()
            .map_err(|e| AppError::Message(e.to_string()))?;
        map.get(&run_id).cloned()
    };
    let run = run
        .or_else(|| {
            let file = AppState::cache_dir()
                .ok()?
                .join("runs")
                .join(format!("{run_id}.json"));
            let data = std::fs::read_to_string(file).ok()?;
            serde_json::from_str(&data).ok()
        })
        .ok_or_else(|| AppError::NotFound(format!("Analysis run {run_id}")))?;

    let source = std::path::PathBuf::from(&run.media_path);
    write_run_artifacts(
        &run,
        std::path::Path::new(&output_path),
        &source,
        true,
        serde_json::json!({ "singleExport": true }),
    )
    .await
}

/// Open factory inbox folder in the OS file manager.
#[tauri::command]
pub async fn open_factory_folder(which: String) -> AppResult<String> {
    let paths = get_factory_paths()?;
    let dir = match which.as_str() {
        "inbox" => paths.inbox,
        "outbox" => paths.outbox,
        "exports" => paths.exports,
        _ => paths.app_data,
    };
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[tauri::command]
pub fn list_policy_packs() -> AppResult<Vec<PolicyPack>> {
    let mut packs = builtin_policy_packs();
    if let Ok(dir) = AppState::app_data_dir().map(|d| d.join("policies")) {
        if dir.is_dir() {
            if let Ok(rd) = std::fs::read_dir(dir) {
                for e in rd.flatten() {
                    let p = e.path();
                    if p.extension().and_then(|x| x.to_str()) != Some("json") {
                        continue;
                    }
                    if let Ok(data) = std::fs::read_to_string(&p) {
                        if let Ok(mut pack) = serde_json::from_str::<PolicyPack>(&data) {
                            pack.is_builtin = false;
                            if let Some(pos) = packs.iter().position(|x| x.id == pack.id) {
                                packs[pos] = pack;
                            } else {
                                packs.push(pack);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(packs)
}

#[tauri::command]
pub fn get_policy_pack(id: String) -> AppResult<PolicyPack> {
    list_policy_packs()?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| AppError::NotFound(format!("Policy pack {id}")))
}

#[tauri::command]
pub fn save_policy_pack(mut pack: PolicyPack) -> AppResult<PolicyPack> {
    pack.is_builtin = false;
    let dir = AppState::app_data_dir()?.join("policies");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", pack.id));
    std::fs::write(path, serde_json::to_string_pretty(&pack)?)?;
    Ok(pack)
}
