//! Temporal + textual deduplication of clip candidates.

use crate::models::clipping::ClipCandidate;

/// Group overlapping / similar clips; keep best as primary, mark rest as variants.
pub fn dedupe_and_group(mut candidates: Vec<ClipCandidate>) -> Vec<ClipCandidate> {
    if candidates.is_empty() {
        return candidates;
    }
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut assigned = vec![false; candidates.len()];

    for i in 0..candidates.len() {
        if assigned[i] {
            continue;
        }
        let mut group = vec![i];
        assigned[i] = true;
        for j in (i + 1)..candidates.len() {
            if assigned[j] {
                continue;
            }
            if similar(&candidates[i], &candidates[j]) {
                group.push(j);
                assigned[j] = true;
            }
        }
        groups.push(group);
    }

    let mut out = Vec::new();
    for group in groups {
        let primary_idx = group[0];
        let gid = candidates[primary_idx].id.clone();
        for (k, &idx) in group.iter().enumerate() {
            let mut c = candidates[idx].clone();
            c.variant_group_id = gid.clone();
            c.is_primary_variant = k == 0;
            if k > 0 && c.status == crate::models::clipping::ClipReviewStatus::Suggested {
                // Keep as suggested secondary variant (not discarded)
            }
            out.push(c);
        }
    }

    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn similar(a: &ClipCandidate, b: &ClipCandidate) -> bool {
    let overlap = time_overlap(a.start, a.end, b.start, b.end);
    let min_dur = a.duration.min(b.duration).max(0.1);
    let ratio = overlap / min_dur;
    if ratio >= 0.55 {
        return true;
    }
    jaccard_words(&a.transcript, &b.transcript) >= 0.65
}

fn time_overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    let s = a0.max(b0);
    let e = a1.min(b1);
    (e - s).max(0.0)
}

fn jaccard_words(a: &str, b: &str) -> f64 {
    use std::collections::HashSet;
    let a_l = a.to_lowercase();
    let b_l = b.to_lowercase();
    let wa: HashSet<&str> = a_l
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .collect();
    let wb: HashSet<&str> = b_l
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .collect();
    if wa.is_empty() || wb.is_empty() {
        return 0.0;
    }
    let inter = wa.intersection(&wb).count() as f64;
    let uni = wa.union(&wb).count() as f64;
    inter / uni
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::clipping::{ClipFraming, ClipReviewStatus, ClipScoreBreakdown};

    fn cand(id: &str, start: f64, end: f64, text: &str, score: f64) -> ClipCandidate {
        ClipCandidate {
            id: id.into(),
            analysis_run_id: "r".into(),
            source_media_path: "m.mp4".into(),
            start,
            end,
            duration: end - start,
            transcript: text.into(),
            title: text.into(),
            summary: text.into(),
            score,
            confidence: 0.5,
            breakdown: ClipScoreBreakdown::default(),
            reasons: vec![],
            warnings: vec![],
            strengths: vec![],
            risks: vec![],
            status: ClipReviewStatus::Suggested,
            variant_group_id: id.into(),
            is_primary_variant: true,
            framing: ClipFraming::default(),
            original_start: start,
            original_end: end,
            export_path: None,
            error: None,
        }
    }

    #[test]
    fn groups_overlap() {
        let list = vec![
            cand("a", 0.0, 30.0, "hola mundo importante", 80.0),
            cand("b", 5.0, 35.0, "hola mundo importante hoy", 70.0),
            cand("c", 100.0, 130.0, "otro tema totalmente distinto aquí", 60.0),
        ];
        let out = dedupe_and_group(list);
        assert_eq!(out.len(), 3);
        let primaries = out.iter().filter(|c| c.is_primary_variant).count();
        assert_eq!(primaries, 2);
    }
}
