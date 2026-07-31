//! Smoke vNext Shorts operator path (no UI):
//! project → persist candidates → approve → plan → fixture artifact → delete history.

mod common;

use std::fs;

use vigilcut_lib::models::clipping::{
    ClipCandidate, ClipFraming, ClipReviewStatus, ClipScoreBreakdown, ClippingOptions, ClippingRun,
    ClippingSummary, TranscriptSourceKind,
};
use vigilcut_lib::vnext::application::{
    apply_status_decision, complete_with_fixture_file, create_plan_for_candidate_id,
    deliverable_path_for_source, enqueue_vertical_render, ensure_project_for_media,
    persist_clipping_run,
};
use vigilcut_lib::vnext::persistence::{
    delete_project, get_project, list_artifacts_for_project, list_candidates_for_project,
    list_projects, open_vnext_db, set_vnext_root_override, vnext_test_lock,
};

fn sample_candidate(id: &str, media: &str) -> ClipCandidate {
    ClipCandidate {
        id: id.into(),
        analysis_run_id: "ar-smoke".into(),
        source_media_path: media.into(),
        start: 0.5,
        end: 3.0,
        duration: 2.5,
        original_start: 0.5,
        original_end: 3.0,
        transcript: "hola short".into(),
        title: "Hook".into(),
        summary: "sum".into(),
        score: 80.0,
        confidence: 0.9,
        breakdown: ClipScoreBreakdown::default(),
        reasons: vec![],
        warnings: vec![],
        strengths: vec![],
        risks: vec![],
        status: ClipReviewStatus::Preselected,
        variant_group_id: "g".into(),
        is_primary_variant: true,
        framing: ClipFraming::default(),
        export_path: None,
        error: None,
    }
}

#[test]
fn smoke_vnext_persist_approve_artifact_and_delete_history() {
    let _g = vnext_test_lock();
    let ws = common::test_workspace("smoke_vnext_shorts");
    set_vnext_root_override(Some(ws.join("vnext-db")));
    let _ = open_vnext_db().unwrap();

    let media = ws.join("client-video.mp4");
    fs::write(&media, vec![0u8; 4096]).unwrap();
    let media_s = media.to_string_lossy().into_owned();

    // 1) Project + candidates
    let project = ensure_project_for_media(&media_s).unwrap();
    let run_id = uuid::Uuid::new_v4().to_string();
    let cand_id = "cand-smoke-1".to_string();
    let run = ClippingRun {
        id: run_id.clone(),
        media_path: media_s.clone(),
        source_duration: 30.0,
        options: ClippingOptions::default(),
        candidates: vec![sample_candidate(&cand_id, &media_s)],
        summary: ClippingSummary {
            source_duration: 30.0,
            analysis_seconds: 0.1,
            candidates_found: 1,
            preselected: 1,
            high_confidence: 1,
            needs_review: 0,
            discarded: 0,
            best_score: 80.0,
            selected_total_duration: 2.5,
            transcript_source: TranscriptSourceKind::SrtFile,
            warnings: vec![],
        },
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    persist_clipping_run(&run, &project.id).unwrap();
    assert_eq!(list_candidates_for_project(&project.id).unwrap().len(), 1);

    // 2) Operator approve
    apply_status_decision(&run_id, &cand_id, ClipReviewStatus::Approved, None).unwrap();

    // 3) Render plan (no burn-in)
    let plan = create_plan_for_candidate_id(&run_id, &cand_id, vec![], false).unwrap();
    assert!(!plan.subtitles.enabled || plan.subtitles.cues.is_empty());
    let job_id = enqueue_vertical_render(&plan.id, &project.id).unwrap();

    // 4) Deliverable path layout next to source
    let deliv = deliverable_path_for_source(&media_s, &cand_id).expect("deliverable path");
    let deliv_s = deliv.to_string_lossy().replace('\\', "/");
    assert!(
        deliv_s.contains("/client-video/shorts/"),
        "path={deliv_s}"
    );

    // 5) Complete with fixture bytes (no FFmpeg required for this smoke)
    let final_path = ws.join("client-video").join("shorts").join("out.mp4");
    fs::create_dir_all(final_path.parent().unwrap()).unwrap();
    // minimal "valid" size for complete_with_fixture if it checks size
    fs::write(&final_path, vec![0u8; 64 * 1024]).unwrap();
    // write enough for validate_export_output if used
    let art_id =
        complete_with_fixture_file(&job_id, &project.id, &final_path).expect("fixture complete");
    assert!(!art_id.is_empty());
    let arts = list_artifacts_for_project(&project.id, 10).unwrap();
    assert!(
        arts.iter().any(|a| a.kind == "vertical_mp4" || a.role == "final_deliverable"),
        "arts={arts:?}"
    );

    // 6) Delete history entry
    let pid = project.id.clone();
    delete_project(&pid).unwrap();
    assert!(get_project(&pid).unwrap().is_none());
    assert!(!list_projects(50).unwrap().iter().any(|p| p.id == pid));
    // Source file still on disk
    assert!(media.is_file());

    set_vnext_root_override(None);
    println!("smoke_vnext_shorts OK job={job_id} art={art_id}");
}
