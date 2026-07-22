//! ImageGenerationProvider — enum dispatch (no async_trait).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Mock,
    OmniRoute,
    Local,
}

#[derive(Debug, Clone)]
pub struct GenerationRequest {
    pub prompt: String,
    pub negative_prompt: String,
    pub model: Option<String>,
    pub width: u32,
    pub height: u32,
    pub seed: Option<i64>,
    pub job_id: String,
}

#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub local_path: PathBuf,
    pub provider: String,
    pub model: String,
    pub mime_type: String,
    pub width: u32,
    pub height: u32,
    pub is_paid: bool,
    pub bytes: u64,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ProviderError {
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    #[error("rate limited")]
    RateLimited,
    #[error("timeout")]
    Timeout,
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("paid providers disabled by policy")]
    PaidDisabled,
    #[error("http {status}: {message}")]
    Http { status: u16, message: String },
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderProbe {
    pub provider: String,
    pub model: String,
    pub supports_image: bool,
    pub free_tier: bool,
    pub ok: bool,
    pub latency_ms: u64,
    pub error: Option<String>,
    pub notes: Option<String>,
}

/// Concrete provider selection (extensible without dyn async).
#[derive(Debug, Clone)]
pub enum ImageProvider {
    Mock(super::mock::MockImageProvider),
    OmniRoute(super::omniroute::OmniRouteImageProvider),
}

impl ImageProvider {
    pub fn kind(&self) -> ProviderKind {
        match self {
            Self::Mock(_) => ProviderKind::Mock,
            Self::OmniRoute(_) => ProviderKind::OmniRoute,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Mock(p) => p.name(),
            Self::OmniRoute(p) => p.name(),
        }
    }

    pub fn is_free_tier(&self) -> bool {
        match self {
            Self::Mock(p) => p.is_free_tier(),
            Self::OmniRoute(p) => p.is_free_tier(),
        }
    }

    pub async fn generate(&self, req: &GenerationRequest) -> Result<GenerationResult, ProviderError> {
        match self {
            Self::Mock(p) => p.generate(req).await,
            Self::OmniRoute(p) => p.generate(req).await,
        }
    }

    pub async fn probe(&self) -> Result<ProviderProbe, ProviderError> {
        match self {
            Self::Mock(p) => p.probe().await,
            Self::OmniRoute(p) => p.probe().await,
        }
    }
}

/// Select provider: mock if forced or OmniRoute not configured.
pub fn select_provider(_allow_paid: bool) -> ImageProvider {
    let force_mock = std::env::var("VIGILCUT_IMAGE_PROVIDER")
        .map(|s| s.eq_ignore_ascii_case("mock"))
        .unwrap_or(false);
    let base = std::env::var("OMNIROUTE_BASE_URL").ok();
    let has_omni = base.as_ref().map(|b| !b.is_empty()).unwrap_or(false);
    if force_mock || !has_omni {
        return ImageProvider::Mock(super::mock::MockImageProvider::default());
    }
    ImageProvider::OmniRoute(super::omniroute::OmniRouteImageProvider::from_env())
}
