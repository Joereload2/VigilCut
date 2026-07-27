//! Composition supervision: conflict evaluation + magnetic snap.
//! B-roll is composition ops on the output timeline — not Segment legacy.

use uuid::Uuid;

use crate::models::visual::{
    CompositionIssue, PlacementLayout, PlacementMode, ReviewStatus, VisualPlacement, VisualPlan,
};

/// Evaluate placements against overlaps, protected times, spatial zones, confidence.
/// Updates `plan.issues` and may flip `review_status` to Conflict (unless manual_override keep).
pub fn evaluate_composition(plan: &mut VisualPlan) {
    let mut issues: Vec<CompositionIssue> = Vec::new();
    let n = plan.placements.len();

    for i in 0..n {
        let pl = &plan.placements[i];
        if pl.status != "active" {
            continue;
        }

        // Low semantic confidence
        if pl.confidence < 0.55 && pl.provenance != "manual" {
            issues.push(issue(
                &pl.id,
                "semantic_low",
                "warn",
                format!(
                    "Correspondencia semántica dudosa ({:.0}%). Revisa imagen vs frase.",
                    pl.confidence * 100.0
                ),
                None,
            ));
        }

        // Very short / long duration heuristics
        let dur = pl.duration();
        if dur < 0.8 {
            issues.push(issue(
                &pl.id,
                "timing_unclear",
                "warn",
                "Entrada/salida muy corta — puede parpadear.".into(),
                None,
            ));
        } else if dur > 12.0 {
            issues.push(issue(
                &pl.id,
                "timing_unclear",
                "info",
                "B-roll largo (>12s): confirma que sigue la idea.".into(),
                None,
            ));
        }

        // Temporal protected ranges
        if plan.is_protected(pl.output_start, pl.output_end) {
            issues.push(issue(
                &pl.id,
                "protected_time",
                "error",
                "Cae en un tramo temporal sin B-roll.".into(),
                None,
            ));
        }

        // Overlap with other active placements
        for j in (i + 1)..n {
            let other = &plan.placements[j];
            if other.status != "active" {
                continue;
            }
            if pl.output_start < other.output_end && pl.output_end > other.output_start {
                issues.push(issue(
                    &pl.id,
                    "overlap",
                    "warn",
                    format!(
                        "Solapa con otro B-roll ({})",
                        other
                            .label
                            .as_deref()
                            .unwrap_or(&other.id[..8.min(other.id.len())])
                    ),
                    None,
                ));
            }
        }

        // Spatial zone invasions (midpoint of placement)
        let mid = (pl.output_start + pl.output_end) * 0.5;
        let (cx, cy, bw, bh) = pl.frame_rect();
        if pl.mode.is_overlay() {
            for z in &plan.spatial_zones {
                if !z.active_at(mid) {
                    continue;
                }
                let avoid = pl.avoid_zones.iter().any(|k| k == &z.kind)
                    || matches!(
                        z.kind.as_str(),
                        "face" | "subtitle" | "text" | "logo" | "product"
                    );
                if !avoid {
                    continue;
                }
                let score = z.rect_overlap_score(cx, cy, bw, bh);
                if score > 0.18 {
                    let kind = match z.kind.as_str() {
                        "face" => "face_covered",
                        "subtitle" => "subtitle_covered",
                        "safe_area" => "safe_area",
                        _ => "safe_area",
                    };
                    let sev = if score > 0.4 || z.severity == "error" {
                        "error"
                    } else {
                        "warn"
                    };
                    // Suggest opposite corner
                    let (sx, sy) = suggest_away(cx, cy, &z.kind);
                    issues.push(issue(
                        &pl.id,
                        kind,
                        sev,
                        format!(
                            "Posible cobertura de {} ({:.0}% solape).{}",
                            z.label.as_deref().unwrap_or(&z.kind),
                            score * 100.0,
                            if pl.manual_override {
                                " Override manual activo."
                            } else {
                                ""
                            }
                        ),
                        Some((sx, sy, pl.layout.w)),
                    ));
                }
            }
        }

        // Fullframe covers faces by design — soft note only if confidence low
        if pl.mode == PlacementMode::Fullframe && pl.confidence < 0.6 {
            issues.push(issue(
                &pl.id,
                "face_covered",
                "info",
                "Fullscreen oculta el video: confirma que la frase lo justifica.".into(),
                None,
            ));
        }

        // Aspect: extreme narrow overlays
        if pl.mode.is_overlay() && pl.layout.w < 0.12 {
            issues.push(issue(
                &pl.id,
                "aspect",
                "warn",
                "Tamaño muy pequeño — puede no leerse en móvil.".into(),
                None,
            ));
        }
    }

    // Apply review_status from issues (do not demote manual approved without conflict error)
    for pl in plan.placements.iter_mut() {
        if pl.status != "active" {
            continue;
        }
        let has_error = issues
            .iter()
            .any(|i| i.placement_id == pl.id && i.severity == "error");
        let has_warn = issues
            .iter()
            .any(|i| i.placement_id == pl.id && i.severity == "warn");
        if has_error || (has_warn && pl.review_status != ReviewStatus::Approved) {
            pl.review_status = ReviewStatus::Conflict;
        } else if pl.confidence >= 0.72
            && !has_warn
            && !has_error
            && pl.review_status == ReviewStatus::Pending
        {
            // Auto-quiet high confidence
            pl.review_status = ReviewStatus::Approved;
        }
    }

    plan.issues = issues;
    plan.warnings = plan
        .issues
        .iter()
        .filter(|i| i.severity != "info")
        .map(|i| i.message.clone())
        .collect();
    plan.touch();
}

fn issue(
    placement_id: &str,
    kind: &str,
    severity: &str,
    message: String,
    suggested: Option<(f64, f64, f64)>,
) -> CompositionIssue {
    CompositionIssue {
        id: Uuid::new_v4().to_string(),
        placement_id: placement_id.into(),
        kind: kind.into(),
        severity: severity.into(),
        message,
        suggested_x: suggested.map(|s| s.0),
        suggested_y: suggested.map(|s| s.1),
        suggested_w: suggested.map(|s| s.2),
    }
}

fn suggest_away(cx: f64, cy: f64, zone_kind: &str) -> (f64, f64) {
    // Prefer opposite of current center; for face push to upper-right or lower-right
    if zone_kind == "face" {
        if cx < 0.5 {
            (0.82, 0.18)
        } else {
            (0.18, 0.18)
        }
    } else if zone_kind == "subtitle" || zone_kind == "safe_area" {
        (cx.clamp(0.2, 0.8), 0.22)
    } else if cy > 0.5 {
        (cx, 0.2)
    } else {
        (cx, 0.75)
    }
}

/// Magnetic snap of a time to nearest anchors within threshold.
pub fn snap_time(t: f64, anchors: &[f64], threshold: f64) -> f64 {
    let mut best = t;
    let mut best_d = threshold;
    for &a in anchors {
        let d = (a - t).abs();
        if d <= best_d {
            best_d = d;
            best = a;
        }
    }
    best.max(0.0)
}

/// Build snap anchors from transcript words/phrases (output times), cuts, placement edges.
pub fn collect_snap_anchors(
    transcript_output_edges: &[f64],
    cut_output_edges: &[f64],
    other_placement_edges: &[f64],
) -> Vec<f64> {
    let mut v = Vec::with_capacity(
        transcript_output_edges.len() + cut_output_edges.len() + other_placement_edges.len() + 1,
    );
    v.push(0.0);
    v.extend_from_slice(transcript_output_edges);
    v.extend_from_slice(cut_output_edges);
    v.extend_from_slice(other_placement_edges);
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-4);
    v
}

/// Apply snap to placement edges; returns (start, end).
pub fn snap_placement_edges(start: f64, end: f64, anchors: &[f64], threshold: f64) -> (f64, f64) {
    let s = snap_time(start, anchors, threshold);
    let e = snap_time(end, anchors, threshold).max(s + 0.25);
    (s, e)
}

/// Restore AI-suggested layout/mode on a placement.
pub fn restore_suggested(pl: &mut VisualPlacement) {
    if let Some(layout) = pl.suggested_layout.clone() {
        pl.layout = layout.clamp();
    } else {
        pl.layout = PlacementLayout::for_mode(pl.mode);
    }
    if let Some(m) = pl.suggested_mode {
        pl.mode = m;
    }
    pl.manual_override = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual::VisualPlan;

    #[test]
    fn snap_to_anchor() {
        let a = vec![0.0, 1.0, 2.5, 5.0];
        assert!((snap_time(1.1, &a, 0.2) - 1.0).abs() < 1e-6);
        assert!((snap_time(3.0, &a, 0.2) - 3.0).abs() < 1e-6); // no snap
    }

    #[test]
    fn evaluate_flags_overlap() {
        let mut plan = VisualPlan::new("r", "m.mp4", "fp");
        plan.placements.push(VisualPlacement::manual(
            "a",
            1.0,
            4.0,
            PlacementMode::PictureInPicture,
            PlacementLayout::for_mode(PlacementMode::PictureInPicture),
            "cover",
            Some("one".into()),
        ));
        plan.placements.push(VisualPlacement::manual(
            "b",
            2.0,
            5.0,
            PlacementMode::PictureInPicture,
            PlacementLayout::for_mode(PlacementMode::PictureInPicture),
            "cover",
            Some("two".into()),
        ));
        evaluate_composition(&mut plan);
        assert!(plan.issues.iter().any(|i| i.kind == "overlap"));
    }
}
