//! VigilCut vNext — durable content production (Phase 2).
//!
//! Layering:
//! - `domain` — pure types and invariants (no Tauri / SQLite / FFmpeg)
//! - `persistence` — SQLite `vnext.db`
//! - `application` — use-cases coordinating domain + persistence
//!
//! Biblioteca Visual (`library.db`) remains a separate database.

pub mod application;
pub mod domain;
pub mod persistence;
