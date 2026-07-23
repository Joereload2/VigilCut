//! Smoke (no network): need → library match OR mock generate → QA → library asset.
//!
//! ```text
//! cargo test --test smoke_visual_intel -- --nocapture
//! ```

use image::{Rgb, RgbImage};
use vigilcut_lib::models::visual::LicenseStatus;
use vigilcut_lib::models::visual_intel::{NeedCoverage, VisualNeed};
use vigilcut_lib::pipeline::visual::generation::worker::{
    cover_project_needs, queue_generation_for_need, worker_tick,
};
use vigilcut_lib::pipeline::visual::intelligent_match::{apply_best_match, MatchOptions};
use vigilcut_lib::pipeline::visual::library::{
    get_asset_by_id, import_image, list_active_assets, set_library_root_override,
};
use vigilcut_lib::pipeline::visual::needs::{list_needs, save_needs};

#[tokio::test]
async fn smoke_search_before_generate_and_cost_gate() {
    std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
    std::env::remove_var("OMNIROUTE_BASE_URL");
    std::env::remove_var("VIGILCUT_PAID_PROVIDERS");

    let ws = std::env::temp_dir().join(format!("vc-smoke-intel-{}", uuid::Uuid::new_v4()));
    let lib = ws.join("library");
    std::fs::create_dir_all(&lib).unwrap();
    set_library_root_override(Some(lib));

    // Import reusable image
    let png = ws.join("match.png");
    let mut img = RgbImage::new(960, 540);
    for y in 0..540 {
        for x in 0..960 {
            img.put_pixel(x, y, Rgb([(x % 180) as u8, 40, (y % 180) as u8]));
        }
    }
    image::DynamicImage::ImageRgb8(img).save(&png).unwrap();
    let asset = import_image(
        &png,
        Some("inflacion supermercado".into()),
        vec!["inflacion".into()],
        vec!["inflacion".into(), "supermercado".into()],
        LicenseStatus::Owned,
    )
    .expect("import");

    let mut need_hit = VisualNeed::from_label("smoke-proj", "inflacion");
    need_hit.terms = vec!["inflacion".into()];
    need_hit.output_start = Some(0.0);
    need_hit.output_end = Some(4.0);
    save_needs(std::slice::from_ref(&need_hit)).unwrap();
    assert!(
        apply_best_match(&mut need_hit, &MatchOptions::default()),
        "must reuse library before generating"
    );
    assert_eq!(need_hit.coverage, NeedCoverage::Matched);
    assert_eq!(
        need_hit.matched_asset_id.as_deref(),
        Some(asset.id.as_str())
    );
    save_needs(&[need_hit]).unwrap();

    // Uncovered need → mock generation path
    let mut need_miss = VisualNeed::from_label("smoke-proj", "zzz_unique_concept_smoke_xyz");
    need_miss.terms = vec!["zzz_unique_concept_smoke_xyz".into()];
    need_miss.output_start = Some(8.0);
    need_miss.output_end = Some(12.0);
    save_needs(std::slice::from_ref(&need_miss)).unwrap();
    assert!(!apply_best_match(&mut need_miss, &MatchOptions::default()));
    let job = queue_generation_for_need(&mut need_miss, false)
        .expect("enqueue")
        .expect("job id");
    assert!(!job.is_empty());
    let processed = worker_tick(3).await.expect("worker");
    assert!(processed >= 1, "worker should process mock job");

    let needs = list_needs("smoke-proj").unwrap();
    assert!(
        needs.iter().any(|n| matches!(
            n.coverage,
            NeedCoverage::Covered | NeedCoverage::NeedsReview | NeedCoverage::Matched
        )),
        "expected at least one covered/matched need: {:?}",
        needs
            .iter()
            .map(|n| n.coverage.as_str())
            .collect::<Vec<_>>()
    );

    // Cover without generation should not invent paid work
    let out = cover_project_needs("smoke-proj", false, 0)
        .await
        .expect("cover");
    assert!(out.get("coverage").is_some());

    // Library still has active assets
    let active = list_active_assets().unwrap();
    assert!(!active.is_empty());
    let _ = get_asset_by_id(&asset.id).unwrap();

    set_library_root_override(None);
    let _ = std::fs::remove_dir_all(&ws);
}
