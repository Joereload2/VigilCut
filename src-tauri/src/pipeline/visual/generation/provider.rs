//! ImageGenerationProvider — enum dispatch (no async_trait).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// CostKind needs Serialize/Deserialize for API responses.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Mock,
    OmniRoute,
    Local,
    Pollinations,
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

/// How we present cost to the user — never claim "free" without distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CostKind {
    #[default]
    Unknown,
    /// Free tier verified by probe / explicit free provider response
    FreeVerified,
    /// Config says free (OMNIROUTE_FREE_TIER) but not verified
    FreeConfigured,
    Local,
    Paid,
}

impl CostKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::FreeVerified => "free_verified",
            Self::FreeConfigured => "free_configured",
            Self::Local => "local",
            Self::Paid => "paid",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "free_verified" => Self::FreeVerified,
            "free_configured" => Self::FreeConfigured,
            "local" => Self::Local,
            "paid" => Self::Paid,
            _ => Self::Unknown,
        }
    }

    pub fn label_es(self) -> &'static str {
        match self {
            Self::FreeVerified => "Gratis verificado",
            Self::FreeConfigured => "Gratis configurado, no verificado",
            Self::Local => "Generación local",
            Self::Paid => "Pagado",
            Self::Unknown => "Coste desconocido",
        }
    }
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
    pub cost_kind: CostKind,
    pub free_verified: bool,
    /// How negative prompt was applied
    pub prompt_strategy: String,
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
    /// True only after an image-capable endpoint is confirmed — never from /models alone.
    pub supports_image: bool,
    pub free_tier: bool,
    /// Free claim verified by probe or known free mock
    pub free_verified: bool,
    pub cost_kind: CostKind,
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
    Pollinations(
        crate::visual_library::infrastructure::providers::pollinations::PollinationsImageProvider,
    ),
}

impl ImageProvider {
    pub fn kind(&self) -> ProviderKind {
        match self {
            Self::Mock(_) => ProviderKind::Mock,
            Self::OmniRoute(_) => ProviderKind::OmniRoute,
            Self::Pollinations(_) => ProviderKind::Pollinations,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Mock(p) => p.name(),
            Self::OmniRoute(p) => p.name(),
            Self::Pollinations(p) => p.name(),
        }
    }

    pub fn is_free_tier(&self) -> bool {
        match self {
            Self::Mock(p) => p.is_free_tier(),
            Self::OmniRoute(p) => p.is_free_tier(),
            Self::Pollinations(p) => p.is_free_tier(),
        }
    }

    pub async fn generate(
        &self,
        req: &GenerationRequest,
    ) -> Result<GenerationResult, ProviderError> {
        match self {
            Self::Mock(p) => p.generate(req).await,
            Self::OmniRoute(p) => p.generate(req).await,
            Self::Pollinations(p) => p.generate(req).await,
        }
    }

    pub async fn probe(&self) -> Result<ProviderProbe, ProviderError> {
        match self {
            Self::Mock(p) => p.probe().await,
            Self::OmniRoute(p) => p.probe().await,
            Self::Pollinations(p) => p.probe().await,
        }
    }
}

/// Select provider: mock if forced or OmniRoute not configured.
pub fn select_provider(_allow_paid: bool) -> ImageProvider {
    let selected = std::env::var("VIGILCUT_IMAGE_PROVIDER").unwrap_or_default();
    if selected.eq_ignore_ascii_case("pollinations") {
        return ImageProvider::Pollinations(
            crate::visual_library::infrastructure::providers::pollinations::PollinationsImageProvider::from_env(),
        );
    }
    let force_mock = selected.eq_ignore_ascii_case("mock");
    let base = std::env::var("OMNIROUTE_BASE_URL").ok();
    let has_omni = base.as_ref().map(|b| !b.is_empty()).unwrap_or(false);
    if force_mock || !has_omni {
        return ImageProvider::Mock(super::mock::MockImageProvider);
    }
    ImageProvider::OmniRoute(super::omniroute::OmniRouteImageProvider::from_env())
}
