//! Phase 2 integration tests for durable jobs.

use crate::vnext::application::{
    cancel_job, claim, complete_job_with_artifact, enqueue, fail_job,
    insert_passed_artifact_and_complete, recover_interrupted, retry_job,
};
use crate::vnext::domain::{
    ArtifactValidationStatus, ContentProjectRecord, JobKind, JobStatus, ProductionRecipeRecord,
};
use crate::vnext::persistence::{
    get_artifact, get_job, insert_artifact, insert_project, insert_recipe, list_parent_ids,
    open_vnext_db, set_vnext_root_override, vnext_test_lock, InsertArtifact,
};
use std::path::PathBuf;
use std::sync::{Arc, Barrier, MutexGuard};
use std::thread;

struct TestEnv {
    _guard: MutexGuard<'static, ()>,
    dir: PathBuf,
}

fn setup(label: &str) -> TestEnv {
    let guard = vnext_test_lock();
    let dir = std::env::temp_dir().join(format!("vc-vnext-app-{}-{}", label, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    set_vnext_root_override(Some(dir.clone()));
    let _ = open_vnext_db().unwrap();
    TestEnv { _guard: guard, dir }
}

fn teardown(env: TestEnv) {
    set_vnext_root_override(None);
    let _ = std::fs::remove_dir_all(env.dir);
}

fn seed_project() -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    insert_project(&ContentProjectRecord::new_local(
        id.clone(),
        "Test".into(),
        r"C:\media\source.mp4".into(),
        r"C:\work\proj".into(),
        now,
    ))
    .unwrap();
    id
}

#[test]
fn project_and_recipe_roundtrip_recipe_insert_only() {
    let env = setup("proj");
    let pid = seed_project();
    let rid = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    insert_recipe(&ProductionRecipeRecord {
        id: rid.clone(),
        content_project_id: pid,
        label: "default".into(),
        recipe_json: r#"{"contractVersion":"v1"}"#.into(),
        supersedes_recipe_id: None,
        created_at: now,
    })
    .unwrap();
    assert!(crate::vnext::persistence::get_recipe(&rid)
        .unwrap()
        .is_some());
    teardown(env);
}

#[test]
fn enqueue_is_idempotent_by_key() {
    let env = setup("idem");
    let pid = seed_project();
    let a = enqueue(&pid, JobKind::IngestProbe, "key-1", "{}", 2).unwrap();
    let b = enqueue(&pid, JobKind::IngestProbe, "key-1", "{}", 2).unwrap();
    assert_eq!(a.id, b.id);
    assert_eq!(a.status, "queued");
    teardown(env);
}

#[test]
fn claim_concurrent_only_one_wins() {
    let env = setup("claim");
    let pid = seed_project();
    enqueue(&pid, JobKind::Transcribe, "only-one", "{}", 2).unwrap();

    let barrier = Arc::new(Barrier::new(2));
    let mut handles = vec![];
    for i in 0..2 {
        let b = barrier.clone();
        let wid = format!("w{i}");
        handles.push(thread::spawn(move || {
            b.wait();
            claim(&wid)
        }));
    }
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let claimed: Vec<_> = results
        .into_iter()
        .filter_map(|r| r.ok().flatten())
        .collect();
    assert_eq!(claimed.len(), 1, "exactly one claim must succeed");
    assert_eq!(claimed[0].status, "running");
    assert_eq!(claimed[0].attempt, 1);
    teardown(env);
}

#[test]
fn complete_requires_passed_artifact() {
    let env = setup("complete");
    let pid = seed_project();
    let job = enqueue(&pid, JobKind::VerticalRender, "render-1", "{}", 2).unwrap();
    claim(DEFAULT_WORKER).unwrap();

    // No artifact
    assert!(complete_job_with_artifact(&job.id, "missing").is_err());

    // Artifact failed validation
    let aid = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    insert_artifact(&InsertArtifact {
        id: aid.clone(),
        content_project_id: pid.clone(),
        kind: "vertical_mp4".into(),
        role: "final_deliverable".into(),
        path: r"C:\out\x.mp4".into(),
        sha256: "abc".into(),
        byte_size: 10,
        mime_type: "video/mp4".into(),
        created_by_job_id: job.id.clone(),
        probe_json: None,
        validation_status: ArtifactValidationStatus::Failed,
        validation_notes: Some("bad".into()),
        parent_ids: vec![],
        created_at: now,
    })
    .unwrap();
    assert!(complete_job_with_artifact(&job.id, &aid).is_err());
    let still = get_job(&job.id).unwrap().unwrap();
    assert_eq!(still.status, "running");

    // Passed works
    let done =
        insert_passed_artifact_and_complete(&job.id, &pid, r"C:\out\ok.mp4", "deadbeef").unwrap();
    assert_eq!(done.status, JobStatus::Completed.as_str());
    assert!(done.result_artifact_id.is_some());
    teardown(env);
}

const DEFAULT_WORKER: &str = "local-vnext";

#[test]
fn retry_cancel_and_lease_recovery() {
    let env = setup("retry");
    let pid = seed_project();
    let job = enqueue(&pid, JobKind::IngestProbe, "retry-key", "{}", 3).unwrap();

    // cancel queued → cancelled
    let c = cancel_job(&job.id).unwrap();
    assert_eq!(c.status, "cancelled");

    let job2 = enqueue(&pid, JobKind::IngestProbe, "retry-key-2", "{}", 3).unwrap();
    let claimed = claim("worker-a").unwrap().unwrap();
    assert_eq!(claimed.id, job2.id);

    fail_job(
        &job2.id,
        r#"{"code":"x","message":"boom","retryable":true,"contractVersion":"v1"}"#,
    )
    .unwrap();
    let retried = retry_job(&job2.id).unwrap();
    assert_eq!(retried.status, "queued");

    // claim and expire lease
    let running = claim("worker-b").unwrap().unwrap();
    // Force expired lease via SQL
    let conn = open_vnext_db().unwrap();
    conn.execute(
        "UPDATE jobs SET lease_expires_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
        rusqlite::params![running.id],
    )
    .unwrap();
    let n = recover_interrupted().unwrap();
    assert!(n >= 1);
    let interrupted = get_job(&running.id).unwrap().unwrap();
    assert_eq!(interrupted.status, "interrupted");
    let again = retry_job(&running.id).unwrap();
    assert_eq!(again.status, "queued");
    teardown(env);
}

#[test]
fn artifact_parents_linked() {
    let env = setup("parents");
    let pid = seed_project();
    let job = enqueue(&pid, JobKind::VerticalRender, "par-1", "{}", 1).unwrap();
    claim("w").unwrap();
    let parent = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    insert_artifact(&InsertArtifact {
        id: parent.clone(),
        content_project_id: pid.clone(),
        kind: "transcript_json".into(),
        role: "sidecar".into(),
        path: r"C:\work\t.json".into(),
        sha256: "p1".into(),
        byte_size: 2,
        mime_type: "application/json".into(),
        created_by_job_id: job.id.clone(),
        probe_json: None,
        validation_status: ArtifactValidationStatus::Passed,
        validation_notes: None,
        parent_ids: vec![],
        created_at: now.clone(),
    })
    .unwrap();
    let child = uuid::Uuid::new_v4().to_string();
    insert_artifact(&InsertArtifact {
        id: child.clone(),
        content_project_id: pid,
        kind: "vertical_mp4".into(),
        role: "final_deliverable".into(),
        path: r"C:\work\o.mp4".into(),
        sha256: "c1".into(),
        byte_size: 3,
        mime_type: "video/mp4".into(),
        created_by_job_id: job.id,
        probe_json: None,
        validation_status: ArtifactValidationStatus::Passed,
        validation_notes: None,
        parent_ids: vec![parent.clone()],
        created_at: now,
    })
    .unwrap();
    let parents = list_parent_ids(&child).unwrap();
    assert_eq!(parents, vec![parent]);
    assert!(get_artifact(&child).unwrap().is_some());
    teardown(env);
}
