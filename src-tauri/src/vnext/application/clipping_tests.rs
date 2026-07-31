//! Phase 3: durable clipping + append-only review decisions.

use crate::models::clipping::{
    ClipCandidate, ClipFraming, ClipReviewStatus, ClipScoreBreakdown, ClippingOptions, ClippingRun,
    ClippingSummary, TranscriptSourceKind,
};
use crate::vnext::application::{
    apply_framing_decision, apply_span_decision, apply_status_decision, decision_count,
    ensure_project_for_media, list_review_history, load_run, persist_clipping_run,
    project_next_action,
};
use crate::vnext::domain::NextAction;
use crate::vnext::persistence::{
    delete_project, get_project, list_candidates_for_project, list_projects, open_vnext_db,
    set_vnext_root_override, vnext_test_lock,
};
use std::path::PathBuf;
use std::sync::MutexGuard;

struct Env {
    _g: MutexGuard<'static, ()>,
    dir: PathBuf,
}

fn setup(label: &str) -> Env {
    let g = vnext_test_lock();
    let dir =
        std::env::temp_dir().join(format!("vc-vnext-clip-{}-{}", label, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    set_vnext_root_override(Some(dir.clone()));
    let _ = open_vnext_db().unwrap();
    Env { _g: g, dir }
}

fn teardown(env: Env) {
    set_vnext_root_override(None);
    let _ = std::fs::remove_dir_all(env.dir);
}

fn sample_candidate(id: &str, media: &str) -> ClipCandidate {
    ClipCandidate {
        id: id.into(),
        analysis_run_id: "ar1".into(),
        source_media_path: media.into(),
        start: 1.0,
        end: 12.0,
        duration: 11.0,
        original_start: 1.0,
        original_end: 12.0,
        transcript: "hola mundo".into(),
        title: "Clip".into(),
        summary: "sum".into(),
        score: 72.0,
        confidence: 0.8,
        breakdown: ClipScoreBreakdown::default(),
        reasons: vec![],
        warnings: vec![],
        strengths: vec![],
        risks: vec![],
        status: ClipReviewStatus::Suggested,
        variant_group_id: "g1".into(),
        is_primary_variant: true,
        framing: ClipFraming::default(),
        export_path: None,
        error: None,
    }
}

fn sample_run(run_id: &str, media: &str, candidates: Vec<ClipCandidate>) -> ClippingRun {
    ClippingRun {
        id: run_id.into(),
        media_path: media.into(),
        source_duration: 120.0,
        options: ClippingOptions::default(),
        candidates,
        summary: ClippingSummary {
            source_duration: 120.0,
            analysis_seconds: 1.0,
            candidates_found: 1,
            preselected: 1,
            high_confidence: 0,
            needs_review: 1,
            discarded: 0,
            best_score: 72.0,
            selected_total_duration: 11.0,
            transcript_source: TranscriptSourceKind::AnalysisSpeechFallback,
            warnings: vec![],
        },
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

#[test]
fn candidates_survive_simulated_restart() {
    let env = setup("surv");
    let media = r"C:\videos\client.mp4";
    let project = ensure_project_for_media(media).unwrap();
    let run_id = uuid::Uuid::new_v4().to_string();
    let c = sample_candidate("cand-1", media);
    let run = sample_run(&run_id, media, vec![c]);
    persist_clipping_run(&run, &project.id).unwrap();

    // "Restart": clear override is not needed; just reload via new connection.
    let loaded = load_run(&run_id).unwrap().expect("run must load");
    assert_eq!(loaded.candidates.len(), 1);
    assert_eq!(loaded.candidates[0].id, "cand-1");
    assert!((loaded.candidates[0].start - 1.0).abs() < 1e-6);
    assert_eq!(loaded.candidates[0].framing.output_width, 1080);

    teardown(env);
}

#[test]
fn approve_and_span_survive_reload() {
    let env = setup("appr");
    let media = r"C:\videos\a.mp4";
    let project = ensure_project_for_media(media).unwrap();
    let run_id = uuid::Uuid::new_v4().to_string();
    let c = sample_candidate("cand-2", media);
    persist_clipping_run(&sample_run(&run_id, media, vec![c]), &project.id).unwrap();

    apply_status_decision(&run_id, "cand-2", ClipReviewStatus::Approved, None).unwrap();
    apply_span_decision(&run_id, "cand-2", 2.5, 15.0).unwrap();
    let mut framing = ClipFraming::default();
    framing.center_x = 0.4;
    apply_framing_decision(&run_id, "cand-2", framing).unwrap();

    let loaded = load_run(&run_id).unwrap().unwrap();
    let c = &loaded.candidates[0];
    // Last decision set framing + modified status
    assert!((c.start - 2.5).abs() < 1e-6);
    assert!((c.end - 15.0).abs() < 1e-6);
    assert!((c.framing.center_x - 0.4).abs() < 1e-6);

    teardown(env);
}

#[test]
fn reject_is_append_only_second_decision_keeps_history() {
    let env = setup("hist");
    let media = r"C:\videos\b.mp4";
    let project = ensure_project_for_media(media).unwrap();
    let run_id = uuid::Uuid::new_v4().to_string();
    persist_clipping_run(
        &sample_run(&run_id, media, vec![sample_candidate("cand-3", media)]),
        &project.id,
    )
    .unwrap();

    apply_status_decision(
        &run_id,
        "cand-3",
        ClipReviewStatus::Rejected,
        Some("malo".into()),
    )
    .unwrap();
    apply_status_decision(&run_id, "cand-3", ClipReviewStatus::Approved, None).unwrap();

    let n = decision_count("cand-3").unwrap();
    assert!(n >= 2, "expected >=2 decisions, got {n}");
    let hist = list_review_history("cand-3").unwrap();
    assert_eq!(hist[0].decision, "reject");
    assert_eq!(hist[0].reason.as_deref(), Some("malo"));
    assert_eq!(hist[1].decision, "approve");
    // First row not overwritten
    assert_ne!(hist[0].id, hist[1].id);

    teardown(env);
}

#[test]
fn next_action_after_candidates_is_review() {
    let env = setup("next");
    let media = r"C:\videos\c.mp4";
    let project = ensure_project_for_media(media).unwrap();
    let run_id = uuid::Uuid::new_v4().to_string();
    persist_clipping_run(
        &sample_run(&run_id, media, vec![sample_candidate("cand-4", media)]),
        &project.id,
    )
    .unwrap();
    let action = project_next_action(&project.id).unwrap();
    assert_eq!(action, NextAction::Review);

    apply_status_decision(&run_id, "cand-4", ClipReviewStatus::Approved, None).unwrap();
    let action2 = project_next_action(&project.id).unwrap();
    assert_eq!(action2, NextAction::Render);

    teardown(env);
}

#[test]
fn delete_project_removes_history_and_candidates() {
    let env = setup("del");
    let media = r"C:\videos\history-del.mp4";
    let project = ensure_project_for_media(media).unwrap();
    let pid = project.id.clone();
    let run_id = uuid::Uuid::new_v4().to_string();
    persist_clipping_run(
        &sample_run(
            &run_id,
            media,
            vec![sample_candidate("cand-del", media)],
        ),
        &pid,
    )
    .unwrap();
    assert_eq!(list_candidates_for_project(&pid).unwrap().len(), 1);
    assert!(list_projects(20)
        .unwrap()
        .iter()
        .any(|p| p.id == pid));

    delete_project(&pid).unwrap();

    assert!(get_project(&pid).unwrap().is_none());
    assert!(list_candidates_for_project(&pid).unwrap().is_empty());
    assert!(!list_projects(20).unwrap().iter().any(|p| p.id == pid));

    teardown(env);
}
