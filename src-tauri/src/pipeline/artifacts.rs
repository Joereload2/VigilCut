use std::path::{Path, PathBuf};

use crate::error::AppResult;
use crate::models::analysis::AnalysisRun;
use crate::models::artifacts::{
    ArtifactRef, ART_CHAPTERS, ART_EDL, ART_EVENTS, ART_LONGFORM, ART_MANIFEST, ART_SHORTS,
};
use crate::models::preset::ExportOptions;
use crate::pipeline::detectors::{chapters_from_events, shorts_from_events};
use crate::pipeline::export::export_clip;

/// Write factory sidecar artifacts next to the longform export.
///
/// Layout (creator-friendly):
/// - `{stem}.mp4`              — video principal (lo que subes)
/// - `{stem}.chapters.txt`     — timestamps YouTube
/// - `{stem}-meta/`            — JSON de fábrica + EDL para re-importar
/// - `{stem}-shorts/`          — clips MP4 opcionales
pub async fn write_run_artifacts(
    run: &AnalysisRun,
    output_mp4: &Path,
    source_media: &Path,
    export_shorts: bool,
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

    // All machine-readable sidecars go in a subfolder so Explorer
    // shows the MP4 as the main export, not a pile of JSON.
    let meta_dir = dir.join(format!("{stem}-meta"));
    std::fs::create_dir_all(&meta_dir)?;

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
        label: Some("Video editado (MP4)".into()),
    }];

    // YouTube chapters next to the video (human-facing, not JSON)
    let chapters_txt = dir.join(format!("{stem}.chapters.txt"));
    let mut txt = String::from("0:00 Intro\n");
    for c in &chapters {
        // Skip near-zero duplicates of the default intro line
        if c.at_output < 0.5 {
            continue;
        }
        txt.push_str(&format!("{} {}\n", format_ts(c.at_output), c.title));
    }
    // If we already had a real first chapter at 0, rewrite cleanly
    if chapters
        .first()
        .map(|c| c.at_output < 0.5)
        .unwrap_or(false)
    {
        txt.clear();
        for c in &chapters {
            txt.push_str(&format!("{} {}\n", format_ts(c.at_output), c.title));
        }
    }
    std::fs::write(&chapters_txt, txt)?;
    artifacts.push(ArtifactRef {
        kind: "chapters_txt".into(),
        path: chapters_txt.to_string_lossy().into_owned(),
        label: Some("Capítulos YouTube (.txt)".into()),
    });

    // --- meta/ : factory JSON + NLE EDL (optional for power users) ---

    let events_path = meta_dir.join("events.json");
    std::fs::write(&events_path, serde_json::to_string_pretty(&run.events)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_EVENTS.into(),
        path: events_path.to_string_lossy().into_owned(),
        label: Some("Eventos de análisis".into()),
    });

    let edl_json_path = meta_dir.join("edl.json");
    std::fs::write(&edl_json_path, serde_json::to_string_pretty(&run.edl)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_EDL.into(),
        path: edl_json_path.to_string_lossy().into_owned(),
        label: Some("EDL JSON (interno)".into()),
    });

    // Classic CMX3600-style EDL for Premiere / Resolve / FCP
    let edl_txt_path = meta_dir.join("cutlist.edl");
    std::fs::write(&edl_txt_path, build_cmx_edl(stem, source_media, &keep))?;
    artifacts.push(ArtifactRef {
        kind: "edl_cmx".into(),
        path: edl_txt_path.to_string_lossy().into_owned(),
        label: Some("EDL CMX (NLE)".into()),
    });

    let chapters_path = meta_dir.join("chapters.json");
    std::fs::write(&chapters_path, serde_json::to_string_pretty(&chapters)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_CHAPTERS.into(),
        path: chapters_path.to_string_lossy().into_owned(),
        label: Some("Capítulos JSON".into()),
    });

    let shorts_path = meta_dir.join("shorts.json");
    std::fs::write(&shorts_path, serde_json::to_string_pretty(&shorts)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_SHORTS.into(),
        path: shorts_path.to_string_lossy().into_owned(),
        label: Some("Candidatos Shorts".into()),
    });

    // Real short clips (top 5 by score) as MP4, not JSON
    let mut short_clips: Vec<ArtifactRef> = Vec::new();
    if export_shorts && !shorts.is_empty() {
        let shorts_dir = dir.join(format!("{stem}-shorts"));
        std::fs::create_dir_all(&shorts_dir)?;
        let opts = ExportOptions {
            crf: 20,
            preset: "veryfast".into(),
            ..ExportOptions::default()
        };
        for (i, s) in shorts.iter().take(5).enumerate() {
            let clip_path = shorts_dir.join(format!("short-{:02}.mp4", i + 1));
            match export_clip(source_media, &clip_path, s.start, s.end, &opts).await {
                Ok(p) => {
                    short_clips.push(ArtifactRef {
                        kind: "short_mp4".into(),
                        path: p.to_string_lossy().into_owned(),
                        label: Some(format!(
                            "Short {} ({:.0}s, score {:.0}%)",
                            i + 1,
                            s.end - s.start,
                            s.score * 100.0
                        )),
                    });
                }
                Err(e) => {
                    tracing::warn!("short clip {} failed: {e}", i + 1);
                }
            }
        }
        artifacts.extend(short_clips.clone());
    }

    let breaths = run
        .events
        .iter()
        .filter(|e| e.event_type == "audio.breath")
        .count();
    let fillers = run
        .events
        .iter()
        .filter(|e| e.event_type == "speech.filler")
        .count();

    // Copy cached SRT next to the video (publish-ready)
    for a in &run.artifacts {
        if a.kind == "captions_srt_cache" {
            let dest = dir.join(format!("{stem}.srt"));
            if std::fs::copy(&a.path, &dest).is_ok() {
                artifacts.push(ArtifactRef {
                    kind: "captions_srt".into(),
                    path: dest.to_string_lossy().into_owned(),
                    label: Some("Subtítulos SRT".into()),
                });
            }
        }
    }

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
        "shortClips": short_clips,
        "breathEvents": breaths,
        "fillerEvents": fillers,
        "extra": extra,
    });
    let manifest_path = meta_dir.join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    artifacts.push(ArtifactRef {
        kind: ART_MANIFEST.into(),
        path: manifest_path.to_string_lossy().into_owned(),
        label: Some("Manifiesto fábrica".into()),
    });

    // README so the meta folder is self-explanatory
    let readme = meta_dir.join("README.txt");
    let _ = std::fs::write(
        &readme,
        format!(
            "VigilCut — metadatos de fábrica\n\
             ================================\n\
             El video listo para publicar está un nivel arriba:\n\
               {stem}.mp4\n\
               {stem}.chapters.txt  (timestamps YouTube)\n\n\
             Aquí solo hay datos para re-análisis / NLE:\n\
               events.json   — eventos detectados\n\
               edl.json      — cutlist interno VigilCut\n\
               cutlist.edl   — EDL CMX3600 (Premiere/Resolve)\n\
               chapters.json — capítulos estructurados\n\
               shorts.json   — candidatos a Short\n\
               manifest.json — resumen completo del run\n"
        ),
    );

    Ok(artifacts)
}

/// Minimal CMX3600 EDL from keep ranges (source timebase).
fn build_cmx_edl(title: &str, source: &Path, keep: &[(f64, f64)]) -> String {
    let src_name = source
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("SOURCE");
    let mut out = String::new();
    out.push_str(&format!("TITLE: {title}\n"));
    out.push_str("FCM: NON-DROP FRAME\n\n");

    let mut rec_cursor = 0.0_f64;
    for (i, (start, end)) in keep.iter().enumerate() {
        let dur = (end - start).max(0.0);
        if dur <= 0.001 {
            continue;
        }
        let event = i + 1;
        let src_in = format_edl_tc(*start);
        let src_out = format_edl_tc(*end);
        let rec_in = format_edl_tc(rec_cursor);
        let rec_out = format_edl_tc(rec_cursor + dur);
        out.push_str(&format!(
            "{event:03}  AX       V     C        {src_in} {src_out} {rec_in} {rec_out}\n"
        ));
        out.push_str(&format!("* FROM CLIP NAME: {src_name}\n\n"));
        rec_cursor += dur;
    }
    out
}

fn format_edl_tc(seconds: f64) -> String {
    // 30 fps non-drop timecode for export EDL
    let total = (seconds.max(0.0) * 30.0).round() as u64;
    let f = total % 30;
    let s = (total / 30) % 60;
    let m = (total / 30 / 60) % 60;
    let h = total / 30 / 60 / 60;
    format!("{h:02}:{m:02}:{s:02}:{f:02}")
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
