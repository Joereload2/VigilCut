//! Storage implementations remain local-first. Remote storage is introduced
//! only behind the optional sync phase.

pub use crate::visual_library::infrastructure::legacy_adapter::library_root;

pub mod supabase_storage;
