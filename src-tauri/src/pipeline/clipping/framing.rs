//! Vertical 9:16 framing (static center crop + blur/fit modes).
//! Face tracking is contracted via `tracking_ready` for a future detector.

use crate::models::clipping::{ClipFraming, FramingMode};
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

/// Build FFmpeg video filter for 9:16 output from framing config.
pub fn compute_crop_filter(framing: &ClipFraming, src_w: u32, src_h: u32) -> String {
    let out_w = framing.output_width;
    let out_h = framing.output_height;
    let src_w = src_w.max(2) as f64;
    let src_h = src_h.max(2) as f64;

    match framing.mode {
        FramingMode::BlurredBackground => {
            // Scale full frame to cover 9:16 blur, overlay sharp center
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
            // Crop a 9:16 window then scale
            let target_ar = 9.0 / 16.0;
            let crop_h = src_h / framing.zoom.max(1.0);
            let crop_w = (crop_h * target_ar).min(src_w);
            let crop_h = (crop_w / target_ar).min(src_h);
            let cx = (framing.center_x.clamp(0.05, 0.95) * src_w) - crop_w / 2.0;
            let cy = (framing.center_y.clamp(0.05, 0.95) * src_h) - crop_h / 2.0;
            let x = cx.clamp(0.0, (src_w - crop_w).max(0.0));
            let y = cy.clamp(0.0, (src_h - crop_h).max(0.0));
            format!(
                "crop={crop_w:.0}:{crop_h:.0}:{x:.0}:{y:.0},scale={out_w}:{out_h}"
            )
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
}
