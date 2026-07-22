//! Smoke tests — real FFmpeg + analysis engine (no GUI).
//!
//! ```text
//! cargo test --test smoke_pipeline -- --nocapture
//! ```

mod common;

use vigilcut_lib::models::edl::PolicyConfig;
use vigilcut_lib::models::event::{TYPE_AUDIO_SILENCE, TYPE_AUDIO_SPEECH};
use vigilcut_lib::pipeline::engine::run_silence_analysis;
use vigilcut_lib::pipeline::export::{keep_ranges_from_edl, keep_ranges_from_segments};

#[tokio::test]
async fn smoke_analyze_synthetic_video_produces_edl_and_segments() {
    let ws = common::test_workspace("smoke_analyze");
    let media = ws.join("talk.mp4");
    common::make_talking_head_fixture(&media);

    let policy = PolicyConfig {
        auto_approve_min_score: 0.75,
        min_silence_duration: 0.3,
        padding: 0.05,
        threshold: 0.5,
        prefer_silero: false, // force ffmpeg path — deterministic in CI without ONNX quirks
        prefer_whisper: false,
    };

    let run = run_silence_analysis(&media, &policy)
        .await
        .expect("analysis should succeed");

    assert!(
        run.duration > 2.5 && run.duration < 3.5,
        "duration unexpected: {}",
        run.duration
    );
    assert!(
        !run.events.is_empty(),
        "expected speech/silence events"
    );
    assert!(
        run.events
            .iter()
            .any(|e| e.event_type == TYPE_AUDIO_SPEECH),
        "expected speech events"
    );
    assert!(
        run.events
            .iter()
            .any(|e| e.event_type == TYPE_AUDIO_SILENCE),
        "expected silence in the middle of the fixture"
    );
    assert!(!run.edl.video_track.is_empty(), "EDL keep track empty");
    assert!(
        run.edl.output_duration > 0.0 && run.edl.output_duration <= run.duration + 0.05,
        "output_duration={} source={}",
        run.edl.output_duration,
        run.duration
    );
    assert!(!run.segments.is_empty(), "segment projection empty");

    let from_edl = keep_ranges_from_edl(&run.edl);
    let from_segs = keep_ranges_from_segments(&run.segments);
    assert!(
        !from_edl.is_empty(),
        "EDL keep ranges empty after merge"
    );
    // Segments may keep pending silences (conservative) so segment keep >= edl keep-ish
    let edl_dur: f64 = from_edl.iter().map(|(s, e)| e - s).sum();
    let seg_dur: f64 = from_segs.iter().map(|(s, e)| e - s).sum();
    assert!(
        edl_dur > 0.5,
        "EDL keep duration too small: {edl_dur}"
    );
    assert!(
        seg_dur + 0.01 >= edl_dur - 0.5,
        "segment keep ({seg_dur}) far below EDL keep ({edl_dur})"
    );

    println!(
        "smoke analyze OK method={} events={} auto_cuts={} exceptions={} {:.2}s → {:.2}s",
        run.method,
        run.events.len(),
        run.stats.auto_cut_count,
        run.stats.pending_exception_count,
        run.duration,
        run.edl.output_duration
    );
}

#[tokio::test]
async fn smoke_policy_auto_cut_removes_mid_silence() {
    let ws = common::test_workspace("smoke_autocut");
    let media = ws.join("talk.mp4");
    common::make_talking_head_fixture(&media);

    // Very low threshold → almost all silences auto-cut
    let aggressive = PolicyConfig {
        auto_approve_min_score: 0.5,
        min_silence_duration: 0.25,
        padding: 0.02,
        threshold: 0.5,
        prefer_silero: false,
        prefer_whisper: false,
    };
    let run = run_silence_analysis(&media, &aggressive)
        .await
        .expect("analysis");

    assert!(
        run.stats.auto_cut_count > 0 || run.edl.removed_duration > 0.2,
        "expected auto-cuts on synthetic mid-silence; auto={} removed={:.2}",
        run.stats.auto_cut_count,
        run.edl.removed_duration
    );
    assert!(
        run.edl.output_duration < run.duration - 0.15,
        "expected shorter output after silence cut"
    );
}
