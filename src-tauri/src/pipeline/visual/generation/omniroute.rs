//! OmniRoute HTTP image provider — optional, never default to paid.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
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
            .redirect(reqwest::redirect::Policy::none())
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
        let mut current = validate_download_url(url, &self.base_url)?;
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            // Manual redirects so every hop is re-validated (Codex CRIT-004)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| ProviderError::Other(e.to_string()))?;

        for hop in 0..=MAX_REDIRECTS {
            let resp = client
                .get(current.as_str())
                .send()
                .await
                .map_err(|e| ProviderError::Unavailable(e.to_string()))?;
            let status = resp.status();
            if status.is_redirection() {
                if hop == MAX_REDIRECTS {
                    return Err(ProviderError::InvalidResponse("too many redirects".into()));
                }
                let loc = resp
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| {
                        ProviderError::InvalidResponse("redirect without Location".into())
                    })?;
                let next = if loc.starts_with("http://") || loc.starts_with("https://") {
                    loc.to_string()
                } else {
                    current
                        .join(loc)
                        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?
                        .to_string()
                };
                current = validate_download_url(&next, &self.base_url)?;
                continue;
            }
            if !status.is_success() {
                return Err(ProviderError::Http {
                    status: status.as_u16(),
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
            return Ok(buf);
        }
        Err(ProviderError::InvalidResponse("redirect loop".into()))
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

/// Parse URL, allow only http(s), resolve DNS, reject non-global IPs (SSRF).
fn validate_download_url(url: &str, base_url: &str) -> Result<reqwest::Url, ProviderError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| ProviderError::InvalidResponse(format!("bad url: {e}")))?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(ProviderError::InvalidResponse(
                "url scheme not allowed".into(),
            ))
        }
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| ProviderError::InvalidResponse("url missing host".into()))?
        .to_lowercase();
    if (host.ends_with(".local") || host.ends_with(".internal") || host.ends_with(".localhost"))
        && !allow_loopback_for_base(base_url, &host)
    {
        return Err(ProviderError::InvalidResponse(
            "download URL host not allowed".into(),
        ));
    }

    let port = parsed
        .port_or_known_default()
        .unwrap_or(if parsed.scheme() == "https" { 443 } else { 80 });

    // Literal IP in host
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_forbidden_ip(ip) && !allow_loopback_for_base(base_url, &host) {
            return Err(ProviderError::InvalidResponse(
                "download URL resolves to blocked IP".into(),
            ));
        }
        return Ok(parsed);
    }

    // DNS resolve all A/AAAA — reject if any address is non-global
    let addrs: Vec<SocketAddr> = match (host.as_str(), port).to_socket_addrs() {
        Ok(iter) => iter.collect(),
        Err(e) => {
            return Err(ProviderError::InvalidResponse(format!(
                "DNS resolve failed: {e}"
            )))
        }
    };
    if addrs.is_empty() {
        return Err(ProviderError::InvalidResponse(
            "DNS returned no addresses".into(),
        ));
    }
    for addr in &addrs {
        if is_forbidden_ip(addr.ip()) {
            if allow_loopback_for_base(base_url, &host) && addr.ip().is_loopback() {
                continue;
            }
            return Err(ProviderError::InvalidResponse(format!(
                "download URL resolves to blocked IP {}",
                addr.ip()
            )));
        }
    }
    Ok(parsed)
}

fn allow_loopback_for_base(base_url: &str, host: &str) -> bool {
    let base_local = base_url.contains("localhost")
        || base_url.contains("127.0.0.1")
        || base_url.contains("[::1]");
    base_local
        && (host == "localhost"
            || host == "127.0.0.1"
            || host == "::1"
            || host == "[::1]"
            || host.ends_with(".localhost"))
}

fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_forbidden_v4(v4),
        IpAddr::V6(v6) => is_forbidden_v6(v6),
    }
}

fn is_forbidden_v4(v4: Ipv4Addr) -> bool {
    if v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || v4.is_broadcast()
        || v4.is_documentation()
        || v4.is_unspecified()
        || v4.is_multicast()
    {
        return true;
    }
    let o = v4.octets();
    // CGNAT 100.64/10
    if o[0] == 100 && (o[1] & 0xc0) == 64 {
        return true;
    }
    // 0/8
    if o[0] == 0 {
        return true;
    }
    // Reserved / benchmark 198.18/15
    if o[0] == 198 && (o[1] == 18 || o[1] == 19) {
        return true;
    }
    false
}

fn is_forbidden_v6(v6: Ipv6Addr) -> bool {
    if v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() {
        return true;
    }
    let s = v6.segments();
    // Unique local fc00::/7 (MSRV-safe; avoid is_unique_local 1.84+)
    if (s[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // Link-local fe80::/10
    if (s[0] & 0xffc0) == 0xfe80 {
        return true;
    }
    // IPv4-mapped
    if let Some(v4) = v6.to_ipv4_mapped() {
        return is_forbidden_v4(v4);
    }
    // Documentation 2001:db8::/32
    if s[0] == 0x2001 && s[1] == 0xdb8 {
        return true;
    }
    false
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

    #[test]
    fn ssrf_blocks_private_literals() {
        let base = "https://api.example.com/v1";
        for u in [
            "http://127.0.0.1/secret",
            "http://10.0.0.5/x",
            "http://192.168.1.1/x",
            "http://172.16.0.1/x",
            "http://169.254.169.254/latest/meta-data/",
            "http://100.64.0.1/x",
            "http://[::1]/x",
            "ftp://example.com/x",
        ] {
            assert!(
                validate_download_url(u, base).is_err(),
                "expected block for {u}"
            );
        }
    }

    #[test]
    fn ssrf_allows_public_https() {
        // example.com is public; DNS may fail offline — only check parse/scheme path for IP form
        let r = validate_download_url("https://93.184.216.34/", "https://api.example.com/v1");
        // 93.184.216.34 is public (example.com historically); may be reassigned but not private
        match r {
            Ok(_) => {}
            Err(e) => {
                // if network policy blocks, still not a private-IP error path failure for literals
                let s = e.to_string();
                assert!(!s.contains("blocked IP") || s.contains("DNS"), "{s}");
            }
        }
    }

    #[test]
    fn ssrf_loopback_ok_when_base_local() {
        assert!(validate_download_url(
            "http://127.0.0.1:20128/img.png",
            "http://127.0.0.1:20128/v1"
        )
        .is_ok());
        assert!(validate_download_url(
            "http://localhost:20128/img.png",
            "http://localhost:20128/v1"
        )
        .is_ok());
        assert!(
            validate_download_url("http://127.0.0.1/img.png", "https://api.example.com/v1")
                .is_err()
        );
    }

    #[test]
    fn forbidden_ip_table() {
        assert!(is_forbidden_v4(Ipv4Addr::new(127, 1, 2, 3)));
        assert!(is_forbidden_v4(Ipv4Addr::new(10, 1, 2, 3)));
        assert!(is_forbidden_v4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_forbidden_v4(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_forbidden_v4(Ipv4Addr::new(169, 254, 1, 1)));
        assert!(is_forbidden_v4(Ipv4Addr::new(100, 64, 0, 1)));
        assert!(!is_forbidden_v4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(is_forbidden_v6(Ipv6Addr::LOCALHOST));
        assert!(is_forbidden_v6("fc00::1".parse::<Ipv6Addr>().unwrap()));
        assert!(is_forbidden_ip(
            "::ffff:127.0.0.1".parse::<IpAddr>().unwrap()
        ));
    }
}
