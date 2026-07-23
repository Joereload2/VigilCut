use std::time::Duration;

use crate::pipeline::visual::generation::omniroute::OmniRouteImageProvider;
use crate::pipeline::visual::generation::provider::{
    CostKind, GenerationRequest, GenerationResult, ProviderError, ProviderProbe,
};

const DEFAULT_BASE_URL: &str = "https://gen.pollinations.ai/v1";
const DEFAULT_MODEL: &str = "flux";

/// Experimental Pollinations route using the hardened OpenAI-compatible
/// transport instead of duplicating download and SSRF handling.
#[derive(Debug, Clone)]
pub struct PollinationsImageProvider {
    inner: OmniRouteImageProvider,
    catalog_url: String,
}

impl PollinationsImageProvider {
    pub fn from_env() -> Self {
        // Fixed official host: never forward a secret key to a configurable URL.
        let base_url = DEFAULT_BASE_URL.to_string();
        let catalog_url = format!(
            "{}/image/models",
            base_url
                .strip_suffix("/v1")
                .unwrap_or(&base_url)
                .trim_end_matches('/'),
        );
        Self {
            inner: OmniRouteImageProvider {
                base_url,
                api_key: std::env::var("POLLINATIONS_API_KEY")
                    .ok()
                    .filter(|key| !key.trim().is_empty()),
                model: std::env::var("POLLINATIONS_IMAGE_MODEL")
                    .unwrap_or_else(|_| DEFAULT_MODEL.into()),
                // Pollinations consumes metered Pollen. Promotional credits are
                // not a verified zero-cost entitlement.
                free_configured: false,
                timeout: Duration::from_secs(
                    std::env::var("POLLINATIONS_TIMEOUT_SECS")
                        .ok()
                        .and_then(|value| value.parse().ok())
                        .unwrap_or(90),
                ),
            },
            catalog_url,
        }
    }

    pub fn name(&self) -> &str {
        "pollinations"
    }

    pub fn is_free_tier(&self) -> bool {
        false
    }

    pub async fn generate(
        &self,
        request: &GenerationRequest,
    ) -> Result<GenerationResult, ProviderError> {
        let enabled = std::env::var("VIGILCUT_POLLINATIONS_EXPERIMENTAL")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !enabled {
            return Err(ProviderError::Unavailable(
                "Pollinations experimental route is disabled".into(),
            ));
        }
        if self.inner.api_key.is_none() {
            return Err(ProviderError::Unavailable(
                "POLLINATIONS_API_KEY is required".into(),
            ));
        }

        let mut result = self.inner.generate(request).await?;
        result.provider = self.name().into();
        result.cost_kind = CostKind::Paid;
        result.is_paid = true;
        result.free_verified = false;
        Ok(result)
    }

    /// Safe catalogue probe. It proves model discovery only, not generation,
    /// price, output license, or commercial-use rights.
    pub async fn probe(&self) -> Result<ProviderProbe, ProviderError> {
        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| ProviderError::Other(error.to_string()))?;
        let mut request = client.get(&self.catalog_url);
        if let Some(key) = &self.inner.api_key {
            request = request.bearer_auth(key);
        }
        match request.send().await {
            Ok(response) => {
                let ok = response.status().is_success();
                let status = response.status().as_u16();
                let body = if ok {
                    response.text().await.unwrap_or_default()
                } else {
                    String::new()
                };
                let model_visible = body.contains(&self.inner.model);
                Ok(ProviderProbe {
                    provider: self.name().into(),
                    model: self.inner.model.clone(),
                    supports_image: ok && model_visible,
                    free_tier: false,
                    free_verified: false,
                    cost_kind: CostKind::Paid,
                    ok,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: (!ok).then(|| format!("catalog status {status}")),
                    notes: Some(
                        "Catálogo solamente: generación, coste y licencia no verificados; daily feed bloqueado."
                            .into(),
                    ),
                })
            }
            Err(error) => Ok(ProviderProbe {
                provider: self.name().into(),
                model: self.inner.model.clone(),
                supports_image: false,
                free_tier: false,
                free_verified: false,
                cost_kind: CostKind::Unknown,
                ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(error.to_string()),
                notes: Some("Pollinations no disponible; no se intentó generación.".into()),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn generation_is_disabled_without_explicit_opt_in() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        std::env::remove_var("VIGILCUT_POLLINATIONS_EXPERIMENTAL");
        std::env::remove_var("POLLINATIONS_API_KEY");
        let provider = PollinationsImageProvider::from_env();
        assert!(!provider.is_free_tier());
        let result = provider
            .generate(&GenerationRequest {
                prompt: "test fixture".into(),
                negative_prompt: String::new(),
                model: None,
                width: 512,
                height: 512,
                seed: Some(1),
                job_id: "pollinations-disabled".into(),
            })
            .await;
        assert!(matches!(result, Err(ProviderError::Unavailable(_))));
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn paid_gate_blocks_before_network() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        std::env::set_var("VIGILCUT_POLLINATIONS_EXPERIMENTAL", "1");
        std::env::set_var("POLLINATIONS_API_KEY", "test-key-never-sent");
        std::env::remove_var("VIGILCUT_PAID_PROVIDERS");
        let result = PollinationsImageProvider::from_env()
            .generate(&GenerationRequest {
                prompt: "test fixture".into(),
                negative_prompt: String::new(),
                model: None,
                width: 512,
                height: 512,
                seed: Some(1),
                job_id: "pollinations-paid-gate".into(),
            })
            .await;
        assert!(matches!(result, Err(ProviderError::PaidDisabled)));
        std::env::remove_var("VIGILCUT_POLLINATIONS_EXPERIMENTAL");
        std::env::remove_var("POLLINATIONS_API_KEY");
    }
}
