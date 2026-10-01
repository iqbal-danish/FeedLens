pub mod db;
pub mod engine;
pub mod commands;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use parking_lot::Mutex;
use commands::AppState;
use db::DuckDbManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = Arc::new(DuckDbManager::new().expect("Failed to initialize DuckDB engine"));
    let state = AppState {
        db,
        current_cancel_flag: Arc::new(Mutex::new(None)),
        last_elapsed: Arc::new(Mutex::new(0.0)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes = include_bytes!("../icons/128x128.png");
                if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                    let _ = window.set_icon(icon);
                }
            }
            Ok(())
        })
        .on_window_event(|_window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Ensure immediate clean exit of all background threads and webview sub-processes
                std::process::exit(0);
            }
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::pick_feed_file,
            commands::pick_export_file,
            commands::start_ingestion,
            commands::cancel_ingestion,
            commands::get_recent_feeds,
            commands::delete_recent_feed,
            commands::load_recent_feed,
            commands::export_column_frequency,
            commands::get_overview_stats,
            commands::get_completeness_matrix,
            commands::get_column_distribution,
            commands::get_duplicate_records,
            commands::query_records,
            commands::export_records,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
