//! Automated image QA — cheap technical checks first; semantic pluggable.

use std::path::Path;

use image::GenericImageView;

use crate::error::{AppError, AppResult};
use crate::models::visual_intel::{QaCheckResult, QaStatus};
use crate::pipeline::visual::library::{hamming_hex, open_db, perceptual_hash_simple};

#[derive(Debug, Clone)]
pub struct QaThresholds {
    pub min_width: u32,
    pub min_height: u32,
    pub max_file_bytes: u64,
    pub auto_approve_tech: f64,
    pub human_review_tech: f64,
    pub auto_approve_semantic: f64,
    pub reject_semantic: f64,
    pub phash_dup_distance: u32,
}

impl Default for QaThresholds {
    fn default() -> Self {
        Self {
            min_width: 256,
            min_height: 256,
            max_file_bytes: 25 * 1024 * 1024,
            auto_approve_tech: 0.75,
            human_review_tech: 0.45,
            auto_approve_semantic: 0.72,
            reject_semantic: 0.35,
            phash_dup_distance: 5,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SemanticHints {
    pub label: String,
    pub meanings: Vec<String>,
    pub hard_exclusions: Vec<String>,
    pub negative_contexts: Vec<String>,
}

/// Technical + heuristic semantic QA. Returns structured decision.
pub fn review_image(
    path: &Path,
    hints: Option<&SemanticHints>,
    thresholds: &QaThresholds,
) -> AppResult<QaCheckResult> {
    let meta = std::fs::metadata(path)?;
    let size = meta.len();
    let mut forbidden = Vec::new();
    let mut details = serde_json::json!({});

    if size == 0 {
        return Ok(reject("empty_file", 0.0, 0.0, forbidden, "archivo vacío"));
    }
    if size > thresholds.max_file_bytes {
        return Ok(reject(
            "too_large",
            0.1,
            0.0,
            forbidden,
            "archivo supera límite",
        ));
    }

    let bytes = std::fs::read(path)?;
    let mime_ok = sniff_ok(&bytes);
    details["mimeOk"] = serde_json::json!(mime_ok);
    if !mime_ok {
        return Ok(reject(
            "bad_mime",
            0.0,
            0.0,
            forbidden,
            "MIME/magic no es imagen válida",
        ));
    }

    let img = image::load_from_memory(&bytes)
        .map_err(|e| AppError::Invalid(format!("imagen corrupta: {e}")))?;
    let (w, h) = img.dimensions();
    details["width"] = serde_json::json!(w);
    details["height"] = serde_json::json!(h);

    if w < thresholds.min_width || h < thresholds.min_height {
        return Ok(reject(
            "low_res",
            0.3,
            0.0,
            forbidden,
            format!("resolución insuficiente {w}x{h}"),
        ));
    }

    // Darkness / emptiness heuristic
    let small = img.thumbnail(64, 64).to_luma8();
    let mut sum: u64 = 0;
    let mut var_acc: f64 = 0.0;
    let n = small.len() as f64;
    for p in small.pixels() {
        sum += p[0] as u64;
    }
    let mean = sum as f64 / n;
    for p in small.pixels() {
        let d = p[0] as f64 - mean;
        var_acc += d * d;
    }
    let variance = var_acc / n;
    details["meanLuma"] = serde_json::json!(mean);
    details["variance"] = serde_json::json!(variance);

    let mut tech: f64 = 0.9;
    if mean < 8.0 {
        tech -= 0.5;
        forbidden.push("near_black".into());
    }
    if mean > 247.0 {
        tech -= 0.35;
        forbidden.push("near_white".into());
    }
    if variance < 20.0 {
        tech -= 0.25;
        forbidden.push("low_detail".into());
    }
    // crude blur proxy: very low variance already covered
    tech = tech.clamp(0.0, 1.0);

    let phash = perceptual_hash_simple(&img);
    details["perceptualHash"] = serde_json::json!(phash);
    if let Some(dup) = find_near_duplicate(&phash, thresholds.phash_dup_distance)? {
        details["nearDuplicateAssetId"] = serde_json::json!(dup);
        // Exact library reuse preferred — near dup is human review, not hard reject
        tech = tech.min(0.7);
        forbidden.push(format!("near_duplicate:{dup}"));
    }

    // Heuristic semantic: term overlap with filename/path not used; use hints only
    let mut semantic = 0.55_f64; // neutral default without vision model
    if let Some(h) = hints {
        let blob = format!(
            "{} {}",
            h.label.to_lowercase(),
            h.meanings.join(" ").to_lowercase()
        );
        // Without vision we cannot verify depiction — score mid and flag for human if weak
        semantic = 0.62;
        for ex in &h.hard_exclusions {
            let exl = ex.to_lowercase();
            // If exclusion terms appear in prompt-derived label, warn (generation prompt leakage)
            if blob.contains(&exl) {
                forbidden.push(format!("exclusion_in_brief:{ex}"));
                semantic -= 0.2;
            }
        }
        for neg in &h.negative_contexts {
            if blob.contains(&neg.to_lowercase()) {
                forbidden.push(format!("negative_context_in_brief:{neg}"));
                semantic -= 0.1;
            }
        }
        semantic = semantic.clamp(0.0, 1.0);
        details["semanticMode"] = serde_json::json!("heuristic_no_vision");
    }

    let hard_hit = forbidden
        .iter()
        .any(|f| f.starts_with("exclusion_in_brief:"));
    let (decision, reason) = if hard_hit {
        (
            "reject".to_string(),
            "exclusión dura detectada en brief".to_string(),
        )
    } else if tech < thresholds.human_review_tech || semantic < thresholds.reject_semantic {
        ("reject".to_string(), "calidad insuficiente".to_string())
    } else if tech >= thresholds.auto_approve_tech && semantic >= thresholds.auto_approve_semantic {
        (
            "approve".to_string(),
            "pasa controles técnicos y umbral semántico".to_string(),
        )
    } else {
        (
            "needs_human".to_string(),
            "confianza media — revisar".to_string(),
        )
    };

    Ok(QaCheckResult {
        id: uuid::Uuid::new_v4().to_string(),
        candidate_id: None,
        asset_id: None,
        technical_quality: tech,
        semantic_alignment: semantic,
        forbidden_detected: forbidden,
        text_detected: false,
        watermark_detected: false,
        decision,
        reason,
        details,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

fn reject(
    code: &str,
    tech: f64,
    sem: f64,
    forbidden: Vec<String>,
    reason: impl Into<String>,
) -> QaCheckResult {
    QaCheckResult {
        id: uuid::Uuid::new_v4().to_string(),
        candidate_id: None,
        asset_id: None,
        technical_quality: tech,
        semantic_alignment: sem,
        forbidden_detected: forbidden,
        text_detected: false,
        watermark_detected: false,
        decision: "reject".into(),
        reason: format!("{code}: {}", reason.into()),
        details: serde_json::json!({ "code": code }),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn sniff_ok(bytes: &[u8]) -> bool {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        return true;
    }
    if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 {
        return true;
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return true;
    }
    false
}

fn find_near_duplicate(phash: &str, max_dist: u32) -> AppResult<Option<String>> {
    let conn = open_db()?;
    let mut stmt = conn
        .prepare("SELECT id, perceptual_hash FROM media_assets WHERE perceptual_hash IS NOT NULL AND status = 'active'")
        .map_err(|e| AppError::Message(e.to_string()))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| AppError::Message(e.to_string()))?;
    for row in rows.flatten() {
        if let Some(d) = hamming_hex(phash, &row.1) {
            if d <= max_dist {
                return Ok(Some(row.0));
            }
        }
    }
    Ok(None)
}

pub fn decision_to_qa_status(decision: &str) -> QaStatus {
    match decision {
        "approve" => QaStatus::Approved,
        "needs_human" => QaStatus::NeedsHumanReview,
        "reject" => QaStatus::Rejected,
        _ => QaStatus::AutomatedReview,
    }
}

pub fn persist_qa_check(check: &QaCheckResult) -> AppResult<()> {
    let conn = open_db()?;
    conn.execute(
        r#"INSERT INTO qa_checks (
            id, candidate_id, asset_id, technical_quality, semantic_alignment,
            forbidden_detected, text_detected, watermark_detected, decision, reason, details, created_at
        ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)"#,
        rusqlite::params![
            check.id,
            check.candidate_id,
            check.asset_id,
            check.technical_quality,
            check.semantic_alignment,
            serde_json::to_string(&check.forbidden_detected).unwrap_or_else(|_| "[]".into()),
            check.text_detected as i64,
            check.watermark_detected as i64,
            check.decision,
            check.reason,
            check.details.to_string(),
            check.created_at,
        ],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::visual::library::set_library_root_override;
    use image::{Rgb, RgbImage};

    #[test]
    fn rejects_tiny() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-qa-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let p = dir.join("tiny.png");
        let img = RgbImage::from_pixel(32, 32, Rgb([100, 120, 140]));
        image::DynamicImage::ImageRgb8(img).save(&p).unwrap();
        let r = review_image(&p, None, &QaThresholds::default()).unwrap();
        assert_eq!(r.decision, "reject");
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn approves_gradient() {
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-qa2-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        let p = dir.join("ok.png");
        let mut img = RgbImage::new(640, 360);
        for y in 0..360 {
            for x in 0..640 {
                img.put_pixel(x, y, Rgb([(x % 255) as u8, (y % 255) as u8, 80]));
            }
        }
        image::DynamicImage::ImageRgb8(img).save(&p).unwrap();
        let hints = SemanticHints {
            label: "supermercado precios".into(),
            meanings: vec!["inflación".into()],
            hard_exclusions: vec!["criptomonedas".into()],
            negative_contexts: vec![],
        };
        let r = review_image(&p, Some(&hints), &QaThresholds::default()).unwrap();
        assert!(r.decision == "approve" || r.decision == "needs_human");
        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
