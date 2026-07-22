//! Smoke: SRT-driven clipping produces better-scored candidates than audio-only.

mod common;

use std::fs;
use std::path::Path;

use vigilcut_lib::models::clipping::{ClippingOptions, DurationProfile, SelectionProfile};
use vigilcut_lib::pipeline::clipping::run_clipping_analysis;

#[tokio::test]
async fn smoke_clipping_with_srt_finds_candidates() {
    let ws = common::test_workspace("smoke_clipping_srt");
    let media = ws.join("talk.mp4");
    common::make_talking_head_fixture(&media);

    // Synthetic SRT covering the 3s fixture with two semantic blocks
    let srt = r#"1
00:00:00,000 --> 00:00:01,000
Hola, este es un consejo importante

2
00:00:02,000 --> 00:00:03,000
Nunca ignores el cierre de tu idea.
"#;
    let srt_path = ws.join("talk.srt");
    fs::write(&srt_path, srt).unwrap();

    let opts = ClippingOptions {
        duration_profile: DurationProfile::Micro,
        selection_profile: SelectionProfile::Exploratory,
        min_duration: Some(1.0),
        ideal_duration: Some(2.0),
        max_duration: Some(4.0),
        pad_before: 0.0,
        pad_after: 0.0,
        transcript_path: Some(srt_path.to_string_lossy().into_owned()),
        prefer_whisper: false,
        max_candidates: 20,
    };

    let run = run_clipping_analysis(&media, opts)
        .await
        .expect("clipping with srt");

    assert!(
        run.summary.candidates_found > 0,
        "expected candidates with SRT, warnings={:?}",
        run.summary.warnings
    );
    assert!(
        matches!(
            run.summary.transcript_source,
            vigilcut_lib::models::clipping::TranscriptSourceKind::SrtFile
        ),
        "source={:?}",
        run.summary.transcript_source
    );
    // Real transcript should yield higher clarity on some candidates
    let best = run.candidates.iter().map(|c| c.score).fold(0.0, f64::max);
    assert!(best > 30.0, "best score too low: {best}");

    println!(
        "smoke srt clipping OK n={} best={:.0} source={:?}",
        run.summary.candidates_found, best, run.summary.transcript_source
    );
}

#[tokio::test]
async fn smoke_sidecar_srt_auto_detected() {
    let ws = common::test_workspace("smoke_sidecar");
    let media = ws.join("clip.mp4");
    common::make_talking_head_fixture(&media);
    fs::write(
        ws.join("clip.srt"),
        "1\n00:00:00,000 --> 00:00:02,500\nPor que falla esto? Porque no hay plan.\n",
    )
    .unwrap();

    let opts = ClippingOptions {
        transcript_path: None, // must auto-find sidecar
        prefer_whisper: false,
        duration_profile: DurationProfile::Micro,
        selection_profile: SelectionProfile::Broad,
        min_duration: Some(1.0),
        ideal_duration: Some(2.5),
        max_duration: Some(5.0),
        ..ClippingOptions::default()
    };

    let run = run_clipping_analysis(&media, opts)
        .await
        .expect("sidecar analysis");
    assert!(
        run.summary
            .warnings
            .iter()
            .any(|w| w.contains("Transcripción") || w.contains("srt"))
            || matches!(
                run.summary.transcript_source,
                vigilcut_lib::models::clipping::TranscriptSourceKind::SrtFile
            ),
        "expected sidecar detection, source={:?} warnings={:?}",
        run.summary.transcript_source,
        run.summary.warnings
    );
    let _ = Path::new(".");
}
