mod commands;
mod error;
mod ffmpeg;
mod models;
mod pipeline;
mod state;

use state::AppState;
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
        .invoke_handler(tauri::generate_handler![
            // System
            commands::system::get_app_info,
            commands::system::check_ffmpeg,
            commands::system::get_workspace_paths,
            // Media
            commands::media::probe_media,
            commands::media::extract_waveform,
            commands::media::generate_thumbnail,
            // Analysis (VAD / silence)
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
            // Future-ready stubs
            commands::audio::enhance_audio_preview,
            commands::color::analyze_color_stats,
            commands::subtitles::import_subtitles,
            commands::subtitles::generate_subtitles_whisper,
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
