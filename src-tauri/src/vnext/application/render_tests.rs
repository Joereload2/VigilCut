//! Phase 4: render plan immutability, path safety, complete-with-validated artifact.

use crate::models::clipping::{
    ClipCandidate, ClipFraming, ClipReviewStatus, ClipScoreBreakdown, ClippingOptions, ClippingRun,
    ClippingSummary, TranscriptSourceKind,
};
use crate::pipeline::safe_paths::{finalize_atomic, temp_export_path, validate_export_request};
use crate::vnext::application::{
    complete_with_fixture_file, create_plan_for_candidate_id, enqueue_vertical_render,
    ensure_project_for_media, persist_clipping_run,
};
use crate::vnext::domain::{
    assert_distinct_paths, SubtitleCueV1, SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1,
};
use crate::vnext::persistence::{
    get_job, get_render_plan, open_vnext_db, set_vnext_root_override, vnext_test_lock,
};
use std::path::PathBuf;
use std::sync::MutexGuard;

struct Env {
    _g: MutexGuard<'static, ()>,
    dir: PathBuf,
}

fn setup(label: &str) -> Env {
    let g = vnext_test_lock();
    let dir = std::env::temp_dir().join(format!("vc-vnext-r-{}-{}", label, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    set_vnext_root_override(Some(dir.clone()));
    let _ = open_vnext_db().unwrap();
    Env { _g: g, dir }
}

fn teardown(env: Env) {
    set_vnext_root_override(None);
    let _ = std::fs::remove_dir_all(env.dir);
}

fn seed_run(media: &str) -> (String, String, String) {
    let project = ensure_project_for_media(media).unwrap();
    let run_id = uuid::Uuid::new_v4().to_string();
    let cand_id = uuid::Uuid::new_v4().to_string();
    let c = ClipCandidate {
        id: cand_id.clone(),
        analysis_run_id: "ar".into(),
        source_media_path: media.into(),
        start: 0.5,
        end: 3.5,
        duration: 3.0,
        original_start: 0.5,
        original_end: 3.5,
        transcript: "hola".into(),
        title: "T".into(),
        summary: "s".into(),
        score: 80.0,
        confidence: 0.9,
        breakdown: ClipScoreBreakdown::default(),
        reasons: vec![],
        warnings: vec![],
        strengths: vec![],
        risks: vec![],
        status: ClipReviewStatus::Approved,
        variant_group_id: "g".into(),
        is_primary_variant: true,
        framing: ClipFraming::default(),
        export_path: None,
        error: None,
    };
    let run = ClippingRun {
        id: run_id.clone(),
        media_path: media.into(),
        source_duration: 30.0,
        options: ClippingOptions::default(),
        candidates: vec![c],
        summary: ClippingSummary {
            source_duration: 30.0,
            analysis_seconds: 0.1,
            candidates_found: 1,
            preselected: 1,
            high_confidence: 1,
            needs_review: 0,
            discarded: 0,
            best_score: 80.0,
            selected_total_duration: 3.0,
            transcript_source: TranscriptSourceKind::AnalysisSpeechFallback,
            warnings: vec![],
        },
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    persist_clipping_run(&run, &project.id).unwrap();
    (project.id, run_id, cand_id)
}

#[test]
fn render_plan_is_insert_only_and_validates_preset() {
    let env = setup("plan");
    let media = r"C:\src\video.mp4";
    let (_pid, run_id, cand) = seed_run(media);
    let plan = create_plan_for_candidate_id(
        &run_id,
        &cand,
        vec![SubtitleCueV1 {
            id: "c1".into(),
            start_s: 0.0,
            end_s: 1.0,
            text: "hola".into(),
        }],
        true,
    )
    .unwrap();
    assert_eq!(plan.output_spec.width, 1080);
    assert_eq!(plan.output_spec.height, 1920);
    assert_eq!(
        plan.subtitles.preset_id,
        SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1
    );
    assert!(get_render_plan(&plan.id).unwrap().is_some());
    // Second insert same id must fail (immutability)
    let mut p2 = plan.clone();
    p2.created_at = "other".into();
    assert!(crate::vnext::persistence::insert_render_plan(&p2).is_err());
    teardown(env);
}

#[test]
fn input_output_paths_must_differ() {
    assert!(assert_distinct_paths(r"C:\a\v.mp4", r"C:\a\v.mp4").is_err());
    assert!(assert_distinct_paths(r"C:\a\v.mp4", r"C:\a\out.mp4").is_ok());
    let dir = std::env::temp_dir().join(format!("vc-paths-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("in.mp4");
    std::fs::write(&input, vec![0u8; 2048]).unwrap();
    assert!(validate_export_request(&input, &input).is_err());
    let out = dir.join("out.mp4");
    assert!(validate_export_request(&input, &out).is_ok());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn temp_never_final_and_complete_with_fixture() {
    let env = setup("fix");
    let media = r"C:\src\b.mp4";
    let (pid, run_id, cand) = seed_run(media);
    let plan = create_plan_for_candidate_id(&run_id, &cand, vec![], false).unwrap();
    let job_id = enqueue_vertical_render(&plan.id, &pid).unwrap();

    let final_path = env.dir.join("deliverable.mp4");
    let temp = temp_export_path(&final_path);
    // Write enough bytes for validate_export_output
    std::fs::write(&temp, vec![1u8; 4096]).unwrap();
    assert!(temp
        .file_name()
        .unwrap()
        .to_string_lossy()
        .contains("vigilcut-tmp"));
    finalize_atomic(&temp, &final_path).unwrap();
    assert!(final_path.exists());
    assert!(!temp.exists());

    // Completing with temp name must fail if we still had temp path
    let bad = env.dir.join(".x.vigilcut-tmp-abc.mp4");
    std::fs::write(&bad, vec![1u8; 4096]).unwrap();
    assert!(complete_with_fixture_file(&job_id, &pid, &bad).is_err());

    let art = complete_with_fixture_file(&job_id, &pid, &final_path).unwrap();
    let job = get_job(&job_id).unwrap().unwrap();
    assert_eq!(job.status, "completed");
    assert_eq!(job.result_artifact_id.as_deref(), Some(art.as_str()));
    teardown(env);
}

#[test]
fn idempotent_enqueue_same_plan() {
    let env = setup("idem-r");
    let media = r"C:\src\c.mp4";
    let (pid, run_id, cand) = seed_run(media);
    let plan = create_plan_for_candidate_id(&run_id, &cand, vec![], false).unwrap();
    let a = enqueue_vertical_render(&plan.id, &pid).unwrap();
    let b = enqueue_vertical_render(&plan.id, &pid).unwrap();
    assert_eq!(a, b);
    teardown(env);
}
