//! Independent, reusable Visual Library domain.
//!
//! `pipeline::visual` is a consumer (B-roll composition). New consumers such as
//! Story Builder depend on the contracts and service exposed here, never on
//! SQLite tables, managed paths, or provider details.

pub mod application;
pub mod commands;
pub mod domain;
pub mod infrastructure;

pub use application::library_service::{LibraryService, LocalVisualLibrary, VisualLibrary};
pub use domain::contracts::*;
