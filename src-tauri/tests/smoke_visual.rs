//! Smoke: synthetic cut video + library image → VisualPlan overlay render (FFmpeg).
//!
//! ```text
//! cargo test --test smoke_visual -- --nocapture
//! ```

mod common;

use std::path::Path;

use image::{Rgb, RgbImage};
use vigilcut_lib::models::visual::{
    PlacementMode, VisualPlacement, VisualPlan,
};
use vigilcut_lib::pipeline::visual::library::{
    get_asset_by_id, import_image, list_usage, set_library_root_override,
};
use vigilcut_lib::pipeline::visual::render::render_visual_plan;
use vigilcut_lib::models::visual::LicenseStatus;

fn write_png(path: &Path, color: [u8; 3]) {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p).ok();
    }
    let img = RgbImage::from_fn(160, 120, |_, _| Rgb(color));
    img.save(path).expect("write png");
}

#[tokio::test]
async fn smoke_visual_overlay_render_atomic_and_manifest() {
    let ws = common::test_workspace("smoke_visual_overlay");
    let lib = ws.join("library");
    set_library_root_override(Some(lib));

    let cut = ws.join("cut.mp4");
    common::make_talking_head_fixture(&cut);

    let png = ws.join("src-image.png");
    write_png(&png, [200, 40, 40]);
    // Keep a copy of original path to assert it still exists after import
    let orig_meta = std::fs::metadata(&png).unwrap().len();

    let asset = import_image(
        &png,
        Some("smoke-red".into()),
        vec!["test".into()],
        vec!["smoke".into()],
        LicenseStatus::Owned,
    )
    .expect("import image");
    assert!(Path::new(&asset.managed_path).is_file());
    assert_eq!(std::fs::metadata(&png).unwrap().len(), orig_meta, "original must stay intact");

    let mut plan = VisualPlan::new("smoke-run", cut.to_string_lossy(), "fp-smoke");
    plan.placements.push(VisualPlacement::manual(
        asset.id.clone(),
        0.5,
        1.5,
        PlacementMode::Fullframe,
        Default::default(),
        "cover",
        Some("smoke".into()),
    ));

    let out = ws.join("visual-enriched.mp4");
    let rendered = render_visual_plan(&cut, &plan, &out, "smoke-media.mp4")
        .await
        .expect("visual render");

    assert!(
        rendered.is_file(),
        "render output missing: {}",
        rendered.display()
    );
    let size = std::fs::metadata(&rendered).unwrap().len();
    assert!(size > 2000, "render too small: {size} bytes");

    let manifest = rendered.with_extension("visual-manifest.json");
    assert!(
        manifest.is_file(),
        "manifest missing next to output: {}",
        manifest.display()
    );
    let man = std::fs::read_to_string(&manifest).unwrap();
    assert!(man.contains("visual_render_manifest"));
    assert!(man.contains(&asset.id));

    let plan_json = rendered.with_extension("visual-plan.json");
    assert!(plan_json.is_file(), "plan JSON missing beside output");

    // Usage only after success
    let usage = list_usage(Some(&asset.id), 10).expect("usage");
    assert_eq!(usage.len(), 1, "expected one usage row");
    let a2 = get_asset_by_id(&asset.id).unwrap();
    assert_eq!(a2.times_used, 1);

    // Original source image still present
    assert!(png.is_file());

    set_library_root_override(None);
    println!(
        "smoke visual OK out={} ({} bytes) asset={}",
        rendered.display(),
        size,
        asset.id
    );
}

#[tokio::test]
async fn smoke_visual_refuses_empty_plan() {
    let ws = common::test_workspace("smoke_visual_empty");
    let cut = ws.join("cut.mp4");
    common::make_talking_head_fixture(&cut);
    let plan = VisualPlan::new("r", cut.to_string_lossy(), "fp");
    let err = render_visual_plan(&cut, &plan, &ws.join("out.mp4"), "m")
        .await
        .expect_err("empty plan must fail");
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("placement") || msg.to_lowercase().contains("visual"),
        "unexpected error: {msg}"
    );
    println!("smoke visual empty-plan refusal OK: {msg}");
}
