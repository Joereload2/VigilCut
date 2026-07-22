//! Shared composition geometry for live preview and FFmpeg bake.
//!
//! # Contract (single source of truth)
//!
//! Placement layout fields mean:
//! - `layout.x`, `layout.y` — **center** of the overlay on the frame, normalized 0..1
//!   (0 = left/top edge of center, 1 = right/bottom).
//! - `layout.w` — width as fraction of frame width (0.05..1).
//! - `layout.h` — height as fraction of frame height; `0` means “derive from width”
//!   (default aspect 16:9 box for layout math; FFmpeg may still preserve image AR).
//! - `fit` — `contain` | `cover` | `crop` (crop ≡ cover for encode).
//!
//! Fullframe mode ignores x/y/w for position (fills the frame).
//!
//! Preview (CSS) and export (FFmpeg) **must** use these helpers so
//! “what you see” matches “what you export”.

use crate::models::visual::{PlacementLayout, PlacementMode, VisualPlacement};
use serde::{Deserialize, Serialize};

/// Canonical working frame for longform bake (matches historical encode).
pub const DEFAULT_FRAME_W: u32 = 1280;
pub const DEFAULT_FRAME_H: u32 = 720;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    Contain,
    Cover,
}

impl FitMode {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "contain" => Self::Contain,
            "cover" | "crop" => Self::Cover,
            _ => Self::Cover,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contain => "contain",
            Self::Cover => "cover",
        }
    }

    /// FFmpeg `force_original_aspect_ratio` value.
    pub fn ffmpeg_force_ar(self) -> &'static str {
        match self {
            Self::Contain => "decrease",
            Self::Cover => "increase",
        }
    }
}

/// Normalized placement geometry (frame-relative).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlacementGeom {
    pub mode_fullframe: bool,
    /// Center X 0..1
    pub cx: f64,
    /// Center Y 0..1
    pub cy: f64,
    /// Box width 0..1 of frame
    pub w: f64,
    /// Box height 0..1 of frame (may be derived)
    pub h: f64,
    pub opacity: f64,
    pub fit_contain: bool,
}

impl PlacementGeom {
    pub fn from_placement(pl: &VisualPlacement) -> Self {
        Self::from_parts(pl.mode, &pl.layout, &pl.fit)
    }

    pub fn from_parts(mode: PlacementMode, layout: &PlacementLayout, fit: &str) -> Self {
        let fit_mode = FitMode::parse(fit);
        let opacity = layout.opacity.clamp(0.05, 1.0);

        match mode {
            PlacementMode::Fullframe => Self {
                mode_fullframe: true,
                cx: 0.5,
                cy: 0.5,
                w: 1.0,
                h: 1.0,
                opacity,
                fit_contain: fit_mode == FitMode::Contain,
            },
            PlacementMode::PictureInPicture => {
                let w = layout.w.clamp(0.08, 1.0);
                let h = if layout.h > 0.01 {
                    layout.h.clamp(0.05, 1.0)
                } else {
                    // Default PIP box ~ 16:9 relative to 1280x720 frame
                    (w * (DEFAULT_FRAME_W as f64) * 9.0 / 16.0 / (DEFAULT_FRAME_H as f64))
                        .clamp(0.05, 1.0)
                };
                Self {
                    mode_fullframe: false,
                    cx: layout.x.clamp(0.0, 1.0),
                    cy: layout.y.clamp(0.0, 1.0),
                    w,
                    h,
                    opacity,
                    fit_contain: fit_mode == FitMode::Contain,
                }
            }
            PlacementMode::LowerThird => {
                let w = layout.w.clamp(0.15, 1.0);
                let h = if layout.h > 0.01 {
                    layout.h.clamp(0.05, 0.5)
                } else {
                    0.22
                };
                Self {
                    mode_fullframe: false,
                    // Historically lower-third was horizontally centered in FFmpeg
                    cx: 0.5,
                    cy: layout.y.clamp(0.0, 1.0),
                    w,
                    h,
                    opacity,
                    fit_contain: fit_mode == FitMode::Contain,
                }
            }
        }
    }

    /// Axis-aligned box in pixels (top-left origin), clamped to frame.
    pub fn box_px(&self, frame_w: u32, frame_h: u32) -> RectPx {
        let fw = frame_w.max(1) as f64;
        let fh = frame_h.max(1) as f64;
        if self.mode_fullframe {
            return RectPx {
                x: 0,
                y: 0,
                w: frame_w as i32,
                h: frame_h as i32,
            };
        }
        let bw = (self.w * fw).round().max(1.0);
        let bh = (self.h * fh).round().max(1.0);
        let mut x = (self.cx * fw - bw / 2.0).round();
        let mut y = (self.cy * fh - bh / 2.0).round();
        // Clamp inside frame
        x = x.clamp(0.0, (fw - bw).max(0.0));
        y = y.clamp(0.0, (fh - bh).max(0.0));
        RectPx {
            x: x as i32,
            y: y as i32,
            w: bw as i32,
            h: bh as i32,
        }
    }

    /// CSS percentages for a box positioned with **top-left** (no translate).
    /// Parent must be the video frame content box.
    pub fn css_top_left_percent(&self) -> CssBoxPercent {
        if self.mode_fullframe {
            return CssBoxPercent {
                left: 0.0,
                top: 0.0,
                width: 100.0,
                height: 100.0,
                opacity: self.opacity,
                object_fit: if self.fit_contain {
                    "contain"
                } else {
                    "cover"
                },
                fullframe: true,
            };
        }
        let left = ((self.cx - self.w / 2.0) * 100.0).clamp(0.0, 100.0);
        let top = ((self.cy - self.h / 2.0) * 100.0).clamp(0.0, 100.0);
        let width = (self.w * 100.0).clamp(1.0, 100.0);
        let height = (self.h * 100.0).clamp(1.0, 100.0);
        CssBoxPercent {
            left,
            top,
            width,
            height,
            opacity: self.opacity,
            object_fit: if self.fit_contain {
                "contain"
            } else {
                "cover"
            },
            fullframe: false,
        }
    }

    /// FFmpeg scale filter for input index `idx`, output label `ov{i}`.
    pub fn ffmpeg_scale_filter(&self, idx: usize, i: usize, frame_w: u32, frame_h: u32) -> String {
        let alpha = self.opacity;
        let force_ar = if self.fit_contain {
            "decrease"
        } else {
            "increase"
        };
        if self.mode_fullframe {
            if self.fit_contain {
                return format!(
                    "[{idx}:v]scale={frame_w}:{frame_h}:force_original_aspect_ratio=decrease,pad={frame_w}:{frame_h}:(ow-iw)/2:(oh-ih)/2,format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
                );
            }
            return format!(
                "[{idx}:v]scale={frame_w}:{frame_h}:force_original_aspect_ratio=increase,crop={frame_w}:{frame_h},format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
            );
        }
        let r = self.box_px(frame_w, frame_h);
        let wf = r.w.max(80);
        let hf = r.h.max(60);
        if self.fit_contain {
            // Fit inside box, pad transparent not needed — overlay uses actual w/h
            format!(
                "[{idx}:v]scale={wf}:{hf}:force_original_aspect_ratio={force_ar},format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
            )
        } else {
            format!(
                "[{idx}:v]scale={wf}:{hf}:force_original_aspect_ratio={force_ar},crop={wf}:{hf},format=rgba,colorchannelmixer=aa={alpha:.3}[ov{i}]"
            )
        }
    }

    /// FFmpeg overlay x/y expressions (center-based, matches CSS top-left box).
    pub fn ffmpeg_overlay_xy(&self, frame_w: u32, frame_h: u32) -> (String, String) {
        if self.mode_fullframe {
            return ("(W-w)/2".into(), "(H-h)/2".into());
        }
        let r = self.box_px(frame_w, frame_h);
        // Absolute pixel placement for exact parity with box_px
        (format!("{}", r.x), format!("{}", r.y))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RectPx {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// CSS box in % of parent (video frame).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CssBoxPercent {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    pub object_fit: &'static str,
    pub fullframe: bool,
}

/// Serialize geom for UI / manifest (stable API).
pub fn geom_json(pl: &VisualPlacement) -> serde_json::Value {
    let g = PlacementGeom::from_placement(pl);
    let css = g.css_top_left_percent();
    let px = g.box_px(DEFAULT_FRAME_W, DEFAULT_FRAME_H);
    serde_json::json!({
        "cx": g.cx,
        "cy": g.cy,
        "w": g.w,
        "h": g.h,
        "opacity": g.opacity,
        "fullframe": g.mode_fullframe,
        "fit": if g.fit_contain { "contain" } else { "cover" },
        "css": {
            "leftPct": css.left,
            "topPct": css.top,
            "widthPct": css.width,
            "heightPct": css.height,
            "objectFit": css.object_fit,
        },
        "framePx": {
            "x": px.x,
            "y": px.y,
            "w": px.w,
            "h": px.h,
            "frameW": DEFAULT_FRAME_W,
            "frameH": DEFAULT_FRAME_H,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::visual::{PlacementLayout, PlacementMode, VisualPlacement};

    fn pip_placement(cx: f64, cy: f64, w: f64, h: f64) -> VisualPlacement {
        VisualPlacement::manual(
            "a",
            0.0,
            2.0,
            PlacementMode::PictureInPicture,
            PlacementLayout {
                x: cx,
                y: cy,
                w,
                h,
                opacity: 1.0,
            },
            "cover",
            None,
        )
    }

    #[test]
    fn fullframe_fills_frame() {
        let pl = VisualPlacement::manual(
            "a",
            0.0,
            1.0,
            PlacementMode::Fullframe,
            PlacementLayout::for_mode(PlacementMode::Fullframe),
            "cover",
            None,
        );
        let g = PlacementGeom::from_placement(&pl);
        let r = g.box_px(1280, 720);
        assert_eq!(r, RectPx { x: 0, y: 0, w: 1280, h: 720 });
        let css = g.css_top_left_percent();
        assert!(css.fullframe);
        assert!((css.width - 100.0).abs() < 1e-6);
    }

    #[test]
    fn center_box_parity_css_and_px() {
        // Center at 50%, size 20% x 20% → top-left at 40%
        let pl = pip_placement(0.5, 0.5, 0.2, 0.2);
        let g = PlacementGeom::from_placement(&pl);
        let css = g.css_top_left_percent();
        assert!((css.left - 40.0).abs() < 0.1);
        assert!((css.top - 40.0).abs() < 0.1);
        assert!((css.width - 20.0).abs() < 0.1);

        let r = g.box_px(1000, 1000);
        assert_eq!(r.w, 200);
        assert_eq!(r.h, 200);
        assert_eq!(r.x, 400);
        assert_eq!(r.y, 400);

        // FFmpeg absolute overlay matches px
        let (ox, oy) = g.ffmpeg_overlay_xy(1000, 1000);
        assert_eq!(ox, "400");
        assert_eq!(oy, "400");
    }

    #[test]
    fn css_and_ffmpeg_same_top_left_on_default_frame() {
        let pl = pip_placement(0.82, 0.18, 0.28, 0.0);
        let g = PlacementGeom::from_placement(&pl);
        let css = g.css_top_left_percent();
        let r = g.box_px(DEFAULT_FRAME_W, DEFAULT_FRAME_H);
        // left% * frame_w ≈ r.x
        let css_x = (css.left / 100.0) * DEFAULT_FRAME_W as f64;
        let css_y = (css.top / 100.0) * DEFAULT_FRAME_H as f64;
        assert!((css_x - r.x as f64).abs() < 2.0, "x css={css_x} px={}", r.x);
        assert!((css_y - r.y as f64).abs() < 2.0, "y css={css_y} px={}", r.y);
        let (ox, oy) = g.ffmpeg_overlay_xy(DEFAULT_FRAME_W, DEFAULT_FRAME_H);
        assert_eq!(ox.parse::<i32>().unwrap(), r.x);
        assert_eq!(oy.parse::<i32>().unwrap(), r.y);
    }

    #[test]
    fn lower_third_is_horizontally_centered() {
        let pl = VisualPlacement::manual(
            "a",
            0.0,
            1.0,
            PlacementMode::LowerThird,
            PlacementLayout::for_mode(PlacementMode::LowerThird),
            "contain",
            None,
        );
        let g = PlacementGeom::from_placement(&pl);
        assert!((g.cx - 0.5).abs() < 1e-9);
        assert!(g.fit_contain);
    }

    #[test]
    fn geom_json_stable_keys() {
        let pl = pip_placement(0.5, 0.5, 0.3, 0.2);
        let v = geom_json(&pl);
        assert!(v.get("css").is_some());
        assert!(v.get("framePx").is_some());
        assert_eq!(v["fit"], "cover");
    }
}
