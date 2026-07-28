//! Domain usage row. Fields match the legacy SQLite mapping; the type is
//! owned by `visual_library` so call sites do not import `pipeline::visual::library`.

pub use crate::visual_library::infrastructure::legacy_adapter::AssetUsageRow;
