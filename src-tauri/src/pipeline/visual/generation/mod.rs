//! Image generation service — replaceable providers, cost policy, worker.

pub mod cost;
pub mod mock;
pub mod omniroute;
pub mod provider;
pub mod worker;

pub use cost::{can_enqueue_generation, increment_generation_counter, CostGate};
pub use mock::MockImageProvider;
pub use omniroute::OmniRouteImageProvider;
pub use provider::{
    GenerationRequest, GenerationResult, ImageProvider, ProviderError, ProviderKind,
};
pub use worker::{process_next_job, queue_generation_for_need, worker_tick};
