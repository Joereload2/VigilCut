//! Preselection profiles: mark top candidates, discard weak ones.

use crate::models::clipping::{ClipCandidate, ClipReviewStatus, ClippingOptions};

pub fn apply_preselection(candidates: &mut [ClipCandidate], options: &ClippingOptions) {
    let (max_pre, floor) = options.selection_profile.limits();
    let floor_score = floor * 100.0;

    let mut order: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.is_primary_variant)
        .map(|(i, _)| i)
        .collect();
    order.sort_by(|&a, &b| {
        candidates[b]
            .score
            .partial_cmp(&candidates[a].score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut pre_count = 0usize;
    for &idx in &order {
        let c = &mut candidates[idx];
        if c.score < floor_score * 0.55 {
            c.status = ClipReviewStatus::Discarded;
            continue;
        }
        if pre_count < max_pre && c.score >= floor_score {
            c.status = ClipReviewStatus::Preselected;
            pre_count += 1;
        } else if c.score >= floor_score * 0.75 {
            c.status = ClipReviewStatus::Suggested;
        } else {
            c.status = ClipReviewStatus::Discarded;
        }
    }

    // Snapshot primary statuses then apply to variants
    let primary_status: Vec<(String, ClipReviewStatus)> = candidates
        .iter()
        .filter(|c| c.is_primary_variant)
        .map(|c| (c.variant_group_id.clone(), c.status))
        .collect();

    for c in candidates.iter_mut() {
        if c.is_primary_variant {
            continue;
        }
        let st = primary_status
            .iter()
            .find(|(gid, _)| gid == &c.variant_group_id)
            .map(|(_, s)| *s)
            .unwrap_or(ClipReviewStatus::Discarded);
        c.status = match st {
            ClipReviewStatus::Discarded => ClipReviewStatus::Discarded,
            _ => ClipReviewStatus::Suggested,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::clipping::{
        ClipFraming, ClipScoreBreakdown, ClippingOptions, SelectionProfile,
    };

    fn c(id: &str, score: f64, primary: bool, gid: &str) -> ClipCandidate {
        ClipCandidate {
            id: id.into(),
            analysis_run_id: "r".into(),
            source_media_path: "m".into(),
            start: 0.0,
            end: 20.0,
            duration: 20.0,
            transcript: "x".into(),
            title: "t".into(),
            summary: "s".into(),
            score,
            confidence: 0.6,
            breakdown: ClipScoreBreakdown::default(),
            reasons: vec![],
            warnings: vec![],
            strengths: vec![],
            risks: vec![],
            status: ClipReviewStatus::Suggested,
            variant_group_id: gid.into(),
            is_primary_variant: primary,
            framing: ClipFraming::default(),
            original_start: 0.0,
            original_end: 20.0,
            export_path: None,
            error: None,
        }
    }

    #[test]
    fn preselects_top_under_balanced() {
        let mut list = vec![
            c("a", 90.0, true, "g1"),
            c("b", 80.0, true, "g2"),
            c("c", 40.0, true, "g3"),
            c("a2", 70.0, false, "g1"),
        ];
        let mut opts = ClippingOptions::default();
        opts.selection_profile = SelectionProfile::Balanced;
        apply_preselection(&mut list, &opts);
        let pre = list
            .iter()
            .filter(|x| x.status == ClipReviewStatus::Preselected)
            .count();
        assert!(pre >= 1);
        assert!(pre <= 8);
        // weak discarded
        assert_eq!(
            list.iter().find(|x| x.id == "c").unwrap().status,
            ClipReviewStatus::Discarded
        );
        // secondary stays non-primary
        assert!(!list.iter().find(|x| x.id == "a2").unwrap().is_primary_variant);
    }
}
