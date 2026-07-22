//! Path validation and atomic export helpers.
//!
//! Invariants:
//! - Never modify the original media file.
//! - Write to a temp path, validate, then rename into place.
//! - Never silently overwrite an existing destination.

use std::path::{Path, PathBuf};

use crate::error::{AppError, AppResult};

/// Canonicalize when possible; fall back to normalized string compare.
pub fn paths_same_file(a: &Path, b: &Path) -> bool {
    if let (Ok(ca), Ok(cb)) = (a.canonicalize(), b.canonicalize()) {
        return ca == cb;
    }
    let na = normalize_path_str(&a.to_string_lossy());
    let nb = normalize_path_str(&b.to_string_lossy());
    na == nb
}

fn normalize_path_str(s: &str) -> String {
    s.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Validate export request before any encode.
pub fn validate_export_request(input: &Path, output: &Path) -> AppResult<()> {
    if !input.exists() {
        return Err(AppError::NotFound(format!(
            "Archivo de entrada no existe: {}",
            input.display()
        )));
    }
    if !input.is_file() {
        return Err(AppError::Invalid(format!(
            "La entrada no es un archivo: {}",
            input.display()
        )));
    }
    // Readable check
    let meta = std::fs::metadata(input).map_err(|e| {
        AppError::Invalid(format!(
            "No se puede leer la entrada {}: {e}",
            input.display()
        ))
    })?;
    if meta.len() == 0 {
        return Err(AppError::Invalid("El archivo de entrada está vacío".into()));
    }
    if paths_same_file(input, output) {
        return Err(AppError::Invalid(
            "La salida no puede ser el mismo archivo que la entrada (el original no se modifica)"
                .into(),
        ));
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    Ok(())
}

/// If `desired` exists, produce `stem-editado-2.mp4`, `stem-editado-3.mp4`, …
pub fn unique_output_path(desired: &Path) -> PathBuf {
    if !desired.exists() {
        return desired.to_path_buf();
    }
    let parent = desired.parent().unwrap_or_else(|| Path::new("."));
    let stem = desired
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = desired
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    for n in 2..10_000 {
        let candidate = parent.join(format!("{stem}-{n}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}-{}.{}", uuid::Uuid::new_v4().simple(), ext))
}

/// Temp path next to final output (same volume → atomic rename on Windows/Unix).
pub fn temp_export_path(final_out: &Path) -> PathBuf {
    let parent = final_out.parent().unwrap_or_else(|| Path::new("."));
    let stem = final_out
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("export");
    let ext = final_out
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");
    let id = uuid::Uuid::new_v4().simple();
    parent.join(format!(".{stem}.vigilcut-tmp-{id}.{ext}"))
}

/// Minimum plausible finished MP4 size (headers + tiny content).
pub const MIN_VALID_EXPORT_BYTES: u64 = 1024;

pub fn validate_export_output(path: &Path, estimated_duration_secs: f64) -> AppResult<()> {
    if !path.exists() {
        return Err(AppError::Ffmpeg(format!(
            "Export terminó sin archivo: {}",
            path.display()
        )));
    }
    let meta = std::fs::metadata(path)?;
    let len = meta.len();
    if len < MIN_VALID_EXPORT_BYTES {
        let _ = std::fs::remove_file(path);
        return Err(AppError::Ffmpeg(format!(
            "Export inválido (archivo demasiado pequeño: {len} bytes). No se guardó el resultado."
        )));
    }
    // Soft check: very long sources shouldn't produce tiny files if estimate is large
    if estimated_duration_secs > 30.0 && len < 8_000 {
        let _ = std::fs::remove_file(path);
        return Err(AppError::Ffmpeg(
            "Export inválido (tamaño inconsistente con la duración). No se guardó el resultado."
                .into(),
        ));
    }
    Ok(())
}

/// Rename temp → final. Cleans temp on failure.
pub fn finalize_atomic(temp: &Path, final_out: &Path) -> AppResult<()> {
    if final_out.exists() {
        // Caller should have used unique_output_path; still refuse silent overwrite.
        let _ = std::fs::remove_file(temp);
        return Err(AppError::Invalid(format!(
            "El destino ya existe (no se sobrescribe): {}",
            final_out.display()
        )));
    }
    match std::fs::rename(temp, final_out) {
        Ok(()) => Ok(()),
        Err(e) => {
            // Cross-device fallback: copy then remove temp
            if e.kind() == std::io::ErrorKind::Other || e.raw_os_error() == Some(17) {
                // EXDEV
                if let Err(ce) = std::fs::copy(temp, final_out) {
                    let _ = std::fs::remove_file(temp);
                    return Err(AppError::Io(ce));
                }
                let _ = std::fs::remove_file(temp);
                return Ok(());
            }
            // Windows may use ERROR_NOT_SAME_DEVICE
            #[cfg(windows)]
            {
                const ERROR_NOT_SAME_DEVICE: i32 = 17;
                if e.raw_os_error() == Some(ERROR_NOT_SAME_DEVICE) {
                    if let Err(ce) = std::fs::copy(temp, final_out) {
                        let _ = std::fs::remove_file(temp);
                        return Err(AppError::Io(ce));
                    }
                    let _ = std::fs::remove_file(temp);
                    return Ok(());
                }
            }
            let _ = std::fs::remove_file(temp);
            Err(AppError::Io(e))
        }
    }
}

pub fn cleanup_temp(path: &Path) {
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn rejects_same_input_output() {
        let dir = std::env::temp_dir().join(format!("vc-same-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.mp4");
        std::fs::write(&f, b"not-empty-content-here").unwrap();
        let err = validate_export_request(&f, &f).unwrap_err();
        assert!(err.to_string().contains("mismo archivo") || err.to_string().contains("original"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_path_avoids_collision() {
        let dir = std::env::temp_dir().join(format!("vc-uniq-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("out.mp4");
        std::fs::write(&a, b"x").unwrap();
        let b = unique_output_path(&a);
        assert_ne!(a, b);
        assert!(!b.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_tiny_export() {
        let dir = std::env::temp_dir().join(format!("vc-tiny-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("tiny.mp4");
        let mut file = std::fs::File::create(&f).unwrap();
        file.write_all(b"xx").unwrap();
        drop(file);
        assert!(validate_export_output(&f, 10.0).is_err());
        assert!(!f.exists()); // cleaned
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_input() {
        let p = PathBuf::from("Z:/definitely/missing/vigilcut-xyz.mp4");
        assert!(validate_export_request(&p, Path::new("out.mp4")).is_err());
    }
}
