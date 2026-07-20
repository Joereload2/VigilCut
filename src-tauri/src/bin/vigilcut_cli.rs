//! VigilCut factory CLI — no UI.
//!
//! ```text
//! cargo run --bin vigilcut-cli -- analyze path/to/video.mp4
//! cargo run --bin vigilcut-cli -- batch ./inbox ./outbox
//! ```

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use vigilcut_lib::pipeline::batch_worker::{list_videos_in_dir, process_one_file, run_batch_job};
use vigilcut_lib::pipeline::clipping::{export_approved_clips, run_clipping_analysis};
use vigilcut_lib::pipeline::engine::run_silence_analysis;
use vigilcut_lib::models::batch::BatchJob;
use vigilcut_lib::models::clipping::{ClipReviewStatus, ClippingOptions};
use vigilcut_lib::models::edl::PolicyConfig;
use vigilcut_lib::models::preset::{ColorOptions, ExportOptions};

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vigilcut=info".into()),
        )
        .init();

    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return ExitCode::FAILURE;
    }

    let cmd = args.remove(0);
    let rt = tokio::runtime::Runtime::new().expect("tokio");

    match cmd.as_str() {
        "analyze" => {
            let path = args.first().cloned().unwrap_or_default();
            if path.is_empty() {
                eprintln!("usage: vigilcut-cli analyze <video> [--policy factory|youtube|podcast|gentle|shorts-first]");
                return ExitCode::FAILURE;
            }
            let policy = policy_from_args(&args);
            match rt.block_on(run_silence_analysis(
                PathBuf::from(&path).as_path(),
                &policy,
            )) {
                Ok(run) => {
                    println!("run_id={}", run.id);
                    println!("method={}", run.method);
                    println!("auto_cuts={}", run.stats.auto_cut_count);
                    println!("exceptions={}", run.stats.pending_exception_count);
                    println!(
                        "duration {:.2}s → {:.2}s (−{:.2}s)",
                        run.duration, run.edl.output_duration, run.edl.removed_duration
                    );
                    let chapters = run
                        .events
                        .iter()
                        .filter(|e| e.event_type == "structure.chapter")
                        .count();
                    let shorts = run
                        .events
                        .iter()
                        .filter(|e| e.event_type == "short.candidate")
                        .count();
                    println!("chapters={chapters} short_candidates={shorts}");
                    if run.stats.pending_exception_count > 0 {
                        println!("pending exceptions:");
                        for ex in run.exceptions.iter().filter(|e| e.is_pending()) {
                            println!(
                                "  [{:.2}-{:.2}] conf={:.0}% {}",
                                ex.span.start,
                                ex.span.end,
                                ex.confidence * 100.0,
                                ex.rationale
                            );
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        "batch" => {
            let inbox = args.first().cloned().unwrap_or_else(|| ".".into());
            let outbox = args
                .get(1)
                .cloned()
                .unwrap_or_else(|| format!("{inbox}/../outbox"));
            let inbox_path = PathBuf::from(&inbox);
            let videos = match list_videos_in_dir(&inbox_path) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            if videos.is_empty() {
                eprintln!("no videos in {inbox}");
                return ExitCode::FAILURE;
            }
            let paths: Vec<String> = videos
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            println!("batch {} files → {outbox}", paths.len());
            let policy = policy_from_args(&args);
            let mode = if args.iter().any(|a| a == "--aggressive") {
                vigilcut_lib::models::exception_mode::ExceptionHandlingMode::Aggressive
            } else {
                vigilcut_lib::models::exception_mode::ExceptionHandlingMode::Safe
            };
            let job = BatchJob::new(paths, "cli".into(), outbox, mode);
            let done = rt.block_on(run_batch_job(job, policy));
            println!(
                "done: {} ok, {} failed, status={:?}",
                done.completed, done.failed, done.status
            );
            for r in &done.results {
                if r.ok {
                    println!(
                        "  OK  {} → {} (−{:.1}s)",
                        r.media_path,
                        r.output_path.as_deref().unwrap_or("?"),
                        r.source_duration - r.output_duration
                    );
                } else {
                    println!("  ERR {} — {}", r.media_path, r.error.as_deref().unwrap_or("?"));
                }
            }
            if done.failed > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "export" => {
            // analyze + force exceptions + export single file (factory headless path)
            let path = args.first().cloned().unwrap_or_default();
            let out = args.get(1).cloned();
            if path.is_empty() {
                eprintln!(
                    "usage: vigilcut-cli export <video> [out.mp4|outdir] [--policy factory|...]"
                );
                return ExitCode::FAILURE;
            }
            let media = PathBuf::from(&path);
            let policy = policy_from_args(&args);
            let (out_dir, rename_to) = match out.as_ref() {
                Some(p) => {
                    let pb = PathBuf::from(p);
                    if pb.extension().and_then(|e| e.to_str()) == Some("mp4") {
                        let dir = pb
                            .parent()
                            .filter(|par| !par.as_os_str().is_empty())
                            .map(|par| par.to_path_buf())
                            .unwrap_or_else(|| PathBuf::from("."));
                        (dir, Some(pb))
                    } else {
                        (pb, None)
                    }
                }
                None => (
                    media
                        .parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from(".")),
                    None,
                ),
            };
            // CLI export defaults to Safe (keep pending exceptions). Use --aggressive to force-cut.
            let mode = if args.iter().any(|a| a == "--aggressive") {
                vigilcut_lib::models::exception_mode::ExceptionHandlingMode::Aggressive
            } else {
                vigilcut_lib::models::exception_mode::ExceptionHandlingMode::Safe
            };
            let result = rt.block_on(process_one_file(
                &media,
                &out_dir,
                &policy,
                mode,
                &ExportOptions::default(),
                &ColorOptions::default(),
            ));
            if result.ok {
                let mut final_path = result.output_path.clone().unwrap_or_default();
                if let (Some(target), Some(src)) = (rename_to, result.output_path.as_ref()) {
                    if src != &target.to_string_lossy() {
                        if let Err(e) = std::fs::rename(src, &target) {
                            eprintln!("warn: could not rename to {}: {e}", target.display());
                        } else {
                            // Move meta folder if present next to default name
                            final_path = target.to_string_lossy().into_owned();
                        }
                    }
                }
                println!("exported {final_path}");
                println!(
                    "auto_cuts={} exceptions_forced={}",
                    result.auto_cuts, result.exceptions_forced
                );
                ExitCode::SUCCESS
            } else {
                eprintln!("{}", result.error.unwrap_or_else(|| "export failed".into()));
                ExitCode::FAILURE
            }
        }
        "clips" => {
            // Analyze + auto-approve preselected + export 9:16
            let path = args.first().cloned().unwrap_or_default();
            if path.is_empty() {
                eprintln!("usage: vigilcut-cli clips <video.mp4> [outdir]");
                return ExitCode::FAILURE;
            }
            let media = PathBuf::from(&path);
            let out_dir = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
                let stem = media
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("video");
                let parent = media
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));
                parent.join(format!("{stem}-clips"))
            });
            let opts = ClippingOptions {
                prefer_whisper: true,
                ..ClippingOptions::default()
            };
            let mut run = match rt.block_on(run_clipping_analysis(&media, opts)) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            println!(
                "candidates={} preselected={} best={:.0}",
                run.summary.candidates_found, run.summary.preselected, run.summary.best_score
            );
            for c in run.candidates.iter_mut() {
                if c.is_primary_variant
                    && matches!(
                        c.status,
                        ClipReviewStatus::Preselected | ClipReviewStatus::Suggested
                    )
                    && c.score >= 55.0
                {
                    c.status = ClipReviewStatus::Approved;
                }
            }
            let (w, h) = match rt.block_on(async {
                use vigilcut_lib::ffmpeg::Ffmpeg;
                Ffmpeg::new()?.probe(&media).await.map(|i| (i.width.max(2), i.height.max(2)))
            }) {
                Ok(dims) => dims,
                Err(e) => {
                    eprintln!("warn: probe failed ({e}), assuming 1920x1080");
                    (1920, 1080)
                }
            };
            let results = match rt.block_on(export_approved_clips(
                &media,
                &mut run.candidates,
                &[],
                &out_dir,
                None,
                w,
                h,
            )) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("export error: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let ok = results.iter().filter(|r| r.ok).count();
            println!("exported {ok}/{} → {}", results.len(), out_dir.display());
            if ok == 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        "visual" => {
            // Headless visual library / transcript helpers
            let sub = args.first().cloned().unwrap_or_default();
            match sub.as_str() {
                "import" => {
                    let path = args.get(1).cloned().unwrap_or_default();
                    if path.is_empty() {
                        eprintln!(
                            "usage: vigilcut-cli visual import <image|folder> [--concepts a,b] [--recursive]"
                        );
                        return ExitCode::FAILURE;
                    }
                    let concepts = arg_csv(&args, "--concepts");
                    let tags = arg_csv(&args, "--tags");
                    let recursive = args.iter().any(|a| a == "--recursive");
                    let p = PathBuf::from(&path);
                    if p.is_dir() {
                        match vigilcut_lib::pipeline::visual::library::import_folder(
                            &p,
                            tags,
                            concepts,
                            recursive,
                        ) {
                            Ok(r) => {
                                println!(
                                    "folder scanned={} imported={} duplicates={} failed={}",
                                    r.scanned, r.imported, r.duplicates, r.failed
                                );
                                for e in r.errors.iter().take(5) {
                                    eprintln!("  warn: {e}");
                                }
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                eprintln!("error: {e}");
                                ExitCode::FAILURE
                            }
                        }
                    } else {
                        match vigilcut_lib::pipeline::visual::import_library_image(
                            &p,
                            None,
                            tags,
                            concepts,
                        ) {
                            Ok(a) => {
                                println!("imported id={} sha256={}", a.id, a.sha256);
                                ExitCode::SUCCESS
                            }
                            Err(e) => {
                                eprintln!("error: {e}");
                                ExitCode::FAILURE
                            }
                        }
                    }
                }
                "list" => {
                    let q = args.get(1).cloned();
                    match vigilcut_lib::pipeline::visual::library::list_assets(
                        q.as_deref(),
                        100,
                    ) {
                        Ok(list) => {
                            println!("{} assets", list.len());
                            for a in list {
                                println!(
                                    "  {}  {}  concepts=[{}]  used={}  {}",
                                    a.id,
                                    a.title,
                                    a.concepts.join(","),
                                    a.times_used,
                                    a.status_label()
                                );
                            }
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("error: {e}");
                            ExitCode::FAILURE
                        }
                    }
                }
                "transcript" => {
                    let media = args.get(1).cloned().unwrap_or_default();
                    let out = args.get(2).cloned().unwrap_or_else(|| ".".into());
                    if media.is_empty() {
                        eprintln!(
                            "usage: vigilcut-cli visual transcript <video.mp4> [outdir] [--srt path] [--whisper]"
                        );
                        return ExitCode::FAILURE;
                    }
                    let media_p = PathBuf::from(&media);
                    let srt = arg_value(&args, "--srt").map(PathBuf::from);
                    let whisper = args.iter().any(|a| a == "--whisper");
                    let tr = match rt.block_on(
                        vigilcut_lib::pipeline::transcript_engine::build_transcript(
                            &media_p,
                            srt.as_deref(),
                            whisper,
                            None,
                        ),
                    ) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("error: {e}");
                            return ExitCode::FAILURE;
                        }
                    };
                    let stem = media_p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("media");
                    match vigilcut_lib::pipeline::transcript_engine::write_transcript_artifacts(
                        &tr,
                        PathBuf::from(&out).as_path(),
                        stem,
                    ) {
                        Ok(arts) => {
                            println!("status={:?} segments={}", tr.status, tr.segments.len());
                            for (k, p) in arts {
                                println!("  {k} → {p}");
                            }
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("error: {e}");
                            ExitCode::FAILURE
                        }
                    }
                }
                "scan-missing" => match vigilcut_lib::pipeline::visual::library::scan_missing_assets()
                {
                    Ok(n) => {
                        println!("marked_missing={n}");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        ExitCode::FAILURE
                    }
                },
                "enrich" => {
                    // Headless: silence EDL + transcript + suggestions + plan JSON (no auto-accept).
                    let media = args.get(1).cloned().unwrap_or_default();
                    let out = args.get(2).cloned().unwrap_or_else(|| {
                        PathBuf::from(&media)
                            .parent()
                            .map(|p| p.join("visual-out").to_string_lossy().into_owned())
                            .unwrap_or_else(|| "visual-out".into())
                    });
                    if media.is_empty() {
                        eprintln!(
                            "usage: vigilcut-cli visual enrich <video.mp4> [outdir] [--srt path] [--whisper] [--policy factory]"
                        );
                        return ExitCode::FAILURE;
                    }
                    let media_p = PathBuf::from(&media);
                    let out_dir = PathBuf::from(&out);
                    let _ = std::fs::create_dir_all(&out_dir);
                    let srt = arg_value(&args, "--srt").map(PathBuf::from);
                    let whisper = args.iter().any(|a| a == "--whisper");
                    let policy = policy_from_args(&args);
                    let run = match rt.block_on(
                        vigilcut_lib::pipeline::engine::run_silence_analysis(&media_p, &policy),
                    ) {
                        Ok(r) => r,
                        Err(e) => {
                            eprintln!("analyze error: {e}");
                            return ExitCode::FAILURE;
                        }
                    };
                    let run_id = vigilcut_lib::models::visual::edl_fingerprint(&run.edl.keep_ranges());
                    let time_map = vigilcut_lib::pipeline::time_map::TimeMap::from_edl(&run.edl);
                    let tr = match rt.block_on(
                        vigilcut_lib::pipeline::transcript_engine::build_transcript(
                            &media_p,
                            srt.as_deref(),
                            whisper,
                            Some(run_id.clone()),
                        ),
                    ) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("transcript error: {e}");
                            return ExitCode::FAILURE;
                        }
                    };
                    let stem = media_p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("media");
                    if let Err(e) =
                        vigilcut_lib::pipeline::transcript_engine::write_transcript_artifacts(
                            &tr, &out_dir, stem,
                        )
                    {
                        eprintln!("warn: transcript artifacts: {e}");
                    }
                    let semantics =
                        vigilcut_lib::pipeline::semantic::extract_semantic_events(
                            &tr, &run_id, &time_map,
                        );
                    let assets =
                        vigilcut_lib::pipeline::visual::library::list_active_assets()
                            .unwrap_or_default();
                    let suggestions = vigilcut_lib::pipeline::visual::match_rank::rank_suggestions(
                        &semantics,
                        &assets,
                        time_map.output_duration,
                        &vigilcut_lib::pipeline::visual::match_rank::MatchConfig::default(),
                    );
                    let mut plan = vigilcut_lib::models::visual::VisualPlan::new(
                        &run_id,
                        media_p.to_string_lossy(),
                        run_id.clone(),
                    );
                    if assets.is_empty() {
                        plan.warnings.push(
                            "Biblioteca vacía: visual import <imagenes> --concepts …".into(),
                        );
                    }
                    if suggestions.is_empty() {
                        plan.warnings.push(
                            "Sin sugerencias (transcripción o conceptos de biblioteca).".into(),
                        );
                    }
                    let sug_path = out_dir.join(format!("{stem}.visual-suggestions.json"));
                    let plan_path = out_dir.join(format!("{stem}.visual-plan.json"));
                    let sem_path = out_dir.join(format!("{stem}.semantic-events.json"));
                    let _ = std::fs::write(
                        &sug_path,
                        serde_json::to_string_pretty(&suggestions).unwrap_or_default(),
                    );
                    let _ = std::fs::write(
                        &sem_path,
                        serde_json::to_string_pretty(&semantics).unwrap_or_default(),
                    );
                    if let Err(e) =
                        vigilcut_lib::pipeline::visual::save_visual_plan(&plan, Some(&plan_path))
                    {
                        eprintln!("warn: plan save: {e}");
                    }
                    println!("run_id={run_id}");
                    println!(
                        "edl {:.2}s → {:.2}s  transcript_segments={}  semantics={}  suggestions={}",
                        run.duration,
                        run.edl.output_duration,
                        tr.segments.len(),
                        semantics.len(),
                        suggestions.len()
                    );
                    for w in &plan.warnings {
                        println!("warn: {w}");
                    }
                    println!("  suggestions → {}", sug_path.display());
                    println!("  plan → {}", plan_path.display());
                    println!("  semantics → {}", sem_path.display());
                    println!(
                        "Human: review suggestions JSON, accept in UI, export cut, then render."
                    );
                    ExitCode::SUCCESS
                }
                "render" => {
                    // Apply VisualPlan overlays onto an already-cut longform.
                    let cut = args.get(1).cloned().unwrap_or_default();
                    let plan_path = args.get(2).cloned().unwrap_or_default();
                    let out = args.get(3).cloned().unwrap_or_default();
                    if cut.is_empty() || plan_path.is_empty() || out.is_empty() {
                        eprintln!(
                            "usage: vigilcut-cli visual render <cut.mp4> <plan.json> <out.mp4> [--media source.mp4]"
                        );
                        return ExitCode::FAILURE;
                    }
                    let plan = match vigilcut_lib::pipeline::visual::load_visual_plan(
                        PathBuf::from(&plan_path).as_path(),
                    ) {
                        Ok(p) => p,
                        Err(e) => {
                            eprintln!("plan error: {e}");
                            return ExitCode::FAILURE;
                        }
                    };
                    if plan.placements.is_empty() {
                        eprintln!(
                            "error: VisualPlan has no placements. Accept suggestions in the UI first."
                        );
                        return ExitCode::FAILURE;
                    }
                    let media_ref = arg_value(&args, "--media")
                        .unwrap_or_else(|| plan.media_path.clone());
                    match rt.block_on(
                        vigilcut_lib::pipeline::visual::render::render_visual_plan(
                            PathBuf::from(&cut).as_path(),
                            &plan,
                            PathBuf::from(&out).as_path(),
                            &media_ref,
                        ),
                    ) {
                        Ok(p) => {
                            println!("rendered {}", p.display());
                            ExitCode::SUCCESS
                        }
                        Err(e) => {
                            eprintln!("render error: {e}");
                            ExitCode::FAILURE
                        }
                    }
                }
                _ => {
                    eprintln!(
                        "usage: vigilcut-cli visual <import|list|transcript|scan-missing|enrich|render> ..."
                    );
                    ExitCode::FAILURE
                }
            }
        }
        "help" | "-h" | "--help" => {
            print_help();
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            print_help();
            ExitCode::FAILURE
        }
    }
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(rest) = args[i].strip_prefix(&format!("{flag}=")) {
            return Some(rest.to_string());
        }
        i += 1;
    }
    None
}

fn arg_csv(args: &[String], flag: &str) -> Vec<String> {
    arg_value(args, flag)
        .map(|s| {
            s.split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn policy_from_args(args: &[String]) -> PolicyConfig {
    use vigilcut_lib::models::policy_pack::builtin_policy_packs;
    let mut id = "factory";
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--policy" {
            if let Some(v) = args.get(i + 1) {
                id = v.as_str();
            }
            break;
        }
        if let Some(rest) = args[i].strip_prefix("--policy=") {
            id = rest;
            break;
        }
        i += 1;
    }
    builtin_policy_packs()
        .into_iter()
        .find(|p| p.id == id)
        .map(|p| p.policy)
        .unwrap_or_default()
}

fn print_help() {
    eprintln!(
        "\
VigilCut CLI v1.1 — factory engine (no UI)

  vigilcut-cli analyze <video.mp4> [--policy factory|youtube|podcast|gentle|shorts-first]
  vigilcut-cli export <video.mp4> [out.mp4]
  vigilcut-cli batch <inbox_dir> [outbox_dir] [--policy ...]
  vigilcut-cli clips <video.mp4> [outdir]   # find + export vertical 9:16 candidates

  Visual library (local, no cloud):
  vigilcut-cli visual import <image|folder> [--concepts a,b] [--tags t] [--recursive]
  vigilcut-cli visual list [query]
  vigilcut-cli visual transcript <video.mp4> [outdir] [--srt path] [--whisper]
  vigilcut-cli visual enrich <video.mp4> [outdir] [--srt path] [--whisper]
  vigilcut-cli visual render <cut.mp4> <plan.json> <out.mp4> [--media source.mp4]
  vigilcut-cli visual scan-missing

  Policies: factory (default), youtube, podcast, gentle, shorts-first

Factory dirs (desktop app):
  %APPDATA%/VigilCut/inbox
  %APPDATA%/VigilCut/outbox
  %APPDATA%/VigilCut/library
"
    );
}
