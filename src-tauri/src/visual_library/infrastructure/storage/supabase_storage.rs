use std::path::Path;
use std::time::Duration;

use crate::error::{AppError, AppResult};
use crate::models::visual::MediaAsset;

const BUCKET: &str = "visual-library";

#[derive(Debug, Clone)]
pub struct SupabaseConfig {
    pub enabled: bool,
    pub url: String,
    pub publishable_key: String,
    pub access_token: String,
    pub workspace_id: String,
}

impl SupabaseConfig {
    pub fn from_env() -> AppResult<Option<Self>> {
        let enabled = std::env::var("VIGILCUT_SUPABASE_SYNC")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !enabled {
            return Ok(None);
        }
        let rls_verified = std::env::var("VIGILCUT_SUPABASE_RLS_VERIFIED")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !rls_verified {
            return Err(AppError::Invalid(
                "Supabase bloqueado hasta verificar RLS y Storage en desarrollo".into(),
            ));
        }
        let url = required_env("SUPABASE_URL")?;
        let parsed = reqwest::Url::parse(&url)
            .map_err(|error| AppError::Invalid(format!("SUPABASE_URL: {error}")))?;
        let local = matches!(parsed.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        let hosted = parsed.scheme() == "https"
            && parsed
                .host_str()
                .is_some_and(|host| host.ends_with(".supabase.co"));
        if !local && !hosted {
            return Err(AppError::Invalid(
                "SUPABASE_URL debe ser https://*.supabase.co o un proyecto local".into(),
            ));
        }
        let publishable_key = required_env("SUPABASE_PUBLISHABLE_KEY")?;
        if publishable_key.starts_with("sb_secret_") {
            return Err(AppError::Invalid(
                "No se permite una clave secret/service_role en VigilCut".into(),
            ));
        }
        Ok(Some(Self {
            enabled,
            url: url.trim_end_matches('/').into(),
            publishable_key,
            access_token: required_env("SUPABASE_ACCESS_TOKEN")?,
            workspace_id: required_env("SUPABASE_WORKSPACE_ID")?,
        }))
    }
}

fn required_env(name: &str) -> AppResult<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Invalid(format!("Falta {name}")))
}

#[derive(Debug, Clone)]
pub struct SupabaseStorage {
    config: SupabaseConfig,
    client: reqwest::Client,
}

impl SupabaseStorage {
    pub fn new(config: SupabaseConfig) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|error| AppError::Message(error.to_string()))?;
        Ok(Self { config, client })
    }

    fn authenticated(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
            .header("apikey", &self.config.publishable_key)
            .bearer_auth(&self.config.access_token)
    }

    pub async fn health_check(&self) -> AppResult<u64> {
        let started = std::time::Instant::now();
        let response = self
            .authenticated(self.client.get(format!("{}/rest/v1/", self.config.url)))
            .send()
            .await
            .map_err(|error| AppError::Message(error.to_string()))?;
        if !response.status().is_success() {
            return Err(AppError::Message(format!(
                "Supabase health HTTP {}",
                response.status()
            )));
        }
        Ok(started.elapsed().as_millis() as u64)
    }

    pub async fn push_asset(&self, asset: &MediaAsset) -> AppResult<()> {
        let source = Path::new(&asset.managed_path);
        let bytes = std::fs::read(source).map_err(AppError::Io)?;
        let extension = source
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("bin");
        let storage_path = format!(
            "{}/assets/{}/original.{}",
            self.config.workspace_id, asset.id, extension
        );
        let upload_url = format!(
            "{}/storage/v1/object/{}/{}",
            self.config.url, BUCKET, storage_path
        );
        let upload = self
            .authenticated(self.client.post(upload_url))
            .header("Content-Type", &asset.mime_type)
            .header("x-upsert", "true")
            .body(bytes)
            .send()
            .await
            .map_err(|error| AppError::Message(error.to_string()))?;
        if !upload.status().is_success() {
            return Err(AppError::Message(format!(
                "Supabase Storage HTTP {}",
                upload.status()
            )));
        }

        let payload = serde_json::json!({
            "id": asset.id,
            "workspace_id": self.config.workspace_id,
            "storage_path": storage_path,
            "sha256": asset.sha256,
            "perceptual_hash": asset.perceptual_hash,
            "title": asset.title,
            "description": asset.description,
            "tags": asset.tags,
            "width": asset.width,
            "height": asset.height,
            "orientation": asset.orientation,
            "mime_type": asset.mime_type,
            "file_size": asset.file_size,
            "license_status": serde_json::to_value(asset.license_status)
                .unwrap_or(serde_json::Value::String("unknown".into())),
            "commercial_use": asset.commercial_use,
            "provenance": asset.provenance,
            "technical_score": asset.technical_score,
            "semantic_score": asset.semantic_score,
            "qa_status": serde_json::to_value(asset.qa_status)
                .unwrap_or(serde_json::Value::String("none".into())),
            "status": serde_json::to_value(asset.status)
                .unwrap_or(serde_json::Value::String("active".into())),
            "updated_at": asset.updated_at,
        });
        let metadata = self
            .authenticated(
                self.client
                    .post(format!("{}/rest/v1/media_assets", self.config.url)),
            )
            .header("Content-Type", "application/json")
            .header("Prefer", "resolution=merge-duplicates,return=minimal")
            .json(&payload)
            .send()
            .await
            .map_err(|error| AppError::Message(error.to_string()))?;
        if !metadata.status().is_success() {
            return Err(AppError::Message(format!(
                "Supabase metadata HTTP {}",
                metadata.status()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_by_default_and_secret_keys_are_rejected() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        std::env::remove_var("VIGILCUT_SUPABASE_SYNC");
        assert!(SupabaseConfig::from_env().unwrap().is_none());

        std::env::set_var("VIGILCUT_SUPABASE_SYNC", "1");
        std::env::set_var("VIGILCUT_SUPABASE_RLS_VERIFIED", "1");
        std::env::set_var("SUPABASE_URL", "https://example.supabase.co");
        std::env::set_var("SUPABASE_PUBLISHABLE_KEY", "sb_secret_forbidden");
        std::env::set_var("SUPABASE_ACCESS_TOKEN", "test");
        std::env::set_var("SUPABASE_WORKSPACE_ID", uuid::Uuid::new_v4().to_string());
        assert!(SupabaseConfig::from_env().is_err());
        for name in [
            "VIGILCUT_SUPABASE_SYNC",
            "VIGILCUT_SUPABASE_RLS_VERIFIED",
            "SUPABASE_URL",
            "SUPABASE_PUBLISHABLE_KEY",
            "SUPABASE_ACCESS_TOKEN",
            "SUPABASE_WORKSPACE_ID",
        ] {
            std::env::remove_var(name);
        }
    }
}
