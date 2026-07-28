//! Single authorized bridge from `visual_library` product code to
//! `pipeline::visual::library` (legacy SQLite owner).
//!
//! Product modules under `visual_library/` must import legacy library helpers
//! only through this adapter. Exception (documented in CYCLE-003): test
//! harness calls in `infrastructure/providers/pollinations.rs` may still use
//! `lock_library_for_test` / `set_library_root_override` directly.

pub use crate::pipeline::visual::library::{
    get_asset_by_id, import_image_detailed, library_root, open_db, record_usage, AssetUsageRow,
    ImportOutcome,
};

#[cfg(test)]
pub use crate::pipeline::visual::library::{lock_library_for_test, set_library_root_override};

#[cfg(test)]
mod architecture_tests {
    use std::path::PathBuf;

    /// CYCLE-003 Paso 1: only legacy_adapter (product) and pollinations tests
    /// may import `pipeline::visual::library` from inside visual_library/.
    #[test]
    fn visual_library_imports_pipeline_library_only_via_adapter() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/visual_library");
        let mut offenders = Vec::new();
        fn walk(dir: &std::path::Path, offenders: &mut Vec<String>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, offenders);
                    continue;
                }
                if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                    continue;
                }
                let rel = path.to_string_lossy().replace('\\', "/");
                if rel.ends_with("/legacy_adapter.rs") {
                    continue;
                }
                // Documented harness exception (CYCLE-003).
                if rel.ends_with("/providers/pollinations.rs") {
                    continue;
                }
                let Ok(src) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (idx, line) in src.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("//") {
                        continue;
                    }
                    if trimmed.contains("pipeline::visual::library") {
                        offenders.push(format!("{rel}:{}: {trimmed}", idx + 1));
                    }
                }
            }
        }
        walk(&root, &mut offenders);
        assert!(
            offenders.is_empty(),
            "visual_library must not import pipeline::visual::library outside legacy_adapter \
             (and pollinations test harness):\n{}",
            offenders.join("\n")
        );
    }
}
