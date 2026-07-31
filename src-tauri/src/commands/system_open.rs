//! Open local files/folders in the OS (shell plugin open() only allows URLs).

use std::path::PathBuf;
use std::process::Command;

use crate::error::{AppError, AppResult};

/// Reveal a file or open a directory in the system file manager / default app.
#[tauri::command]
pub fn open_path_in_os(path: String) -> AppResult<()> {
    let p = PathBuf::from(path.trim());
    if p.as_os_str().is_empty() {
        return Err(AppError::Invalid("ruta vacía".into()));
    }
    if !p.exists() {
        return Err(AppError::NotFound(format!(
            "no existe: {}",
            p.display()
        )));
    }

    #[cfg(target_os = "windows")]
    {
        // explorer /select,file  OR  explorer folder
        if p.is_file() {
            Command::new("explorer")
                .arg(format!("/select,{}", p.display()))
                .spawn()
                .map_err(|e| AppError::Message(format!("explorer: {e}")))?;
        } else {
            Command::new("explorer")
                .arg(p.as_os_str())
                .spawn()
                .map_err(|e| AppError::Message(format!("explorer: {e}")))?;
        }
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(p.as_os_str())
            .spawn()
            .map_err(|e| AppError::Message(format!("open: {e}")))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(p.as_os_str())
            .spawn()
            .map_err(|e| AppError::Message(format!("xdg-open: {e}")))?;
        return Ok(());
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
    {
        Err(AppError::Message("abrir ruta no soportado en este SO".into()))
    }
}
