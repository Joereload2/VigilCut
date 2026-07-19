//! E2E clipping: synthetic media → candidates → vertical export.

mod common;

use vigilcut_lib::models::clipping::{
    ClipReviewStatus, ClippingOptions, DurationProfile, SelectionProfile,
};
use vigilcut_lib::pipeline::clipping::{export_approved_clips, run_clipping_analysis};

#[tokio::test]
async fn e2e_clipping_finds_and_exports_vertical() {
    let ws = common::test_workspace("e2e_clipping");
    let media = ws.join("long.mp4");
    common::make_long_talking_fixture(&media);

    let opts = ClippingOptions {
        duration_profile: DurationProfile::Micro,
        selection_profile: SelectionProfile::Broad,
        min_duration: Some(8.0),
        ideal_duration: Some(12.0),
        max_duration: Some(20.0),
        pad_before: 0.1,
        pad_after: 0.1,
        transcript_path: None,
        prefer_whisper: false,
        max_candidates: 20,
    };

    let mut run = run_clipping_analysis(&media, opts)
        .await
        .expect("clipping analysis");

    assert!(
        run.summary.candidates_found > 0,
        "expected candidates, got 0 warnings={:?}",
        run.summary.warnings
    );

    for c in run.candidates.iter_mut() {
        if c.is_primary_variant && c.status == ClipReviewStatus::Preselected {
            c.status = ClipReviewStatus::Approved;
        }
    }
    if !run
        .candidates
        .iter()
        .any(|c| c.status == ClipReviewStatus::Approved)
    {
        if let Some(c) = run.candidates.iter_mut().find(|c| c.is_primary_variant) {
            c.status = ClipReviewStatus::Approved;
        }
    }

    let out_dir = ws.join("out");
    let results = export_approved_clips(
        &media,
        &mut run.candidates,
        &[],
        &out_dir,
        None,
        320,
        240,
    )
    .await
    .expect("export clips");

    assert!(
        results.iter().any(|r| r.ok),
        "no successful clip export: {:?}",
        results
    );
    assert!(out_dir.join("clips").is_dir());
    assert!(out_dir.join("metadata.json").is_file());
    assert!(out_dir.join("clipping-report.json").is_file());

    // At least one exported file non-trivial size
    let mut any_big = false;
    if let Ok(rd) = std::fs::read_dir(out_dir.join("clips")) {
        for e in rd.flatten() {
            if e.path().extension().and_then(|x| x.to_str()) == Some("mp4")
                && e.metadata().map(|m| m.len()).unwrap_or(0) > 2000
            {
                any_big = true;
            }
        }
    }
    assert!(any_big, "exported mp4 files missing or too small");

    println!(
        "e2e clipping OK candidates={} exported={}",
        run.summary.candidates_found,
        results.iter().filter(|r| r.ok).count()
    );
}
