//! Application use-cases for vNext.

mod clipping;
mod jobs;

pub use clipping::*;
pub use jobs::*;

#[cfg(test)]
#[path = "jobs_tests.rs"]
mod jobs_tests;

#[cfg(test)]
#[path = "clipping_tests.rs"]
mod clipping_tests;
