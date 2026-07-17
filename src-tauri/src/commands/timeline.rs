use crate::error::{AppError, AppResult};
use crate::models::segment::{Segment, SegmentDecision, SegmentEdit, SegmentKind};
use uuid::Uuid;

#[tauri::command]
pub fn apply_segment_edits(
    mut segments: Vec<Segment>,
    edits: Vec<SegmentEdit>,
) -> AppResult<Vec<Segment>> {
    for edit in edits {
        if let Some(seg) = segments.iter_mut().find(|s| s.id == edit.id) {
            if let Some(d) = edit.decision {
                seg.decision = d;
            }
            if let Some(s) = edit.start {
                seg.start = s;
            }
            if let Some(e) = edit.end {
                seg.end = e;
            }
            if edit.label.is_some() {
                seg.label = edit.label;
            }
        }
    }
    segments.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(segments)
}

#[tauri::command]
pub fn merge_adjacent_segments(
    segments: Vec<Segment>,
    max_gap: Option<f64>,
) -> AppResult<Vec<Segment>> {
    let gap = max_gap.unwrap_or(0.05);
    if segments.is_empty() {
        return Ok(segments);
    }

    let mut sorted = segments;
    sorted.sort_by(|a, b| {
        a.start
            .partial_cmp(&b.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut out: Vec<Segment> = Vec::new();
    for seg in sorted {
        if let Some(last) = out.last_mut() {
            if last.kind == seg.kind
                && last.decision == seg.decision
                && seg.start <= last.end + gap
            {
                last.end = last.end.max(seg.end);
                last.confidence = last.confidence.min(seg.confidence);
                continue;
            }
        }
        out.push(seg);
    }
    Ok(out)
}

#[tauri::command]
pub fn split_segment_at(segments: Vec<Segment>, segment_id: String, time: f64) -> AppResult<Vec<Segment>> {
    let mut out = Vec::new();
    let mut found = false;

    for seg in segments {
        if seg.id == segment_id {
            if time <= seg.start + 0.01 || time >= seg.end - 0.01 {
                return Err(AppError::Invalid(
                    "Split time must be inside the segment".into(),
                ));
            }
            found = true;
            let mut left = seg.clone();
            left.id = Uuid::new_v4().to_string();
            left.end = time;

            let mut right = seg;
            right.id = Uuid::new_v4().to_string();
            right.start = time;
            // Splits become manual for clearer human review
            left.kind = SegmentKind::Manual;
            right.kind = SegmentKind::Manual;
            // Preserve decision
            left.decision = right.decision;
            if left.decision == SegmentDecision::Pending {
                left.decision = SegmentDecision::Keep;
                right.decision = SegmentDecision::Keep;
            }

            out.push(left);
            out.push(right);
        } else {
            out.push(seg);
        }
    }

    if !found {
        return Err(AppError::NotFound(format!("Segment {segment_id}")));
    }
    Ok(out)
}
