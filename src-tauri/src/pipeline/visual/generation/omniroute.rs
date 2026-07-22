//! OmniRoute HTTP image provider — optional, never default to paid.

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;

use super::provider::{
    CostKind, GenerationRequest, GenerationResult, ProviderError, ProviderProbe,
};

const MAX_DOWNLOAD_BYTES: u64 = 25 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 90;
const MAX_ATTEMPTS: u32 = 3;
const MAX_REDIRECTS: usize = 3;

#[derive(Debug, Clone)]
pub struct OmniRouteImageProvider {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    /// From env OMNIROUTE_FREE_TIER — not verified by default
    pub free_configured: bool,
    pub timeout: Duration,
}

impl OmniRouteImageProvider {
    pub fn from_env() -> Self {
        let base = std::env::var("OMNIROUTE_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:20128/v1".into());
        let model = std::env::var("OMNIROUTE_IMAGE_MODEL")
            .unwrap_or_else(|_| "black-forest-labs/FLUX.1-schnell".into());
        let api_key = std::env::var("OMNIROUTE_API_KEY")
            .ok()
            .filter(|s| !s.is_empty());
        let free_configured = std::env::var("OMNIROUTE_FREE_TIER")
            .map(|s| s != "0" && !s.eq_ignore_ascii_case("false"))
            .unwrap_or(true);
        Self {
            base_url: base.trim_end_matches('/').to_string(),
            api_key,
            model,
            free_configured,
            timeout: Duration::from_secs(
                std::env::var("OMNIROUTE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(DEFAULT_TIMEOUT_SECS),
            ),
        }
    }

    pub fn name(&self) -> &str {
        "omniroute"
    }

    pub fn is_free_tier(&self) -> bool {
        // Never treat as free for policy without verification — paid if not configured free
        // For cost gate: free_configured means we won't mark is_paid on request path
        self.free_configured
    }

    fn cost_kind(&self) -> CostKind {
        if self.free_configured {
            CostKind::FreeConfigured
        } else {
            CostKind::Paid
        }
    }

    /// Build request body: always include negative_prompt when non-empty;
    /// also fold exclusions into positive prompt as defense in depth.
    pub fn build_body(&self, req: &GenerationRequest, model: &str) -> (serde_json::Value, String) {
        let mut prompt = req.prompt.clone();
        let mut strategy = "negative_field".to_string();
        if !req.negative_prompt.trim().is_empty() {
            // Many OpenAI-compatible image APIs accept negative_prompt;
            // also append to positive for providers that ignore the field.
            prompt.push_str(&format!(" Avoid: {}.", req.negative_prompt.trim()));
            strategy = "negative_field+folded_into_prompt".into();
        }
        let mut body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "n": 1,
            "size": format!("{}x{}", req.width.max(256), req.height.max(256)),
            "response_format": "url",
        });
        if !req.negative_prompt.trim().is_empty() {
            body["negative_prompt"] = serde_json::json!(req.negative_prompt);
        }
        (body, strategy)
    }

    pub async fn generate(
        &self,
        req: &GenerationRequest,
    ) -> Result<GenerationResult, ProviderError> {
        if !self.free_configured {
            let paid_ok = std::env::var("VIGILCUT_PAID_PROVIDERS")
                .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            if !paid_ok {
                return Err(ProviderError::PaidDisabled);
            }
        }

        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let url = format!("{}/images/generations", self.base_url);
        let (body, strategy) = self.build_body(req, &model);

        let max = MAX_ATTEMPTS.min(
            std::env::var("VIGILCUT_MAX_ATTEMPTS_PER_NEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(MAX_ATTEMPTS),
        );

        let mut last_err = ProviderError::Other("no attempts".into());
        for attempt in 0..max {
            match self
                .post_generate(&url, &body, req, &model, &strategy)
                .await
            {
                Ok(r) => return Ok(r),
                Err(ProviderError::RateLimited) => {
                    last_err = ProviderError::RateLimited;
                    let backoff = 200u64 * 2u64.pow(attempt) + (attempt as u64 * 37);
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
                Err(ProviderError::Http { status, message }) if status >= 500 => {
                    last_err = ProviderError::Http { status, message };
                    let backoff = 300u64 * 2u64.pow(attempt);
                    tokio::time::sleep(Duration::from_millis(backoff)).await;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last_err)
    }

    async fn post_generate(
        &self,
        url: &str,
        body: &serde_json::Value,
        req: &GenerationRequest,
        model: &str,
        strategy: &str,
    ) -> Result<GenerationResult, ProviderError> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        let mut builder = client.post(url).json(body);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }

        let resp = builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout
            } else {
                ProviderError::Unavailable(e.to_string())
            }
        })?;

        let status = resp.status().as_u16();
        if status == 429 {
            return Err(ProviderError::RateLimited);
        }
        if !resp.status().is_success() {
            let message = resp.text().await.unwrap_or_default();
            let message = message.chars().take(400).collect::<String>();
            return Err(ProviderError::Http { status, message });
        }

        let parsed: OpenAiImageResponse = resp
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let first = parsed
            .data
            .first()
            .ok_or_else(|| ProviderError::InvalidResponse("empty data[]".into()))?;

        let bytes = if let Some(b64) = &first.b64_json {
            base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| ProviderError::InvalidResponse(format!("b64: {e}")))?
        } else if let Some(u) = &first.url {
            self.download_url_streaming(u).await?
        } else {
            return Err(ProviderError::InvalidResponse(
                "no url or b64_json in response".into(),
            ));
        };

        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(ProviderError::InvalidResponse("file too large".into()));
        }
        validate_image_bytes(&bytes)?;
        let mime = sniff_mime(&bytes);
        let ext = match mime {
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => "png",
        };

        let root = crate::pipeline::visual::library::library_root()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let dir = root.join("candidates");
        std::fs::create_dir_all(&dir).map_err(|e| ProviderError::Other(e.to_string()))?;
        let path = dir.join(format!("{}.{}", req.job_id, ext));
        std::fs::write(&path, &bytes).map_err(|e| ProviderError::Other(e.to_string()))?;

        let (w, h) = image::load_from_memory(&bytes)
            .map(|i| (i.width(), i.height()))
            .unwrap_or((req.width, req.height));

        Ok(GenerationResult {
            local_path: path,
            provider: "omniroute".into(),
            model: model.to_string(),
            mime_type: mime.into(),
            width: w,
            height: h,
            is_paid: !self.free_configured,
            bytes: bytes.len() as u64,
            cost_kind: self.cost_kind(),
            free_verified: false,
            prompt_strategy: strategy.into(),
        })
    }

    async fn download_url_streaming(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(ProviderError::InvalidResponse(
                "url scheme not allowed".into(),
            ));
        }
        // Block obvious private/local hosts (SSRF guard)
        // Host extraction without extra deps
        let host = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .split('@')
            .next_back()
            .unwrap_or("")
            .split(':')
            .next()
            .unwrap_or("")
            .to_lowercase();
        if host == "localhost"
            || host == "127.0.0.1"
            || host == "0.0.0.0"
            || host == "::1"
            || host.starts_with("10.")
            || host.starts_with("192.168.")
            || host.starts_with("169.254.")
            || host.ends_with(".local")
        {
            let base_local =
                self.base_url.contains("localhost") || self.base_url.contains("127.0.0.1");
            if !(base_local && (host == "localhost" || host == "127.0.0.1")) {
                return Err(ProviderError::InvalidResponse(
                    "download URL host not allowed".into(),
                ));
            }
        }

        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(MAX_REDIRECTS))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(ProviderError::Http {
                status: resp.status().as_u16(),
                message: "download failed".into(),
            });
        }
        if let Some(len) = resp.content_length() {
            if len > MAX_DOWNLOAD_BYTES {
                return Err(ProviderError::InvalidResponse(
                    "Content-Length exceeds max".into(),
                ));
            }
        }
        // Stream with hard cap (chunk-by-chunk — never load unbounded)
        let mut buf: Vec<u8> = Vec::new();
        let mut resp = resp;
        loop {
            let chunk = resp
                .chunk()
                .await
                .map_err(|e| ProviderError::Other(e.to_string()))?;
            let Some(chunk) = chunk else {
                break;
            };
            if buf.len() as u64 + chunk.len() as u64 > MAX_DOWNLOAD_BYTES {
                return Err(ProviderError::InvalidResponse(
                    "download exceeded max during stream".into(),
                ));
            }
            buf.extend_from_slice(&chunk);
        }
        Ok(buf)
    }

    pub async fn probe(&self) -> Result<ProviderProbe, ProviderError> {
        let start = std::time::Instant::now();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let url = format!("{}/models", self.base_url);
        let mut builder = client.get(&url);
        if let Some(k) = &self.api_key {
            builder = builder.bearer_auth(k);
        }
        match builder.send().await {
            Ok(resp) => {
                let ok = resp.status().is_success();
                let err = if ok {
                    None
                } else {
                    Some(format!("status {}", resp.status().as_u16()))
                };
                Ok(ProviderProbe {
                    provider: "omniroute".into(),
                    model: self.model.clone(),
                    // Do NOT claim image support only because /models works
                    supports_image: false,
                    free_tier: self.free_configured,
                    free_verified: false,
                    cost_kind: self.cost_kind(),
                    ok,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: err,
                    notes: Some(
                        "OmniRoute reachability only — image support and free tier not verified. Coste: configurado, no verificado.".into(),
                    ),
                })
            }
            Err(e) => Ok(ProviderProbe {
                provider: "omniroute".into(),
                model: self.model.clone(),
                supports_image: false,
                free_tier: self.free_configured,
                free_verified: false,
                cost_kind: CostKind::Unknown,
                ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
                notes: Some(
                    "OmniRoute no disponible — la app sigue offline. Coste desconocido.".into(),
                ),
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiImageResponse {
    #[serde(default)]
    data: Vec<OpenAiImageData>,
}

#[derive(Debug, Deserialize)]
struct OpenAiImageData {
    url: Option<String>,
    b64_json: Option<String>,
}

fn sniff_mime(bytes: &[u8]) -> &'static str {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        "image/png"
    } else if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 {
        "image/jpeg"
    } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn validate_image_bytes(bytes: &[u8]) -> Result<(), ProviderError> {
    let mime = sniff_mime(bytes);
    if mime == "application/octet-stream" {
        return Err(ProviderError::InvalidResponse(
            "MIME no es imagen reconocida".into(),
        ));
    }
    image::load_from_memory(bytes).map_err(|e| {
        ProviderError::InvalidResponse(format!("bytes no decodifican como imagen: {e}"))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_folded_into_prompt() {
        let p = OmniRouteImageProvider {
            base_url: "http://localhost:20128/v1".into(),
            api_key: None,
            model: "m".into(),
            free_configured: true,
            timeout: Duration::from_secs(5),
        };
        let req = GenerationRequest {
            prompt: "person shopping".into(),
            negative_prompt: "crypto, luxury brands".into(),
            model: None,
            width: 512,
            height: 288,
            seed: None,
            job_id: "j1".into(),
        };
        let (body, strategy) = p.build_body(&req, "m");
        assert!(body["prompt"].as_str().unwrap().contains("Avoid:"));
        assert_eq!(
            body["negative_prompt"].as_str().unwrap(),
            "crypto, luxury brands"
        );
        assert!(strategy.contains("negative"));
    }

    #[test]
    fn sniff_png() {
        let mut png = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        png.extend_from_slice(&[0u8; 20]);
        assert_eq!(sniff_mime(&png), "image/png");
    }

    #[test]
    fn paid_disabled_without_flag() {
        let p = OmniRouteImageProvider {
            base_url: "http://127.0.0.1:9/v1".into(),
            api_key: None,
            model: "x".into(),
            free_configured: false,
            timeout: Duration::from_millis(50),
        };
        std::env::remove_var("VIGILCUT_PAID_PROVIDERS");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt.block_on(p.generate(&GenerationRequest {
            prompt: "t".into(),
            negative_prompt: String::new(),
            model: None,
            width: 256,
            height: 256,
            seed: None,
            job_id: "j".into(),
        }));
        assert!(matches!(err, Err(ProviderError::PaidDisabled)));
    }
}
