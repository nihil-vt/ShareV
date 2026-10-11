pub mod capture;
pub mod cli;
pub mod clipboard;
pub mod commands;
pub mod config;
pub mod error;
pub mod history;
pub mod id;
pub mod paths;
pub mod pipeline;
pub mod platform;
pub mod record;
pub mod shortcuts;
pub mod tray;
pub mod upload;

use std::sync::Mutex;

use tauri::Manager;

use config::Config;

/// Shared application state managed by Tauri.
pub struct AppState {
    pub config: Mutex<Config>,
    /// The in-progress recording, if any.
    pub recorder: Mutex<Option<record::ActiveRecording>>,
}

pub fn load_config() -> Config {
    Config::load().unwrap_or_else(|err| {
        log::warn!("could not load config, using defaults: {err}");
        Config::default()
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState {
            config: Mutex::new(load_config()),
            recorder: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::capture,
            commands::diagnostics,
            commands::list_history,
            commands::delete_history,
            commands::clear_history,
            commands::open_path,
            commands::reveal_path,
            commands::copy_file_to_clipboard,
            commands::copy_text,
            commands::pick_directory,
            commands::read_image_base64,
            commands::apply_edits,
            commands::copy_edits,
            commands::upload_path,
            commands::start_recording,
            commands::stop_recording,
            commands::recording_status,
            commands::list_audio_devices,
            commands::is_flatpak,
        ])
        .setup(|app| {
            let handle = app.handle().clone();

            if let Err(err) = tray::setup(&handle) {
                log::warn!("tray unavailable: {err}");
            }

            let config = handle.state::<AppState>().config.lock().unwrap().clone();
            shortcuts::reload(&handle, &config);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ShareV");
}
