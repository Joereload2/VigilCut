//! Explainable visual matching / ranking.

use crate::models::event::Span;
use crate::models::visual::{
    AssetStatus, LicenseStatus, MediaAsset, SemanticEvent, SuggestionStatus, VisualSuggestion,
};

#[derive(Debug, Clone)]
pub struct MatchConfig {
    pub max_per_minute: f64,
    pub min_gap_secs: f64,
    pub duration_secs: f64,
}

impl Default for MatchConfig {
    fn default() -> Self {
        Self {
            max_per_minute: 3.5,
            min_gap_secs: 8.0,
            duration_secs: 4.0,
        }
    }
}

pub fn rank_suggestions(
    events: &[SemanticEvent],
    assets: &[MediaAsset],
    output_duration: f64,
    cfg: &MatchConfig,
) -> Vec<VisualSuggestion> {
    let mut suggestions = Vec::new();
    let mut used_assets: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut last_end = -999.0_f64;

    let mut events: Vec<&SemanticEvent> = events.iter().collect();
    events.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let max_total = ((output_duration / 60.0) * cfg.max_per_minute)
        .ceil()
        .max(1.0) as usize;

    for ev in events {
        if suggestions.len() >= max_total {
            break;
        }
        let Some(out) = ev.output_span else {
            continue;
        };
        if out.start < last_end + cfg.min_gap_secs {
            continue;
        }

        let mut best: Option<(f64, Vec<String>, &MediaAsset)> = None;
        for asset in assets {
            if !matches!(asset.status, AssetStatus::Active) {
                continue;
            }
            if used_assets.contains(&asset.id) && !asset.allow_same_video_repeat {
                continue;
            }
            let (score, reasons) = score_pair(ev, asset);
            if score < 0.25 {
                continue;
            }
            if best.as_ref().map(|(s, _, _)| score > *s).unwrap_or(true) {
                best = Some((score, reasons, asset));
            }
        }

        if let Some((score, reasons, asset)) = best {
            let end = (out.start + cfg.duration_secs).min(output_duration);
            if end - out.start < 1.5 {
                continue;
            }
            used_assets.insert(asset.id.clone());
            last_end = end;
            suggestions.push(VisualSuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                semantic_event_id: ev.id.clone(),
                asset_id: asset.id.clone(),
                source_span: ev.source_span,
                output_span: Span::new(out.start, end),
                match_reasons: reasons,
                match_score: score,
                alternatives: Vec::new(),
                status: SuggestionStatus::Suggested,
                asset_title: Some(asset.title.clone()),
                thumbnail_path: asset.thumbnail_path.clone(),
            });
        }
    }

    suggestions.sort_by(|a, b| {
        a.output_span
            .start
            .partial_cmp(&b.output_span.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    suggestions
}

fn score_pair(ev: &SemanticEvent, asset: &MediaAsset) -> (f64, Vec<String>) {
    let mut score = 0.0_f64;
    let mut reasons = Vec::new();
    let label = ev.label.to_lowercase();
    let terms: Vec<String> = ev.terms.iter().map(|t| t.to_lowercase()).collect();

    for c in &asset.concepts {
        let cl = c.to_lowercase();
        if cl == label || terms.iter().any(|t| t == &cl || cl.contains(t) || t.contains(&cl)) {
            score += 0.45;
            reasons.push(format!("concept:{c}"));
        }
    }
    for t in &asset.tags {
        let tl = t.to_lowercase();
        if terms.iter().any(|x| x == &tl || tl.contains(x)) || label.contains(&tl) {
            score += 0.25;
            reasons.push(format!("tag:{t}"));
        }
    }
    let title = asset.title.to_lowercase();
    for t in &terms {
        if title.contains(t) {
            score += 0.15;
            reasons.push(format!("title:{t}"));
            break;
        }
    }

    if let Some(q) = asset.quality_score {
        score += 0.05 * q;
    }

    // Penalties
    if asset.times_used > 0 {
        score -= 0.08 * (asset.times_used as f64).min(5.0);
        reasons.push("penalty:used_before".into());
    }
    if matches!(asset.license_status, LicenseStatus::Unknown) {
        score -= 0.12;
        reasons.push("penalty:license_unknown".into());
    }
    if matches!(asset.status, AssetStatus::Blocked) {
        score = 0.0;
    }

    (score.clamp(0.0, 1.0), reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual::{AssetStatus, LicenseStatus, SemanticKind};

    fn asset(id: &str, concepts: &[&str]) -> MediaAsset {
        MediaAsset {
            id: id.into(),
            kind: "image".into(),
            managed_path: "/x.jpg".into(),
            thumbnail_path: None,
            sha256: id.into(),
            title: concepts.join(" "),
            description: None,
            tags: concepts.iter().map(|s| (*s).into()).collect(),
            concepts: concepts.iter().map(|s| (*s).into()).collect(),
            category: None,
            width: 1920,
            height: 1080,
            orientation: "landscape".into(),
            mime_type: "image/jpeg".into(),
            file_size: 1000,
            license_status: LicenseStatus::Owned,
            source: None,
            attribution: None,
            times_used: 0,
            last_used_at: None,
            allow_same_video_repeat: false,
            minimum_videos_before_reuse: 0,
            quality_score: Some(0.8),
            status: AssetStatus::Active,
            original_path: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn ranks_matching_concept() {
        let ev = SemanticEvent {
            id: "e1".into(),
            run_id: "r".into(),
            kind: SemanticKind::Concept,
            source_span: Span::new(10.0, 14.0),
            output_span: Some(Span::new(8.0, 12.0)),
            label: "inflacion".into(),
            terms: vec!["inflación".into(), "precios".into()],
            score: 0.8,
            transcript_segment_ids: vec![],
            method: "test".into(),
            payload: serde_json::json!({}),
        };
        let assets = vec![
            asset("a1", &["inflacion", "economia"]),
            asset("a2", &["viaje", "playa"]),
        ];
        let s = rank_suggestions(&[ev], &assets, 120.0, &MatchConfig::default());
        assert!(!s.is_empty());
        assert_eq!(s[0].asset_id, "a1");
        assert!(s[0].match_reasons.iter().any(|r| r.starts_with("concept:")));
    }
}
