mod commands;
mod error;
pub mod ffmpeg;
mod job_control;
pub mod models;
pub mod pipeline;
mod state;
pub mod visual_library;

use commands::analyze::AnalysisCache;
use commands::clipping::ClippingCache;
use commands::visual::VisualSessionState;
use commands::watch::InboxWatchState;
use job_control::JobControl;
use pipeline::visual::VisualSession;
use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

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
            commands::visual::visual_create_manual_placement,
            commands::visual::visual_update_placement,
            commands::visual::visual_snap_placement,
            commands::visual::visual_evaluate_composition,
            commands::visual::visual_remove_placement,
            commands::visual::visual_add_protected_range,
            commands::visual::visual_remove_protected_range,
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
            // Intelligent visual library
            commands::visual_intel::visual_seed_theme_economy,
            commands::visual_intel::visual_list_concepts,
            commands::visual_intel::visual_create_concept,
            commands::visual_intel::visual_detect_needs,
            commands::visual_intel::visual_list_needs,
            commands::visual_intel::visual_coverage,
            commands::visual_intel::visual_skip_need,
            commands::visual_intel::visual_cover_needs,
            commands::visual_intel::visual_list_review_queue,
            commands::visual_intel::visual_approve_candidate,
            commands::visual_intel::visual_reject_candidate,
            commands::visual_intel::visual_apply_needs_to_plan,
            commands::visual_intel::visual_probe_image_provider,
            commands::visual_intel::visual_cost_policy,
            commands::visual_intel::visual_library_dashboard,
            commands::visual_intel::visual_library_concept_coverage,
            commands::visual_intel::visual_library_create_request,
            commands::visual_intel::visual_library_preview_request,
            commands::visual_intel::visual_library_confirm_request,
            commands::visual_intel::visual_library_list_requests,
            commands::visual_intel::visual_library_regenerate_request,
            commands::visual_intel::visual_library_use_existing,
            commands::visual_intel::visual_library_cancel_request,
            commands::visual_intel::visual_match_need,
            commands::visual_intel::visual_supervision,
            commands::visual_intel::visual_search_library_for_need,
            commands::visual_intel::visual_assign_need_asset,
            commands::visual_intel::visual_use_asset_for_need,
            commands::visual_intel::visual_generate_need,
            commands::visual_intel::visual_cancel_job,
            commands::visual_intel::visual_regenerate_need,
            commands::visual_intel::visual_supervision_global,
            commands::visual_intel::visual_approve_and_use,
            commands::visual_intel::visual_daily_feed_settings,
            commands::visual_intel::visual_daily_feed_set_enabled,
            commands::visual_intel::visual_daily_feed_cycle,
            commands::visual_intel::visual_daily_week_summary,
            visual_library::commands::sync_commands::library_sync_status,
            visual_library::commands::sync_commands::library_sync_health_check,
            visual_library::commands::sync_commands::library_sync_enqueue_asset,
            visual_library::commands::sync_commands::library_sync_run_once,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let state = handle.state::<AppState>();
            if let Err(e) = state.ensure_dirs() {
                tracing::warn!("Could not create app directories: {e}");
            }
            // Resident generation supervisor (Codex CRIT-001): queue + daily without UI ticks
            pipeline::visual::generation::supervisor::ensure_started();
            tracing::info!("VigilCut started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running VigilCut");
}
