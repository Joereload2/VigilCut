//! Storage implementations remain local-first. Remote storage is introduced
//! only behind the optional sync phase.

pub use crate::pipeline::visual::library::library_root;

pub mod supabase_storage;
