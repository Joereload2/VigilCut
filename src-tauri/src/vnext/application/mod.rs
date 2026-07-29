//! Application use-cases for vNext Phase 2.

mod jobs;

pub use jobs::*;

#[cfg(test)]
#[path = "jobs_tests.rs"]
mod jobs_tests;
