//! Vertical 9:16 framing (UV green-box crop + multi-panel vstack).
//! Face tracking is contracted via `tracking_ready` for a future detector.

use crate::models::clipping::{ClipFraming, FramingMode, LayoutPanel};
use crate::models::media::MediaInfo;

pub fn default_framing_for_media(info: &MediaInfo) -> ClipFraming {
    let mut f = ClipFraming::default();
    // Prefer slightly above center for talking-head
    f.center_y = 0.42;
    f.tracking_ready = false;
    if info.width > 0 && info.height > 0 {
        let ar = info.width as f64 / info.height as f64;
        // Already vertical-ish → less zoom
        if ar < 0.7 {
            f.zoom = 1.0;
            f.mode = FramingMode::FitWithBars;
        }
    }
    f
}

/// Crop a UV rectangle (0..1 of full source) then scale to out_w×out_h.
fn crop_uv_scale(
    center_x: f64,
    center_y: f64,
    width_frac: f64,
    height_frac: f64,
    src_w: f64,
    src_h: f64,
    out_w: u32,
    out_h: u32,
) -> String {
    let w = width_frac.clamp(0.08, 1.0);
    let h = height_frac.clamp(0.08, 1.0);
    let cx = center_x.clamp(0.0, 1.0);
    let cy = center_y.clamp(0.0, 1.0);
    let mut cw = (w * src_w).max(2.0);
    let mut ch = (h * src_h).max(2.0);
    let mut x = (cx - w / 2.0) * src_w;
    let mut y = (cy - h / 2.0) * src_h;
    // Keep crop inside frame
    if x < 0.0 {
        x = 0.0;
    }
    if y < 0.0 {
        y = 0.0;
    }
    if x + cw > src_w {
        cw = (src_w - x).max(2.0);
    }
    if y + ch > src_h {
        ch = (src_h - y).max(2.0);
    }
    // Even dimensions for yuv420
    let cw = (cw.floor() as u32) & !1;
    let ch = (ch.floor() as u32) & !1;
    let x = (x.floor() as u32) & !1;
    let y = (y.floor() as u32) & !1;
    // Fit: reduce proportionally to fit the panel (letterbox/pillarbox), never stretch
    format!(
        "crop={cw}:{ch}:{x}:{y},scale={out_w}:{out_h}:force_original_aspect_ratio=decrease,pad={out_w}:{out_h}:(ow-iw)/2:(oh-ih)/2:black"
    )
}

/// Stack 1..=3 UV panels vertically into 1080×1920.
fn compute_vstack_panels(
    panels: &[LayoutPanel],
    src_w: f64,
    src_h: f64,
    out_w: u32,
    out_h: u32,
) -> String {
    let n = panels.len().clamp(1, 3);
    let panel_h = (out_h as usize / n) as u32;
    // Adjust last panel for remainder so total == out_h
    let last_h = out_h - panel_h * (n as u32 - 1);

    if n == 1 {
        let p = &panels[0];
        return crop_uv_scale(
            p.center_x,
            p.center_y,
            p.width_frac,
            p.height_frac,
            src_w,
            src_h,
            out_w,
            out_h,
        );
    }

    // FFmpeg: split=2[s0][s1];[s0]crop...[p0];...;[p0][p1]vstack=2
    // Caller must use -filter_complex with optional [0:v] prefix.
    let split_labels: String = (0..n).map(|i| format!("[s{i}]")).collect();
    let mut filter = format!("split={n}{split_labels}");

    let mut stack_in = String::new();
    for i in 0..n {
        let p = &panels[i];
        let h = if i + 1 == n { last_h } else { panel_h };
        let crop = crop_uv_scale(
            p.center_x,
            p.center_y,
            p.width_frac,
            p.height_frac,
            src_w,
            src_h,
            out_w,
            h,
        );
        filter.push_str(&format!(";[s{i}]{crop}[p{i}]"));
        stack_in.push_str(&format!("[p{i}]"));
    }
    filter.push_str(&format!(";{stack_in}vstack={n}"));
    filter
}

/// Build FFmpeg video filter for 9:16 output from framing config.
pub fn compute_crop_filter(framing: &ClipFraming, src_w: u32, src_h: u32) -> String {
    let out_w = framing.output_width.max(2);
    let out_h = framing.output_height.max(2);
    let src_w = src_w.max(2) as f64;
    let src_h = src_h.max(2) as f64;

    // Multi-panel layout from UI green boxes
    if !framing.panels.is_empty() {
        return compute_vstack_panels(&framing.panels, src_w, src_h, out_w, out_h);
    }

    match framing.mode {
        FramingMode::BlurredBackground => {
            format!(
                "split[bg][fg];\
                 [bg]scale={out_w}:{out_h}:force_original_aspect_ratio=increase,crop={out_w}:{out_h},boxblur=20:5[bg];\
                 [fg]scale=-2:{out_h}:force_original_aspect_ratio=decrease[fg];\
                 [bg][fg]overlay=(W-w)/2:(H-h)/2"
            )
        }
        FramingMode::FitWithBars => {
            format!(
                "scale={out_w}:{out_h}:force_original_aspect_ratio=decrease,\
                 pad={out_w}:{out_h}:(ow-iw)/2:(oh-ih)/2:black"
            )
        }
        FramingMode::AutoCenter | FramingMode::Manual => {
            // Prefer exact UV region from green selector when present
            if framing.width_frac > 0.05 && framing.height_frac > 0.05 {
                return crop_uv_scale(
                    framing.center_x,
                    framing.center_y,
                    framing.width_frac,
                    framing.height_frac,
                    src_w,
                    src_h,
                    out_w,
                    out_h,
                );
            }
            // Legacy zoom-based 9:16 crop
            let target_ar = 9.0 / 16.0;
            let crop_h = src_h / framing.zoom.max(1.0);
            let crop_w = (crop_h * target_ar).min(src_w);
            let crop_h = (crop_w / target_ar).min(src_h);
            let cx = (framing.center_x.clamp(0.05, 0.95) * src_w) - crop_w / 2.0;
            let cy = (framing.center_y.clamp(0.05, 0.95) * src_h) - crop_h / 2.0;
            let x = cx.clamp(0.0, (src_w - crop_w).max(0.0));
            let y = cy.clamp(0.0, (src_h - crop_h).max(0.0));
            format!("crop={crop_w:.0}:{crop_h:.0}:{x:.0}:{y:.0},scale={out_w}:{out_h}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_crop_mentions_scale() {
        let f = ClipFraming::default();
        let filter = compute_crop_filter(&f, 1920, 1080);
        assert!(filter.contains("crop="));
        assert!(filter.contains("1080"));
        assert!(filter.contains("1920"));
    }

    #[test]
    fn uv_region_crop() {
        let mut f = ClipFraming::default();
        f.mode = FramingMode::Manual;
        f.center_x = 0.25;
        f.center_y = 0.5;
        f.width_frac = 0.4;
        f.height_frac = 0.9;
        let filter = compute_crop_filter(&f, 1920, 1080);
        assert!(filter.contains("crop="));
        assert!(filter.contains("scale=1080:1920"));
    }

    #[test]
    fn multi_panel_vstack() {
        let mut f = ClipFraming::default();
        f.mode = FramingMode::Manual;
        f.panels = vec![
            LayoutPanel {
                center_x: 0.3,
                center_y: 0.45,
                width_frac: 0.35,
                height_frac: 0.9,
            },
            LayoutPanel {
                center_x: 0.7,
                center_y: 0.45,
                width_frac: 0.35,
                height_frac: 0.9,
            },
        ];
        let filter = compute_crop_filter(&f, 1920, 1080);
        assert!(filter.contains("vstack=2"), "{filter}");
        assert!(filter.contains("split=2"), "{filter}");
    }

    #[test]
    fn blur_and_fit_modes_build() {
        let mut f = ClipFraming::default();
        f.mode = FramingMode::BlurredBackground;
        let b = compute_crop_filter(&f, 1280, 720);
        assert!(b.contains("boxblur") || b.contains("overlay"));
        f.mode = FramingMode::FitWithBars;
        let p = compute_crop_filter(&f, 1280, 720);
        assert!(p.contains("pad="));
    }

    #[test]
    fn manual_center_stays_in_frame() {
        let mut f = ClipFraming::default();
        f.mode = FramingMode::Manual;
        f.center_x = 0.9;
        f.center_y = 0.1;
        let filter = compute_crop_filter(&f, 1920, 1080);
        assert!(filter.contains("crop="));
        assert!(!filter.contains("crop=-"));
    }
}
