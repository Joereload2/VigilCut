//! End-to-end factory path (no GUI): analyze → EDL export → artifacts.
//!
//! ```text
//! cargo test --test e2e_factory -- --nocapture
//! ```

mod common;

use std::path::Path;

use vigilcut_lib::models::edl::PolicyConfig;
use vigilcut_lib::models::preset::{ColorOptions, ExportOptions};
use vigilcut_lib::pipeline::artifacts::write_run_artifacts;
use vigilcut_lib::pipeline::engine::{accept_all_exceptions, run_silence_analysis};
use vigilcut_lib::pipeline::export::export_from_edl;

#[tokio::test]
async fn e2e_export_mp4_and_meta_folder_layout() {
    let ws = common::test_workspace("e2e_export");
    let media = ws.join("source.mp4");
    common::make_talking_head_fixture(&media);

    let policy = PolicyConfig {
        auto_approve_min_score: 0.6,
        min_silence_duration: 0.3,
        padding: 0.05,
        threshold: 0.5,
        prefer_silero: false,
    };

    let mut run = run_silence_analysis(&media, &policy)
        .await
        .expect("analyze");
    // Factory batch behavior: force remaining exceptions
    if run.stats.pending_exception_count > 0 {
        run = accept_all_exceptions(run);
    }

    let out_mp4 = ws.join("source-editado.mp4");
    let export_opts = ExportOptions {
        reencode: true,
        apply_cuts: true,
        crf: 28,
        preset: "ultrafast".into(),
        ..ExportOptions::default()
    };
    let color = ColorOptions::default();

    export_from_edl(
        &media,
        &out_mp4,
        &run.edl,
        &export_opts,
        &color,
        true,
    )
    .await
    .expect("export_from_edl");

    assert!(out_mp4.is_file(), "missing output mp4");
    assert!(
        out_mp4.metadata().unwrap().len() > 2000,
        "output mp4 too small"
    );

    let artifacts = write_run_artifacts(
        &run,
        &out_mp4,
        &media,
        false, // skip short clips for speed
        serde_json::json!({ "e2e": true }),
    )
    .await
    .expect("write artifacts");

    assert!(
        artifacts.iter().any(|a| a.kind == "longform_mp4"),
        "missing longform artifact"
    );

    // Creator-facing files next to MP4
    let chapters_txt = ws.join("source-editado.chapters.txt");
    assert!(
        chapters_txt.is_file(),
        "expected chapters.txt next to mp4"
    );

    // Machine meta isolated
    let meta = ws.join("source-editado-meta");
    assert!(meta.is_dir(), "expected *-meta/ folder");
    assert!(meta.join("manifest.json").is_file());
    assert!(meta.join("events.json").is_file());
    assert!(meta.join("edl.json").is_file());
    assert!(meta.join("cutlist.edl").is_file());
    assert!(meta.join("README.txt").is_file());

    // JSON should NOT litter next to the mp4 (factory packaging rule)
    assert!(
        !ws.join("source-editado.events.json").exists(),
        "events.json must live under *-meta/, not next to mp4"
    );
    assert!(
        !ws.join("source-editado.json").exists(),
        "manifest must not sit next to mp4 as bare .json"
    );

    println!(
        "e2e OK mp4={} bytes artifacts={} meta={}",
        out_mp4.metadata().unwrap().len(),
        artifacts.len(),
        meta.display()
    );
}

#[tokio::test]
async fn e2e_batch_process_one_file() {
    use vigilcut_lib::pipeline::batch_worker::process_one_file;

    let ws = common::test_workspace("e2e_batch");
    let media = ws.join("clip.mp4");
    common::make_talking_head_fixture(&media);
    let outbox = ws.join("outbox");
    std::fs::create_dir_all(&outbox).unwrap();

    let result = process_one_file(
        &media,
        &outbox,
        &PolicyConfig {
            prefer_silero: false,
            auto_approve_min_score: 0.6,
            min_silence_duration: 0.3,
            ..PolicyConfig::default()
        },
        vigilcut_lib::models::exception_mode::ExceptionHandlingMode::Safe,
        &ExportOptions {
            crf: 28,
            preset: "ultrafast".into(),
            ..ExportOptions::default()
        },
        &ColorOptions::default(),
    )
    .await;

    assert!(result.ok, "batch file failed: {:?}", result.error);
    let out = result.output_path.expect("output path");
    assert!(Path::new(&out).is_file());
    assert!(result.output_duration > 0.0);
    println!(
        "e2e batch OK auto_cuts={} forced={} → {}",
        result.auto_cuts, result.exceptions_forced, out
    );
}
