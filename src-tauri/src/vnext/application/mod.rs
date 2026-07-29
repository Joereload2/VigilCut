//! Application use-cases for vNext.

mod clipping;
mod jobs;
mod render;

pub use clipping::*;
pub use jobs::*;
pub use render::*;

#[cfg(test)]
#[path = "jobs_tests.rs"]
mod jobs_tests;

#[cfg(test)]
#[path = "clipping_tests.rs"]
mod clipping_tests;

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;
