use crate::error::AppResult;
use crate::visual_library::{AssetQuery, LibraryQuery, LibraryService};

/// Read-only search surface — bound to `LibraryQuery` so ingest/generation are
/// not callable through this command path.
pub fn search(query: &AssetQuery) -> AppResult<serde_json::Value> {
    let library: &dyn LibraryQuery = &LibraryService::new();
    Ok(serde_json::to_value(library.search(query)?)?)
}
