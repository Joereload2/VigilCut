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
use vigilcut_lib::pipeline::engine::run_silence_analysis;
use vigilcut_lib::models::batch::BatchJob;
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
            let job = BatchJob::new(paths, "cli".into(), outbox, true);
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
            // analyze + force exceptions + export single file
            let path = args.first().cloned().unwrap_or_default();
            let out = args.get(1).cloned();
            if path.is_empty() {
                eprintln!("usage: vigilcut-cli export <video> [out.mp4]");
                return ExitCode::FAILURE;
            }
            let media = PathBuf::from(&path);
            let out_dir = out
                .as_ref()
                .map(|p| {
                    let pb = PathBuf::from(p);
                    pb.parent()
                        .filter(|par| !par.as_os_str().is_empty())
                        .map(|par| par.to_path_buf())
                        .unwrap_or_else(|| PathBuf::from("."))
                })
                .unwrap_or_else(|| media.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from(".")));
            let result = rt.block_on(process_one_file(
                &media,
                &out_dir,
                &PolicyConfig::default(),
                true,
                &ExportOptions::default(),
                &ColorOptions::default(),
            ));
            if result.ok {
                println!("exported {}", result.output_path.as_deref().unwrap_or("?"));
                ExitCode::SUCCESS
            } else {
                eprintln!("{}", result.error.unwrap_or_else(|| "export failed".into()));
                ExitCode::FAILURE
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
VigilCut CLI v1.0 — factory engine (no UI)

  vigilcut-cli analyze <video.mp4> [--policy factory|youtube|podcast|gentle|shorts-first]
  vigilcut-cli export <video.mp4> [out.mp4]
  vigilcut-cli batch <inbox_dir> [outbox_dir] [--policy ...]

  Policies: factory (default), youtube, podcast, gentle, shorts-first

Factory dirs (desktop app):
  %APPDATA%/VigilCut/inbox
  %APPDATA%/VigilCut/outbox
"
    );
}
