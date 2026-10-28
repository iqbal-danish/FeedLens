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

                #[cfg(target_os = "windows")]
                {
                    if let Ok(hwnd) = window.hwnd() {
                        #[link(name = "dwmapi")]
                        extern "system" {
                            fn DwmSetWindowAttribute(
                                hwnd: isize,
                                dw_attribute: u32,
                                pv_attribute: *const std::ffi::c_void,
                                cb_attribute: u32,
                            ) -> i32;
                        }
                        unsafe {
                            // DWMWA_CAPTION_COLOR = 35 (Windows 11 build 22000+)
                            // COLORREF 0x00BBGGRR for #0b101c -> 0x001C100B
                            let caption_color: u32 = 0x001C100B;
                            let _ = DwmSetWindowAttribute(
                                hwnd.0 as isize,
                                35,
                                &caption_color as *const _ as *const std::ffi::c_void,
                                std::mem::size_of::<u32>() as u32,
                            );

                            // DWMWA_TEXT_COLOR = 36
                            // COLORREF 0x00BBGGRR for #f1f5f9 -> 0x00F9F5F1
                            let text_color: u32 = 0x00F9F5F1;
                            let _ = DwmSetWindowAttribute(
                                hwnd.0 as isize,
                                36,
                                &text_color as *const _ as *const std::ffi::c_void,
                                std::mem::size_of::<u32>() as u32,
                            );
                        }
                    }
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
            commands::get_cross_tabulation,
            commands::export_cross_tabulation,
            commands::export_breakdown_pairs,
            commands::export_custom_columns,
            commands::get_overview_stats,
            commands::get_completeness_matrix,
            commands::get_column_distribution,
            commands::get_duplicate_records,
            commands::query_records,
            commands::export_records,
            commands::get_cli_file,
            commands::window_minimize,
            commands::window_toggle_maximize,
            commands::window_close,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
