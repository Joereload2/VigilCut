//! Create immutable render plans and complete vertical_render jobs.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::ffmpeg::Ffmpeg;
use crate::models::clipping::ClipCandidate;
use crate::vnext::domain::{
    ArtifactValidationStatus, JobKind, JobStatus, OutputSpecV1, SubtitleBurnPlanV1, SubtitleCueV1,
    VerticalRenderPlanV1, SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1,
};
use crate::vnext::media::vertical_render::{
    cleanup_job_work_dir, render_vertical_plan, VerticalRenderResult,
};
use crate::vnext::persistence::{
    content_project_id_for_run, get_candidate, get_job, get_render_plan, insert_artifact,
    insert_render_plan, open_vnext_db, set_job_status, InsertArtifact,
};

use super::jobs::{complete_job_with_artifact, enqueue};

/// Build + persist an immutable plan from a short candidate.
pub fn create_vertical_render_plan(
    content_project_id: &str,
    recipe_id: &str,
    candidate: &ClipCandidate,
    cues: Vec<SubtitleCueV1>,
    burn_in: bool,
) -> AppResult<VerticalRenderPlanV1> {
    let now = chrono::Utc::now().to_rfc3339();
    let plan = VerticalRenderPlanV1 {
        contract_version: "v1".into(),
        id: uuid::Uuid::new_v4().to_string(),
        content_project_id: content_project_id.into(),
        recipe_id: recipe_id.into(),
        candidate_id: candidate.id.clone(),
        source_media_path: candidate.source_media_path.clone(),
        source_start_s: candidate.start,
        source_end_s: candidate.end,
        framing: candidate.framing.clone(),
        subtitles: SubtitleBurnPlanV1 {
            enabled: burn_in && !cues.is_empty(),
            preset_id: SUBTITLE_PRESET_SAFE_CENTER_BOTTOM_V1.into(),
            cues,
            source_transcript_artifact_id: None,
        },
        output_spec: OutputSpecV1::default(),
        created_at: now,
        created_by: "local_operator".into(),
    };
    plan.validate()
        .map_err(|e| AppError::Invalid(e.to_string()))?;
    insert_render_plan(&plan)?;
    Ok(plan)
}

pub fn create_plan_for_candidate_id(
    run_id: &str,
    candidate_id: &str,
    cues: Vec<SubtitleCueV1>,
    burn_in: bool,
) -> AppResult<VerticalRenderPlanV1> {
    let project_id = content_project_id_for_run(run_id)?
        .ok_or_else(|| AppError::NotFound(format!("run {run_id}")))?;
    let c = get_candidate(candidate_id)?.ok_or_else(|| AppError::NotFound(candidate_id.into()))?;
    create_vertical_render_plan(&project_id, "default-recipe", &c, cues, burn_in)
}

/// Enqueue a durable vertical_render job for a plan.
pub fn enqueue_vertical_render(plan_id: &str, content_project_id: &str) -> AppResult<String> {
    let key = format!("render:{plan_id}:v1");
    let input = serde_json::json!({ "renderPlanId": plan_id }).to_string();
    let job = enqueue(content_project_id, JobKind::VerticalRender, &key, &input, 2)?;
    let conn = open_vnext_db()?;
    conn.execute(
        "UPDATE jobs SET render_plan_id = ?1 WHERE id = ?2",
        rusqlite::params![plan_id, job.id],
    )
    .map_err(|e| AppError::Message(e.to_string()))?;
    Ok(job.id)
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let data = std::fs::read(path)?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(hex::encode(h.finalize()))
}

/// Sanitize a file stem for use as a folder/file name.
fn safe_stem(raw: &str) -> String {
    let s: String = raw
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let t = s.trim().trim_matches('.');
    if t.is_empty() {
        "video".into()
    } else {
        t.chars().take(80).collect()
    }
}

/// `{parent}/{VideoName}/shorts/{VideoName}_short.mp4` next to the source file.
fn deliverable_path_for_source(source_media_path: &str, candidate_id: &str) -> Option<std::path::PathBuf> {
    use crate::pipeline::safe_paths::unique_output_path;
    let source = Path::new(source_media_path);
    let parent = source.parent()?;
    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .map(safe_stem)
        .unwrap_or_else(|| "video".into());
    let dir = parent.join(&stem).join("shorts");
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    // Short suffix from candidate id for uniqueness among many clips
    let short_tag: String = candidate_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(6)
        .collect();
    let name = if short_tag.is_empty() {
        format!("{stem}_short.mp4")
    } else {
        format!("{stem}_short_{short_tag}.mp4")
    };
    Some(unique_output_path(&dir.join(name)))
}

/// Execute one vertical_render job (must be queued or running).
pub async fn execute_vertical_render_job(job_id: &str) -> AppResult<VerticalRenderResult> {
    let job = get_job(job_id)?.ok_or_else(|| AppError::NotFound(job_id.into()))?;
    let status = JobStatus::parse(&job.status).map_err(|e| AppError::Invalid(e.to_string()))?;
    let now = chrono::Utc::now().to_rfc3339();

    if status == JobStatus::Queued {
        set_job_status(
            job_id,
            JobStatus::Queued,
            JobStatus::Running,
            &now,
            None,
            None,
        )?;
        let conn = open_vnext_db()?;
        let lease = (chrono::Utc::now() + chrono::Duration::seconds(300)).to_rfc3339();
        let _ = conn.execute(
            "UPDATE jobs SET locked_by='vnext-render', lease_expires_at=?1, attempt=attempt+1 WHERE id=?2",
            rusqlite::params![lease, job_id],
        );
    } else if status != JobStatus::Running {
        return Err(AppError::Invalid(format!(
            "job no ejecutable en estado {}",
            job.status
        )));
    }

    let job = get_job(job_id)?.unwrap();
    if job.cancel_requested {
        set_job_status(
            job_id,
            JobStatus::Running,
            JobStatus::Cancelled,
            &chrono::Utc::now().to_rfc3339(),
            Some(
                r#"{"contractVersion":"v1","code":"cancelled","message":"cancel requested","retryable":false}"#,
            ),
            None,
        )?;
        return Err(AppError::Cancelled);
    }

    let plan_id = job
        .render_plan_id
        .clone()
        .or_else(|| {
            serde_json::from_str::<serde_json::Value>(&job.input_json)
                .ok()
                .and_then(|v| {
                    v.get("renderPlanId")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string())
                })
        })
        .ok_or_else(|| AppError::Invalid("job sin renderPlanId".into()))?;

    let plan = get_render_plan(&plan_id)?.ok_or_else(|| AppError::NotFound(plan_id.clone()))?;

    let work_dir = crate::vnext::persistence::vnext_root()?
        .join("jobs")
        .join(job_id);
    std::fs::create_dir_all(&work_dir)?;

    // Entrega al lado del video original:
    //   C:\Videos\MiVideo.mp4  →  C:\Videos\MiVideo\shorts\MiVideo_short.mp4
    // Fallback: work_dir si no se puede crear junto al original.
    let final_out = deliverable_path_for_source(&plan.source_media_path, &plan.candidate_id)
        .unwrap_or_else(|| work_dir.join("output-final.mp4"));
    if let Some(parent) = final_out.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let ffmpeg = Ffmpeg::new()?;
    let (src_w, src_h) = match ffmpeg.probe(Path::new(&plan.source_media_path)).await {
        Ok(info) => (info.width.max(2), info.height.max(2)),
        Err(_) => (1920, 1080),
    };

    let result = match render_vertical_plan(&plan, &final_out, &work_dir, src_w, src_h).await {
        Ok(r) => r,
        Err(e) => {
            cleanup_job_work_dir(&work_dir);
            let err = serde_json::json!({
                "contractVersion": "v1",
                "code": "ffmpeg_failed",
                "message": e.to_string(),
                "retryable": true,
            });
            let _ = set_job_status(
                job_id,
                JobStatus::Running,
                JobStatus::Failed,
                &chrono::Utc::now().to_rfc3339(),
                Some(&err.to_string()),
                None,
            );
            return Err(e);
        }
    };

    let name = result
        .final_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if name.contains("vigilcut-tmp") {
        cleanup_job_work_dir(&work_dir);
        return Err(AppError::Ffmpeg(
            "output final no puede ser path temporal".into(),
        ));
    }

    let sha = sha256_file(&result.final_path)?;
    let art_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let probe = serde_json::json!({
        "durationS": result.duration_s,
        "width": plan.output_spec.width,
        "height": plan.output_spec.height,
        "hasAudio": true,
        "hasVideo": true,
        "videoCodec": plan.output_spec.video_codec,
        "audioCodec": plan.output_spec.audio_codec,
    });
    insert_artifact(&InsertArtifact {
        id: art_id.clone(),
        content_project_id: job.content_project_id.clone(),
        kind: "vertical_mp4".into(),
        role: "final_deliverable".into(),
        path: result.final_path.to_string_lossy().into_owned(),
        sha256: sha,
        byte_size: result.byte_size as i64,
        mime_type: "video/mp4".into(),
        created_by_job_id: job_id.into(),
        probe_json: Some(probe.to_string()),
        validation_status: ArtifactValidationStatus::Passed,
        validation_notes: if result.burned_subtitles {
            Some("subtitles_burned:safe_center_bottom_v1".into())
        } else {
            Some("video_only".into())
        },
        parent_ids: vec![],
        created_at: now,
    })?;

    let manifest_path = work_dir.join("artifact-manifest.json");
    let manifest = serde_json::json!({
        "contractVersion": "v1",
        "jobId": job_id,
        "renderPlanId": plan.id,
        "artifactId": art_id,
        "path": result.final_path,
        "burnedSubtitles": result.burned_subtitles,
        "lineage": {
            "candidateId": plan.candidate_id,
            "sourceMediaPath": plan.source_media_path,
            "sourceStartS": plan.source_start_s,
            "sourceEndS": plan.source_end_s,
        }
    });
    let _ = std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    );

    complete_job_with_artifact(job_id, &art_id)?;
    cleanup_job_work_dir(&work_dir);
    Ok(result)
}

/// Convenience for tests when FFmpeg + a real media file are available.
pub async fn render_candidate_now(
    run_id: &str,
    candidate_id: &str,
    cues: Vec<SubtitleCueV1>,
    burn_in: bool,
) -> AppResult<(VerticalRenderPlanV1, VerticalRenderResult)> {
    let plan = create_plan_for_candidate_id(run_id, candidate_id, cues, burn_in)?;
    let project_id = plan.content_project_id.clone();
    let job_id = enqueue_vertical_render(&plan.id, &project_id)?;
    let result = execute_vertical_render_job(&job_id).await?;
    Ok((plan, result))
}

/// Register a pre-validated file as the job's final artifact (unit tests without FFmpeg).
pub fn complete_with_fixture_file(
    job_id: &str,
    content_project_id: &str,
    final_path: &Path,
) -> AppResult<String> {
    use crate::pipeline::safe_paths::validate_export_output;
    validate_export_output(final_path, 1.0)?;
    let name = final_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if name.contains("vigilcut-tmp") {
        return Err(AppError::Invalid(
            "no se puede completar con path temporal".into(),
        ));
    }
    let sha = sha256_file(final_path)?;
    let meta = std::fs::metadata(final_path)?;
    let art_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    // Ensure job is running for complete transition
    let job = get_job(job_id)?.ok_or_else(|| AppError::NotFound(job_id.into()))?;
    let st = JobStatus::parse(&job.status).map_err(|e| AppError::Invalid(e.to_string()))?;
    if st == JobStatus::Queued {
        set_job_status(
            job_id,
            JobStatus::Queued,
            JobStatus::Running,
            &now,
            None,
            None,
        )?;
    }
    insert_artifact(&InsertArtifact {
        id: art_id.clone(),
        content_project_id: content_project_id.into(),
        kind: "vertical_mp4".into(),
        role: "final_deliverable".into(),
        path: final_path.to_string_lossy().into_owned(),
        sha256: sha,
        byte_size: meta.len() as i64,
        mime_type: "video/mp4".into(),
        created_by_job_id: job_id.into(),
        probe_json: Some(
            r#"{"durationS":1.0,"width":1080,"height":1920,"hasAudio":true,"hasVideo":true}"#
                .into(),
        ),
        validation_status: ArtifactValidationStatus::Passed,
        validation_notes: Some("fixture".into()),
        parent_ids: vec![],
        created_at: now,
    })?;
    complete_job_with_artifact(job_id, &art_id)?;
    Ok(art_id)
}
