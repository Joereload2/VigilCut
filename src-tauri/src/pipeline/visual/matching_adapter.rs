//! B-roll adapter for the independent Visual Library search contract.

use crate::error::AppResult;
use crate::models::visual_intel::{MatchCandidate, VisualNeed};
use crate::visual_library::{LibraryService, VisualLibrary};

pub fn search_for_need(need: &VisualNeed) -> AppResult<Vec<MatchCandidate>> {
    let service = LibraryService::new();
    service.search_for_need(need)
}

pub fn get_asset(asset_id: &str) -> AppResult<crate::models::visual::MediaAsset> {
    LibraryService::new().get_asset(asset_id)
}
