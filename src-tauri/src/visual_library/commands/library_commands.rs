use crate::error::AppResult;
use crate::visual_library::{AssetQuery, LibraryService, VisualLibrary};

pub fn search(query: &AssetQuery) -> AppResult<serde_json::Value> {
    Ok(serde_json::to_value(LibraryService::new().search(query)?)?)
}
