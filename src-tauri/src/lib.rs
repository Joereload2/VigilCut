mod commands;
mod error;
pub mod ffmpeg;
mod job_control;
pub mod models;
pub mod pipeline;
mod state;

use commands::analyze::AnalysisCache;
use commands::clipping::ClippingCache;
use commands::visual::VisualSessionState;
use commands::watch::InboxWatchState;
use job_control::JobControl;
use pipeline::visual::VisualSession;
use state::AppState;
use tauri::Manager;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vigilcut=info,tauri=warn".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(AppState::default())
        .manage(AnalysisCache::default())
        .manage(ClippingCache::default())
        .manage(InboxWatchState::default())
        .manage(JobControl::default())
        .manage(Mutex::new(VisualSession::default()) as VisualSessionState)
        .invoke_handler(tauri::generate_handler![
            // System
            commands::system::get_app_info,
            commands::system::check_ffmpeg,
            commands::system::get_workspace_paths,
            commands::system::cancel_job,
            // Media
            commands::media::probe_media,
            commands::media::extract_waveform,
            commands::media::generate_thumbnail,
            // Analysis engine (events + policy + EDL)
            commands::analyze::run_analysis,
            commands::analyze::get_analysis_run,
            commands::analyze::resolve_analysis_exception,
            commands::analyze::resolve_all_exceptions,
            // Analysis (legacy silence API — still works)
            commands::vad::detect_silences,
            commands::vad::analyze_speech_segments,
            // Project
            commands::project::create_project,
            commands::project::load_project,
            commands::project::save_project,
            commands::project::list_recent_projects,
            // Timeline / segments
            commands::timeline::apply_segment_edits,
            commands::timeline::merge_adjacent_segments,
            commands::timeline::split_segment_at,
            // Export
            commands::export::export_video,
            commands::export::preview_skip_cuts,
            commands::export::estimate_export,
            // Presets & batch
            commands::presets::list_presets,
            commands::presets::save_preset,
            commands::presets::delete_preset,
            commands::batch::queue_batch_job,
            commands::batch::get_batch_status,
            commands::batch::list_batch_jobs,
            commands::batch::queue_inbox_batch,
            // Factory paths / multi-artifact
            commands::factory::get_factory_paths,
            commands::factory::write_export_artifacts,
            commands::factory::open_factory_folder,
            commands::factory::list_policy_packs,
            commands::factory::get_policy_pack,
            commands::factory::save_policy_pack,
            // Inbox watch
            commands::watch::start_inbox_watch,
            commands::watch::stop_inbox_watch,
            commands::watch::get_inbox_watch_status,
            commands::watch::process_factory_inbox_now,
            // Intelligent clipping
            commands::clipping::run_clipping,
            commands::clipping::get_clipping_run,
            commands::clipping::update_clip_status,
            commands::clipping::update_clip_span,
            commands::clipping::update_clip_framing,
            commands::clipping::bulk_clip_status,
            commands::clipping::export_clips,
            commands::clipping::export_single_clip,
            commands::clipping::promote_clip_variant,
            // Future-ready stubs
            commands::audio::enhance_audio_preview,
            commands::color::analyze_color_stats,
            commands::subtitles::import_subtitles,
            commands::subtitles::generate_subtitles_whisper,
            // Visual library + transcript enrichment
            commands::visual::visual_run_enrichment,
            commands::visual::visual_transcribe_whisper,
            commands::visual::visual_whisper_status,
            commands::visual::visual_install_whisper,
            commands::visual::visual_list_assets,
            commands::visual::visual_import_image,
            commands::visual::visual_attach_image,
            commands::visual::visual_import_folder,
            commands::visual::visual_update_asset,
            commands::visual::visual_list_usage,
            commands::visual::visual_scan_missing,
            commands::visual::visual_set_suggestion_status,
            commands::visual::visual_get_session,
            commands::visual::visual_check_edl,
            commands::visual::visual_export_transcript,
            commands::visual::visual_save_plan,
            commands::visual::visual_load_plan,
            commands::visual::visual_render_plan,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = handle.state::<AppState>();
            if let Err(e) = state.ensure_dirs() {
                tracing::warn!("Could not create app directories: {e}");
            }
            tracing::info!("VigilCut started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running VigilCut");
}
