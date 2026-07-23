//! Tauri command groups are kept as compatibility facades during extraction.
//! New commands must call `LibraryService` rather than B-roll pipeline modules.

pub mod library_commands;
