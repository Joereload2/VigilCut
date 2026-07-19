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
