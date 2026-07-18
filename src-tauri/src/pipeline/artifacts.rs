use std::path::{Path, PathBuf};

use crate::error::AppResult;
use crate::models::analysis::AnalysisRun;
use crate::models::artifacts::{
    ArtifactRef, ART_CHAPTERS, ART_EDL, ART_EVENTS, ART_LONGFORM, ART_MANIFEST, ART_SHORTS,
};
use crate::pipeline::detectors::{chapters_from_events, shorts_from_events};

/// Write factory sidecar artifacts next to the longform export.
pub fn write_run_artifacts(
    run: &AnalysisRun,
    output_mp4: &Path,
    extra: serde_json::Value,
) -> AppResult<Vec<ArtifactRef>> {
    let dir = output_mp4
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = output_mp4
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    let keep: Vec<(f64, f64)> = run
        .edl
        .video_track
        .iter()
        .map(|s| (s.start, s.end))
        .collect();

    let chapters = chapters_from_events(&run.events, &keep);
    let shorts = shorts_from_events(&run.events);

    let mut artifacts = vec![ArtifactRef {
        kind: ART_LONGFORM.into(),
        path: output_mp4.to_string_lossy().into_owned(),
        label: Some("Video editado".into()),
    }];

    // events
    let events_path = dir.join(format!("{stem}.events.json"));
    std::fs::write(
        &events_path,
        serde_json::to_string_pretty(&run.events)?,
    )?;
    artifacts.push(ArtifactRef {
        kind: ART_EVENTS.into(),
        path: events_path.to_string_lossy().into_owned(),
        label: Some("Eventos de análisis".into()),
    });

    // edl
    let edl_path = dir.join(format!("{stem}.edl.json"));
    std::fs::write(&edl_path, serde_json::to_string_pretty(&run.edl)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_EDL.into(),
        path: edl_path.to_string_lossy().into_owned(),
        label: Some("EDL / cutlist".into()),
    });

    // chapters (YouTube-friendly text + json)
    let chapters_path = dir.join(format!("{stem}.chapters.json"));
    std::fs::write(
        &chapters_path,
        serde_json::to_string_pretty(&chapters)?,
    )?;
    artifacts.push(ArtifactRef {
        kind: ART_CHAPTERS.into(),
        path: chapters_path.to_string_lossy().into_owned(),
        label: Some("Capítulos".into()),
    });

    let chapters_txt = dir.join(format!("{stem}.chapters.txt"));
    let mut txt = String::new();
    for c in &chapters {
        let t = format_ts(c.at_output);
        txt.push_str(&format!("{} {}\n", t, c.title));
    }
    std::fs::write(&chapters_txt, txt)?;

    // shorts candidates
    let shorts_path = dir.join(format!("{stem}.shorts.json"));
    std::fs::write(&shorts_path, serde_json::to_string_pretty(&shorts)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_SHORTS.into(),
        path: shorts_path.to_string_lossy().into_owned(),
        label: Some("Candidatos Shorts".into()),
    });

    // master manifest
    let manifest = serde_json::json!({
        "source": run.media_path,
        "output": output_mp4.to_string_lossy(),
        "runId": run.id,
        "method": run.method,
        "stats": run.stats,
        "policy": run.policy,
        "artifacts": artifacts,
        "chapters": chapters,
        "shorts": shorts,
        "extra": extra,
    });
    let manifest_path = dir.join(format!("{stem}.json"));
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest)?,
    )?;
    artifacts.push(ArtifactRef {
        kind: ART_MANIFEST.into(),
        path: manifest_path.to_string_lossy().into_owned(),
        label: Some("Manifiesto fábrica".into()),
    });

    Ok(artifacts)
}

fn format_ts(seconds: f64) -> String {
    let s = seconds.max(0.0) as u64;
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let sec = s % 60;
    if h > 0 {
        format!("{h}:{m:02}:{sec:02}")
    } else {
        format!("{m}:{sec:02}")
    }
}
