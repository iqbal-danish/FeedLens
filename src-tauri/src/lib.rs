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
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::pick_feed_file,
            commands::generate_demo_feed,
            commands::pick_export_file,
            commands::start_ingestion,
            commands::cancel_ingestion,
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
