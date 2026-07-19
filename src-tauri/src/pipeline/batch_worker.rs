use std::path::{Path, PathBuf};

use crate::error::AppResult;
use crate::models::batch::{BatchFileResult, BatchJob, BatchStatus};
use crate::models::edl::PolicyConfig;
use crate::models::preset::{ColorOptions, ExportOptions};
use crate::models::segment::{Segment, SegmentDecision};
use crate::pipeline::artifacts::write_run_artifacts;
use crate::pipeline::engine::{accept_all_exceptions, run_silence_analysis};
use crate::pipeline::export::export_from_edl;

/// Process one media file: analyze → force exceptions (factory) → export + artifacts.
pub async fn process_one_file(
    media_path: &Path,
    output_dir: &Path,
    policy: &PolicyConfig,
    auto_accept_exceptions: bool,
    export_opts: &ExportOptions,
    color: &ColorOptions,
) -> BatchFileResult {
    let path_str = media_path.to_string_lossy().into_owned();
    let stem = media_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let out_path = output_dir.join(format!("{stem}-editado.mp4"));

    match run_one(
        media_path,
        &out_path,
        policy,
        auto_accept_exceptions,
        export_opts,
        color,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => BatchFileResult {
            media_path: path_str,
            ok: false,
            output_path: None,
            auto_cuts: 0,
            exceptions_pending: 0,
            exceptions_forced: 0,
            source_duration: 0.0,
            output_duration: 0.0,
            error: Some(e.to_string()),
        },
    }
}

async fn run_one(
    media_path: &Path,
    out_path: &Path,
    policy: &PolicyConfig,
    auto_accept_exceptions: bool,
    export_opts: &ExportOptions,
    color: &ColorOptions,
) -> AppResult<BatchFileResult> {
    let path_str = media_path.to_string_lossy().into_owned();
    let mut run = run_silence_analysis(media_path, policy).await?;
    let auto_cuts = run.stats.auto_cut_count;
    let pending_before = run.stats.pending_exception_count;

    let mut exceptions_forced = 0;
    if auto_accept_exceptions && pending_before > 0 {
        run = accept_all_exceptions(run);
        exceptions_forced = pending_before;
    }

    let has_audio = true;

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Factory path: EDL is the export source of truth (not Segment UI state).
    export_from_edl(
        media_path,
        out_path,
        &run.edl,
        export_opts,
        color,
        has_audio,
    )
    .await?;

    let artifacts = write_run_artifacts(
        &run,
        out_path,
        media_path,
        true, // export real short clips
        serde_json::json!({
            "autoCuts": auto_cuts,
            "exceptionsForced": exceptions_forced,
            "factory": true,
        }),
    )
    .await?;
    run.artifacts = artifacts;

    Ok(BatchFileResult {
        media_path: path_str,
        ok: true,
        output_path: Some(out_path.to_string_lossy().into_owned()),
        auto_cuts,
        exceptions_pending: run.stats.pending_exception_count,
        exceptions_forced,
        source_duration: run.duration,
        output_duration: run.edl.output_duration,
        error: None,
    })
}

/// Collect video files from a directory (non-recursive).
pub fn list_videos_in_dir(dir: &Path) -> AppResult<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let exts = ["mp4", "mov", "mkv", "webm", "m4v", "avi", "wmv"];
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();
        if exts.contains(&ext.as_str()) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Run entire batch job synchronously (CLI).
pub async fn run_batch_job(mut job: BatchJob, policy: PolicyConfig) -> BatchJob {
    job.status = BatchStatus::Running;
    job.touch();
    let total = job.media_paths.len().max(1);
    let export_opts = ExportOptions::default();
    let color = ColorOptions::default();
    let out_dir = PathBuf::from(&job.output_dir);
    let _ = std::fs::create_dir_all(&out_dir);

    for (i, path_str) in job.media_paths.clone().iter().enumerate() {
        job.current_file = Some(path_str.clone());
        job.progress = i as f64 / total as f64;
        job.touch();

        let path = PathBuf::from(path_str);
        let result = process_one_file(
            &path,
            &out_dir,
            &policy,
            job.auto_accept_exceptions,
            &export_opts,
            &color,
        )
        .await;

        if result.ok {
            job.completed += 1;
        } else {
            job.failed += 1;
            if let Some(err) = &result.error {
                job.errors.push(format!("{path_str}: {err}"));
            }
        }
        job.results.push(result);
        job.progress = (i + 1) as f64 / total as f64;
        job.touch();
    }

    job.current_file = None;
    job.status = if job.failed == 0 && job.completed > 0 {
        BatchStatus::Completed
    } else if job.completed == 0 {
        BatchStatus::Failed
    } else {
        BatchStatus::Completed
    };
    job.touch();
    job
}

#[allow(dead_code)]
pub fn segments_from_keep_only(duration: f64, keep: &[(f64, f64)]) -> Vec<Segment> {
    let mut segs = Vec::new();
    let mut cursor = 0.0;
    for (s, e) in keep {
        if *s > cursor + 0.01 {
            let mut cut = Segment::new(
                cursor,
                *s,
                crate::models::segment::SegmentKind::Silence,
                SegmentDecision::Cut,
            );
            cut.auto_applied = true;
            segs.push(cut);
        }
        segs.push(Segment::new(
            *s,
            *e,
            crate::models::segment::SegmentKind::Speech,
            SegmentDecision::Keep,
        ));
        cursor = *e;
    }
    if cursor < duration - 0.01 {
        let mut cut = Segment::new(
            cursor,
            duration,
            crate::models::segment::SegmentKind::Silence,
            SegmentDecision::Cut,
        );
        cut.auto_applied = true;
        segs.push(cut);
    }
    segs
}
