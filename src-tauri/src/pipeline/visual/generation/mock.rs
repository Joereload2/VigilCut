//! Offline mock image generator — solid placeholder PNG for tests and no-network mode.

use super::provider::{GenerationRequest, GenerationResult, ProviderError, ProviderProbe};

#[derive(Debug, Clone, Default)]
pub struct MockImageProvider;

impl MockImageProvider {
    pub fn name(&self) -> &str {
        "mock"
    }

    pub fn is_free_tier(&self) -> bool {
        true
    }

    pub async fn generate(
        &self,
        req: &GenerationRequest,
    ) -> Result<GenerationResult, ProviderError> {
        let w = req.width.clamp(64, 2048);
        let h = req.height.clamp(64, 2048);
        // Deterministic color from prompt hash
        let mut hash: u32 = 2166136261;
        for b in req.prompt.bytes() {
            hash ^= b as u32;
            hash = hash.wrapping_mul(16777619);
        }
        let r = ((hash >> 16) & 0xff) as u8;
        let g = ((hash >> 8) & 0xff) as u8;
        let b = (hash & 0xff) as u8;

        let mut img = image::RgbImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                // soft gradient so QA does not treat as empty solid-black
                let fx = x as f32 / w as f32;
                let fy = y as f32 / h as f32;
                let pr = (r as f32 * (0.4 + 0.6 * fx)).min(255.0) as u8;
                let pg = (g as f32 * (0.4 + 0.6 * fy)).min(255.0) as u8;
                let pb = (b as f32 * (0.5 + 0.5 * (1.0 - fx))).min(255.0) as u8;
                img.put_pixel(x, y, image::Rgb([pr, pg, pb]));
            }
        }

        let root = crate::pipeline::visual::library::library_root()
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let dir = root.join("candidates");
        std::fs::create_dir_all(&dir).map_err(|e| ProviderError::Other(e.to_string()))?;
        let path = dir.join(format!("{}.png", req.job_id));
        image::DynamicImage::ImageRgb8(img)
            .save(&path)
            .map_err(|e| ProviderError::Other(e.to_string()))?;
        let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        Ok(GenerationResult {
            local_path: path,
            provider: "mock".into(),
            model: "mock-gradient-v1".into(),
            mime_type: "image/png".into(),
            width: w,
            height: h,
            is_paid: false,
            bytes,
            cost_kind: super::provider::CostKind::Local,
            free_verified: true,
            prompt_strategy: "mock_ignores_negative".into(),
        })
    }

    pub async fn probe(&self) -> Result<ProviderProbe, ProviderError> {
        Ok(ProviderProbe {
            provider: "mock".into(),
            model: "mock-gradient-v1".into(),
            supports_image: true,
            free_tier: true,
            free_verified: true,
            cost_kind: super::provider::CostKind::Local,
            ok: true,
            latency_ms: 1,
            error: None,
            notes: Some("offline synthetic PNG — generation local, free verified".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::library::set_library_root_override;

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn mock_writes_png() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-mock-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let p = MockImageProvider;
        let r = p
            .generate(&GenerationRequest {
                prompt: "persona supermercado precios".into(),
                negative_prompt: "lujo".into(),
                model: None,
                width: 320,
                height: 180,
                seed: None,
                job_id: "job-test-1".into(),
            })
            .await
            .unwrap();
        assert!(r.local_path.exists());
        assert_eq!(r.width, 320);
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
