//! OmniRoute HTTP image provider — optional, never default to paid.

use std::time::Duration;

use base64::Engine;
use serde::Deserialize;

use super::provider::{GenerationRequest, GenerationResult, ProviderError, ProviderProbe};

const MAX_DOWNLOAD_BYTES: u64 = 25 * 1024 * 1024;
const DEFAULT_TIMEOUT_SECS: u64 = 90;
const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Clone)]
pub struct OmniRouteImageProvider {
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub free_tier: bool,
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
        let free_tier = std::env::var("OMNIROUTE_FREE_TIER")
            .map(|s| s != "0" && !s.eq_ignore_ascii_case("false"))
            .unwrap_or(true);
        Self {
            base_url: base.trim_end_matches('/').to_string(),
            api_key,
            model,
            free_tier,
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
        self.free_tier
    }

    pub async fn generate(
        &self,
        req: &GenerationRequest,
    ) -> Result<GenerationResult, ProviderError> {
        if !self.free_tier {
            let paid_ok = std::env::var("VIGILCUT_PAID_PROVIDERS")
                .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            if !paid_ok {
                return Err(ProviderError::PaidDisabled);
            }
        }

        let model = req.model.clone().unwrap_or_else(|| self.model.clone());
        let url = format!("{}/images/generations", self.base_url);
        let body = serde_json::json!({
            "model": model,
            "prompt": req.prompt,
            "n": 1,
            "size": format!("{}x{}", req.width.max(256), req.height.max(256)),
            "response_format": "url",
        });

        let mut last_err = ProviderError::Other("no attempts".into());
        for attempt in 0..MAX_ATTEMPTS {
            match self.post_generate(&url, &body, req, &model).await {
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
    ) -> Result<GenerationResult, ProviderError> {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
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
            // Never log secrets — truncate body
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
            self.download_url(u).await?
        } else {
            return Err(ProviderError::InvalidResponse(
                "no url or b64_json in response".into(),
            ));
        };

        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(ProviderError::InvalidResponse("file too large".into()));
        }
        validate_image_bytes(&bytes)?;

        let root = crate::pipeline::visual::library::library_root()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let dir = root.join("candidates");
        std::fs::create_dir_all(&dir).map_err(|e| ProviderError::Other(e.to_string()))?;
        let path = dir.join(format!("{}.png", req.job_id));
        std::fs::write(&path, &bytes).map_err(|e| ProviderError::Other(e.to_string()))?;

        let (w, h) = image::load_from_memory(&bytes)
            .map(|i| (i.width(), i.height()))
            .unwrap_or((req.width, req.height));

        Ok(GenerationResult {
            local_path: path,
            provider: "omniroute".into(),
            model: model.to_string(),
            mime_type: sniff_mime(&bytes).into(),
            width: w,
            height: h,
            is_paid: !self.free_tier,
            bytes: bytes.len() as u64,
        })
    }

    async fn download_url(&self, url: &str) -> Result<Vec<u8>, ProviderError> {
        // Only http(s)
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            return Err(ProviderError::InvalidResponse(
                "url scheme not allowed".into(),
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
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
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
            return Err(ProviderError::InvalidResponse("download too large".into()));
        }
        Ok(bytes.to_vec())
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
                    supports_image: true,
                    free_tier: self.free_tier,
                    ok,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error: err,
                    notes: Some(
                        "probe lists /v1/models only; image model must be tested explicitly".into(),
                    ),
                })
            }
            Err(e) => Ok(ProviderProbe {
                provider: "omniroute".into(),
                model: self.model.clone(),
                supports_image: true,
                free_tier: self.free_tier,
                ok: false,
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
                notes: Some("OmniRoute no disponible — la app sigue offline".into()),
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
    fn sniff_png() {
        let mut png = vec![0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
        png.extend_from_slice(&[0u8; 20]);
        assert_eq!(sniff_mime(&png), "image/png");
    }

    #[test]
    fn paid_disabled_without_flag() {
        // Unit path: construct paid provider
        let p = OmniRouteImageProvider {
            base_url: "http://127.0.0.1:9/v1".into(),
            api_key: None,
            model: "x".into(),
            free_tier: false,
            timeout: Duration::from_millis(50),
        };
        // env paid off
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
