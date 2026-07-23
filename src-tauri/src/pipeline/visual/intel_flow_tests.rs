//! Integration: need → match/reuse → generate mock → QA → asset → placement-ready.

#[cfg(test)]
mod tests {
    use crate::models::visual::LicenseStatus;
    use crate::models::visual_intel::{NeedCoverage, VisualNeed};
    use crate::pipeline::visual::generation::worker::{
        cover_project_needs, queue_generation_for_need, worker_tick,
    };
    use crate::pipeline::visual::intelligent_match::{apply_best_match, MatchOptions};
    use crate::pipeline::visual::library::{import_image, set_library_root_override};
    use crate::pipeline::visual::needs::{list_needs, save_needs};
    use image::{Rgb, RgbImage};

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn search_before_generate_and_mock_pipeline() {
        // Exclusive library root for the whole flow (other tests also use override).
        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-intel-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));
        std::env::set_var("VIGILCUT_IMAGE_PROVIDER", "mock");
        std::env::set_var("VIGILCUT_REQUIRE_HUMAN_QA", "0");
        std::env::remove_var("OMNIROUTE_BASE_URL");

        let png = dir.join("seed.png");
        let mut img = RgbImage::new(640, 360);
        for y in 0..360 {
            for x in 0..640 {
                img.put_pixel(x, y, Rgb([(x % 200) as u8, 80, (y % 200) as u8]));
            }
        }
        image::DynamicImage::ImageRgb8(img).save(&png).unwrap();
        let asset = import_image(
            &png,
            Some("supermercado precios".into()),
            vec!["supermercado".into()],
            vec!["supermercado".into(), "inflacion".into()],
            LicenseStatus::Owned,
        )
        .unwrap();
        assert!(matches!(
            asset.status,
            crate::models::visual::AssetStatus::Active
        ));

        let mut need = VisualNeed::from_label("proj-test", "supermercado");
        need.terms = vec!["supermercado".into(), "precios".into()];
        need.output_start = Some(1.0);
        need.output_end = Some(5.0);
        save_needs(std::slice::from_ref(&need)).unwrap();

        let ranked =
            crate::pipeline::visual::intelligent_match::match_need(&need, &MatchOptions::default());
        assert!(
            !ranked.is_empty(),
            "expected library match, got none (asset concepts={:?})",
            asset.concepts
        );
        assert!(apply_best_match(&mut need, &MatchOptions::default()));
        assert_eq!(need.coverage, NeedCoverage::Matched);
        assert!(need.matched_asset_id.is_some());
        save_needs(&[need]).unwrap();

        let mut need2 = VisualNeed::from_label("proj-test", "fluujodecaja_xyz_unique");
        need2.terms = vec!["fluujodecaja_xyz_unique".into()];
        need2.output_start = Some(10.0);
        need2.output_end = Some(14.0);
        save_needs(std::slice::from_ref(&need2)).unwrap();
        assert!(!apply_best_match(&mut need2, &MatchOptions::default()));
        let job = queue_generation_for_need(&mut need2, false).unwrap();
        assert!(job.is_some());
        let n = worker_tick(2).await.unwrap();
        assert!(n >= 1);

        let needs = list_needs("proj-test").unwrap();
        assert!(
            needs.iter().any(|n| matches!(
                n.coverage,
                NeedCoverage::Covered | NeedCoverage::NeedsReview | NeedCoverage::Matched
            )),
            "coverages: {:?}",
            needs
                .iter()
                .map(|n| (n.label.clone(), n.coverage.as_str()))
                .collect::<Vec<_>>()
        );

        let summary = cover_project_needs("proj-test", false, 0).await.unwrap();
        assert!(summary.get("coverage").is_some());

        set_library_root_override(None);
        std::env::remove_var("VIGILCUT_IMAGE_PROVIDER");
        std::env::remove_var("VIGILCUT_REQUIRE_HUMAN_QA");
        let _ = std::fs::remove_dir_all(dir);
    }

    /// PM-003: one use creates exactly one placement; second call replaces.
    #[test]
    #[allow(clippy::await_holding_lock)]
    fn use_asset_for_need_single_placement() {
        use crate::models::edl::Edl;
        use crate::pipeline::visual::{use_asset_for_need, VisualSession, VisualState};
        use std::sync::Mutex;

        let _lock = crate::pipeline::visual::library::lock_library_for_test();
        let dir = std::env::temp_dir().join(format!("vc-use-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        set_library_root_override(Some(dir.clone()));

        let png = dir.join("a.png");
        let mut img = RgbImage::new(320, 180);
        for p in img.pixels_mut() {
            *p = Rgb([40, 90, 140]);
        }
        image::DynamicImage::ImageRgb8(img).save(&png).unwrap();
        let asset = import_image(
            &png,
            Some("escena test".into()),
            vec![],
            vec!["escena".into()],
            LicenseStatus::Owned,
        )
        .unwrap();

        let mut need = VisualNeed::from_label("proj-use", "escena test");
        need.terms = vec!["escena".into()];
        need.output_start = Some(2.0);
        need.output_end = Some(6.0);
        save_needs(std::slice::from_ref(&need)).unwrap();

        let media = dir.join("v.mp4");
        let _ = std::fs::write(&media, b"fake");
        let edl = Edl::from_remove_spans(media.to_string_lossy().as_ref(), 30.0, &[]);
        let state: VisualState = Mutex::new(VisualSession::default());

        let r1 = use_asset_for_need(&state, &edl, &media, &need.id, &asset.id).unwrap();
        let plan1 = r1.get("plan").unwrap();
        let n1 = plan1
            .get("placements")
            .and_then(|p| p.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert_eq!(n1, 1, "first use must create one placement");

        let r2 = use_asset_for_need(&state, &edl, &media, &need.id, &asset.id).unwrap();
        let plan2 = r2.get("plan").unwrap();
        let n2 = plan2
            .get("placements")
            .and_then(|p| p.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        assert_eq!(n2, 1, "second use must replace, not duplicate");

        set_library_root_override(None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
