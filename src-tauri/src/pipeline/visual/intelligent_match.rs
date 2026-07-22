//! Explainable matching: search library before generation.

use crate::models::visual::{AssetStatus, LicenseStatus, MediaAsset};
use crate::models::visual_intel::{MatchCandidate, NeedCoverage, VisualNeed};
use crate::pipeline::visual::library::list_active_assets;

#[derive(Debug, Clone)]
pub struct MatchOptions {
    pub min_score: f64,
    pub prefer_aspect: Option<String>,
    /// Asset ids already used in this project (penalty)
    pub used_in_project: Vec<String>,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            min_score: 0.28,
            prefer_aspect: Some("16:9".into()),
            used_in_project: Vec::new(),
        }
    }
}

/// Rank assets for a need. Never returns blocked / unknown-license for auto path.
pub fn match_need(need: &VisualNeed, opts: &MatchOptions) -> Vec<MatchCandidate> {
    let assets = list_active_assets().unwrap_or_default();
    match_need_against(need, &assets, opts)
}

pub fn match_need_against(
    need: &VisualNeed,
    assets: &[MediaAsset],
    opts: &MatchOptions,
) -> Vec<MatchCandidate> {
    let mut out = Vec::new();
    for asset in assets {
        if !matches!(asset.status, AssetStatus::Active) {
            continue;
        }
        if matches!(asset.license_status, LicenseStatus::Unknown) {
            continue;
        }
        if let Some((score, reasons, exclusions, format_ok, will_crop)) =
            score_asset(need, asset, opts)
        {
            if score < opts.min_score {
                continue;
            }
            out.push(MatchCandidate {
                asset_id: asset.id.clone(),
                asset_title: asset.title.clone(),
                score,
                reasons,
                exclusions_checked: exclusions,
                format_ok,
                will_crop,
                times_used: asset.times_used,
                thumbnail_path: asset.thumbnail_path.clone(),
            });
        }
    }
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn score_asset(
    need: &VisualNeed,
    asset: &MediaAsset,
    opts: &MatchOptions,
) -> Option<(f64, Vec<String>, Vec<String>, bool, bool)> {
    let mut score = 0.0_f64;
    let mut reasons = Vec::new();
    let mut exclusions_checked = Vec::new();

    let label = need.label.to_lowercase();
    let terms: Vec<String> = need.terms.iter().map(|t| t.to_lowercase()).collect();

    // Hard exclusions on asset vs need contexts
    for ex in asset.hard_exclusions.iter().chain(need.hard_exclusions.iter()) {
        let exl = ex.to_lowercase();
        exclusions_checked.push(ex.clone());
        // If need requires a context that asset hard-excludes — skip
        for ctx in &need.required_contexts {
            if ctx.to_lowercase() == exl || ctx.to_lowercase().contains(&exl) {
                return None;
            }
        }
        // If asset is tagged with excluded meaning for this need's terms
        for t in &terms {
            if t == &exl {
                return None;
            }
        }
    }

    // Negative contexts on asset: if need terms hit them, hard fail
    for neg in &asset.negative_contexts {
        let nl = neg.to_lowercase();
        exclusions_checked.push(format!("neg:{neg}"));
        if terms.iter().any(|t| t == &nl || nl.contains(t)) || label.contains(&nl) {
            // need is about a negative context for this asset
            if need
                .required_contexts
                .iter()
                .any(|c| c.to_lowercase().contains(&nl) || nl.contains(&c.to_lowercase()))
            {
                return None;
            }
        }
    }

    for c in asset
        .concepts
        .iter()
        .chain(asset.meanings.iter())
        .chain(asset.literal_description.iter())
    {
        let cl = c.to_lowercase();
        if cl == label || terms.iter().any(|t| t == &cl || cl.contains(t) || t.contains(&cl)) {
            score += 0.4;
            reasons.push(format!("concept:{c}"));
        }
    }
    for t in &asset.tags {
        let tl = t.to_lowercase();
        if terms.iter().any(|x| x == &tl || tl.contains(x)) || label.contains(&tl) {
            score += 0.22;
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
    for pc in &asset.positive_contexts {
        let pl = pc.to_lowercase();
        if need
            .required_contexts
            .iter()
            .any(|c| c.to_lowercase() == pl || c.to_lowercase().contains(&pl))
        {
            score += 0.12;
            reasons.push(format!("context:{pc}"));
        }
    }

    // Format / aspect
    let mut format_ok = true;
    let mut will_crop = false;
    if let Some(pref) = &opts.prefer_aspect {
        if let Some(ar) = &asset.aspect_ratio {
            if ar == pref {
                score += 0.08;
                reasons.push(format!("aspect:{ar}"));
            } else if (ar == "landscape" && pref == "16:9")
                || (ar == "portrait" && pref == "9:16")
            {
                score += 0.03;
                will_crop = true;
                reasons.push("aspect:crop_ok".into());
            } else if ar != pref {
                format_ok = ar == "landscape" || ar == "portrait";
                will_crop = true;
                score -= 0.05;
                reasons.push("aspect:mismatch".into());
            }
        }
    }

    // Reuse penalties
    if opts.used_in_project.contains(&asset.id) && !asset.allow_same_video_repeat {
        score -= 0.35;
        reasons.push("penalty:same_project".into());
    }
    if asset.times_used > 0 {
        score -= 0.05 * (asset.times_used as f64).min(6.0);
        reasons.push("penalty:used_before".into());
    }
    if let Some(q) = asset.technical_score.or(asset.quality_score) {
        score += 0.05 * q;
    }

    if reasons.is_empty() && score < 0.2 {
        return None;
    }
    Some((
        score.clamp(0.0, 1.5),
        reasons,
        exclusions_checked,
        format_ok,
        will_crop,
    ))
}

/// Apply best match onto need if above threshold.
pub fn apply_best_match(need: &mut VisualNeed, opts: &MatchOptions) -> bool {
    let ranked = match_need(need, opts);
    if let Some(best) = ranked.first() {
        need.matched_asset_id = Some(best.asset_id.clone());
        need.match_score = Some(best.score);
        need.match_reasons = best.reasons.clone();
        need.coverage = NeedCoverage::Matched;
        need.updated_at = chrono::Utc::now().to_rfc3339();
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual_intel::VisualNeed;

    fn asset(title: &str, concepts: &[&str]) -> MediaAsset {
        MediaAsset {
            id: uuid::Uuid::new_v4().to_string(),
            kind: "image".into(),
            managed_path: "/tmp/x.png".into(),
            thumbnail_path: None,
            sha256: format!("sha-{title}"),
            title: title.into(),
            description: None,
            tags: concepts.iter().map(|s| (*s).into()).collect(),
            concepts: concepts.iter().map(|s| (*s).into()).collect(),
            category: None,
            width: 1920,
            height: 1080,
            orientation: "landscape".into(),
            mime_type: "image/png".into(),
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
            literal_description: concepts.iter().map(|s| (*s).into()).collect(),
            meanings: vec!["inflación".into()],
            positive_contexts: vec!["economía doméstica".into()],
            negative_contexts: vec!["criptomonedas".into()],
            hard_exclusions: vec!["marcas comerciales".into()],
            aspect_ratio: Some("16:9".into()),
            safe_area: Some("center".into()),
            perceptual_hash: None,
            qa_status: Default::default(),
            technical_score: Some(0.9),
            semantic_score: None,
            provenance: None,
            commercial_use: Some(true),
        }
    }

    #[test]
    fn matches_concept_and_excludes() {
        let a = asset("Super precios", &["supermercado", "precios"]);
        let mut need = VisualNeed::from_label("p1", "supermercado");
        need.required_contexts = vec!["economía doméstica".into()];
        let ranked = match_need_against(&need, &[a.clone()], &MatchOptions::default());
        assert!(!ranked.is_empty());
        assert!(ranked[0].score > 0.3);

        let need2 = VisualNeed::from_label("p1", "marcas comerciales");
        let a2 = asset("x", &["foo"]);
        let ranked4 = match_need_against(&need2, &[a2], &MatchOptions::default());
        // hard exclusion "marcas comerciales" on asset blocks need with that term
        assert!(ranked4.is_empty());
    }

    #[test]
    fn penalty_same_project() {
        let a = asset("Super", &["inflacion"]);
        let need = VisualNeed::from_label("p1", "inflacion");
        let mut opts = MatchOptions::default();
        opts.used_in_project = vec![a.id.clone()];
        let ranked = match_need_against(&need, &[a], &opts);
        if let Some(r) = ranked.first() {
            assert!(r.reasons.iter().any(|x| x.contains("same_project")));
        }
    }
}
