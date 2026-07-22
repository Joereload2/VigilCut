//! Image generation service — replaceable providers, cost policy, worker.

pub mod cost;
pub mod daily_feed;
pub mod mock;
pub mod omniroute;
pub mod provider;
pub mod supervision;
pub mod worker;

#[cfg(test)]
#[path = "supervision_tests.rs"]
mod supervision_tests;

pub use cost::{can_enqueue_generation, increment_generation_counter, CostGate};
pub use mock::MockImageProvider;
pub use omniroute::OmniRouteImageProvider;
pub use provider::{
    CostKind, GenerationRequest, GenerationResult, ImageProvider, ProviderError, ProviderKind,
};
pub use worker::{process_next_job, queue_generation_for_need, worker_tick};
