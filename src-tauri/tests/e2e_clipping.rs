//! E2E clipping: synthetic media → candidates → vertical export.

mod common;

use std::path::PathBuf;

use vigilcut_lib::models::clipping::{
    ClipReviewStatus, ClippingOptions, DurationProfile, SelectionProfile,
};
use vigilcut_lib::pipeline::clipping::{export_approved_clips, run_clipping_analysis};

#[tokio::test]
async fn e2e_clipping_finds_and_exports_vertical() {
    let ws = common::test_workspace("e2e_clipping");
    // Longer synthetic: 45s with mid silence blocks for speech units
    let media = ws.join("long.mp4");
    make_long_fixture(&media);

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

    // Approve top primary
    for c in run.candidates.iter_mut() {
        if c.is_primary_variant && c.status == ClipReviewStatus::Preselected {
            c.status = ClipReviewStatus::Approved;
        }
    }
    if !run.candidates.iter().any(|c| c.status == ClipReviewStatus::Approved) {
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

    println!(
        "e2e clipping OK candidates={} exported={}",
        run.summary.candidates_found,
        results.iter().filter(|r| r.ok).count()
    );
}

fn make_long_fixture(path: &PathBuf) {
    use std::process::Command;
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    common::ensure_ffmpeg();
    // 30s: speech-like tone with two silence gaps
    let mut cmd = Command::new(common::bundled_ffmpeg());
    cmd.args([
        "-y",
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=320x240:d=30:r=25",
        "-f",
        "lavfi",
        "-i",
        "sine=frequency=660:sample_rate=44100:duration=30",
        "-af",
        "volume=enable='between(t,8,10)+between(t,18,20)':volume=0",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-preset",
        "ultrafast",
        "-c:a",
        "aac",
        "-shortest",
        &path.to_string_lossy(),
    ]);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().expect("ffmpeg");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}
