use std::fs::{self, File};
use std::io::{BufReader, Cursor, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::db::{
    delete_recent_feed_entry, get_feedlens_data_dir, load_recent_catalog, save_recent_feed_entry,
    ColumnCompleteness, DuplicateEntry, DuckDbManager, OverviewStats, QueryResultPage, RecentFeed,
    ValueFrequency,
};
use crate::engine::decompressor::{open_streaming_reader, open_streaming_url_reader};
use crate::engine::json_stream::JsonStreamParser;
use crate::engine::progress::ProgressTracker;
use crate::engine::xml_stream::{detect_xml_record_tag, XmlStreamParser};
use crate::engine::{detect_feed_format, sniff_format_from_bytes, FeedFormat};

fn format_size_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Serialize, Deserialize)]
pub struct SelectedFileInfo {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub format: String,
}

pub struct AppState {
    pub db: Arc<DuckDbManager>,
    pub current_cancel_flag: Arc<Mutex<Option<Arc<AtomicBool>>>>,
    pub last_elapsed: Arc<Mutex<f64>>,
}

#[tauri::command]
pub async fn pick_feed_file() -> Result<Option<SelectedFileInfo>, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter(
            "Feeds & Archives",
            &["xml", "json", "jsonl", "ndjson", "gz", "zip", "bz2", "tgz", "tar"],
        )
        .add_filter("XML Files", &["xml", "xml.gz"])
        .add_filter("JSON Files", &["json", "jsonl", "ndjson", "json.gz"])
        .add_filter("All Files", &["*"])
        .pick_file()
        .await;

    if let Some(handle) = file {
        let path = handle.path().to_string_lossy().to_string();
        let filename = handle.file_name();
        let size_bytes = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        // Sniff content directly from the file header
        let sniffed = if let Ok(mut f) = fs::File::open(&path) {
            let mut buf = [0u8; 2048];
            let n = f.read(&mut buf).unwrap_or(0);
            sniff_format_from_bytes(&buf[..n])
        } else {
            FeedFormat::Auto
        };

        let format = if sniffed != FeedFormat::Auto {
            sniffed
        } else {
            detect_feed_format(&filename)
        };

        let format_desc = match format {
            FeedFormat::Xml => "XML",
            FeedFormat::Json => "JSON",
            FeedFormat::Auto => "Unknown",
        };

        Ok(Some(SelectedFileInfo {
            path,
            filename,
            size_bytes,
            format: format_desc.to_string(),
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn pick_export_file(default_name: String, is_json: bool) -> Result<Option<String>, String> {
    let ext = if is_json { "json" } else { "csv" };
    let desc = if is_json { "JSON Files" } else { "CSV Files" };

    let file = rfd::AsyncFileDialog::new()
        .set_file_name(&default_name)
        .add_filter(desc, &[ext])
        .save_file()
        .await;

    Ok(file.map(|h| h.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn cancel_ingestion(state: State<'_, AppState>) -> Result<(), String> {
    let cancel = state.current_cancel_flag.lock();
    if let Some(ref flag) = *cancel {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub async fn start_ingestion(
    app: AppHandle,
    state: State<'_, AppState>,
    source_type: String,
    file_path: String,
    record_tag: Option<String>,
    ingestion_mode: Option<String>,
) -> Result<(), String> {
    let is_url = source_type == "url";
    let mut total_bytes = 0u64;

    if !is_url {
        let path = Path::new(&file_path);
        if !path.exists() {
            return Err(format!("File does not exist: {}", file_path));
        }
        total_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }

    let cancel_flag = Arc::new(AtomicBool::new(false));

    {
        let mut flag_lock = state.current_cancel_flag.lock();
        *flag_lock = Some(cancel_flag.clone());
    }

    let tracker = Arc::new(ProgressTracker::new(total_bytes, cancel_flag.clone()));
    let db = state.db.clone();
    let app_handle = app.clone();
    let last_elapsed_ref = state.last_elapsed.clone();
    let skip_descriptions = ingestion_mode.as_deref() != Some("full");
    let batch_size = match ingestion_mode.as_deref() {
        Some("extreme") => 20_000,
        Some("fast") => 10_000,
        _ => 5_000,
    };

    // Spawn ingestion background thread
    std::thread::spawn(move || {
        let start_time = Instant::now();
        let tracker_clone = tracker.clone();
        let app_handle_progress = app_handle.clone();

        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_ticker = is_running.clone();
        let ticker_tracker = tracker.clone();

        // High-frequency UI progress reporter
        let ticker_handle = std::thread::spawn(move || {
            while is_running_ticker.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(150));
                if !is_running_ticker.load(Ordering::Relaxed) {
                    break;
                }
                let progress = ticker_tracker.snapshot("indexing", "Processing feed records...", None);
                let _ = app_handle_progress.emit("ingest-progress", &progress);
            }
        });

        let mut discovered_col_count = 0usize;

        let ingest_result = (|| -> anyhow::Result<()> {
            let (reader, stream_total_bytes): (Box<dyn std::io::Read + Send>, u64) = if is_url {
                let (r, cl) = open_streaming_url_reader(&file_path, &tracker_clone)?;
                (r, cl)
            } else {
                let r = open_streaming_reader(&file_path, &tracker_clone)?;
                (r, total_bytes)
            };

            if stream_total_bytes > 0 {
                tracker_clone.total_bytes.store(stream_total_bytes, Ordering::Relaxed);
            }

            // Pre-read first 512KB into buffer to sniff format and detect repeating tags
            let mut pre_buffer = Vec::with_capacity(512 * 1024);
            let mut chunk = [0u8; 8192];
            let mut reader = reader;
            while pre_buffer.len() < 512 * 1024 {
                if tracker_clone.is_cancelled() {
                    break;
                }
                let n = reader.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                pre_buffer.extend_from_slice(&chunk[..n]);
            }

            if pre_buffer.is_empty() {
                anyhow::bail!("Feed source is empty (0 bytes received).");
            }

            let sniffed = sniff_format_from_bytes(&pre_buffer);
            let format = if sniffed != FeedFormat::Auto {
                println!("FeedLens: Format detected by content sniffing: {:?}", sniffed);
                sniffed
            } else {
                let fallback = detect_feed_format(&file_path);
                println!("FeedLens: Format fallback by filename: {:?}", fallback);
                fallback
            };

            let chained = Cursor::new(pre_buffer.clone()).chain(reader);
            let buf_reader = BufReader::with_capacity(1024 * 1024 * 4, chained);

            match format {
                FeedFormat::Xml | FeedFormat::Auto => {
                    // Step 1: Detect repeating record tag accurately
                    let target_tag = match record_tag {
                        Some(ref t) if !t.is_empty() && t != "Auto" => t.clone(),
                        _ => {
                            match detect_xml_record_tag(Cursor::new(&pre_buffer), pre_buffer.len()) {
                                Ok(tag) => {
                                    println!("FeedLens: Detected repeating XML tag from stream: <{}>", tag);
                                    tag
                                }
                                Err(_) => {
                                    if !is_url {
                                        let initial_file = File::open(&file_path)?;
                                        detect_xml_record_tag(initial_file, 5 * 1024 * 1024).unwrap_or_else(|_| "job".to_string())
                                    } else {
                                        "job".to_string()
                                    }
                                }
                            }
                        }
                    };

                    println!("FeedLens: Selected XML tag: <{}>, skip_descriptions: {}", target_tag, skip_descriptions);

                    let mut xml_parser = XmlStreamParser::new(buf_reader, &target_tag, skip_descriptions);

                    let mut initial_batch = Vec::with_capacity(10_000);
                    let mut all_discovered_columns = std::collections::BTreeSet::new();

                    // Read first 10,000 records to discover complete schema upfront
                    while initial_batch.len() < 10_000 {
                        if tracker_clone.is_cancelled() {
                            break;
                        }
                        if let Some(rec) = xml_parser.next_record()? {
                            for k in rec.fields.keys() {
                                all_discovered_columns.insert(k.clone());
                            }
                            initial_batch.push(rec.fields);
                            tracker_clone.records_ingested.fetch_add(1, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    }

                    if initial_batch.is_empty() {
                        return Err(anyhow::anyhow!(
                            "No records matching tag <{}> found in XML feed. If this feed uses a different tag, please specify it in the Job Element Tag / Path field.",
                            target_tag
                        ));
                    }

                    let cols: Vec<String> = all_discovered_columns.into_iter().collect();
                    discovered_col_count = cols.len();
                    db.init_table(&cols)?;
                    db.insert_batch(&initial_batch)?;
                    initial_batch.clear();

                    // Stream remaining records in high-throughput dynamic batches
                    let mut batch = Vec::with_capacity(batch_size);
                    while let Some(rec) = xml_parser.next_record()? {
                        if tracker_clone.is_cancelled() {
                            break;
                        }
                        batch.push(rec.fields);
                        tracker_clone.records_ingested.fetch_add(1, Ordering::Relaxed);

                        if batch.len() >= batch_size {
                            db.insert_batch(&batch)?;
                            batch.clear();
                        }
                    }

                    if !batch.is_empty() {
                        db.insert_batch(&batch)?;
                    }
                }
                FeedFormat::Json => {
                    let mut json_parser = JsonStreamParser::new(
                        buf_reader,
                        skip_descriptions,
                        record_tag.clone(),
                    );

                    let mut initial_batch = Vec::with_capacity(10_000);
                    let mut all_discovered_columns = std::collections::BTreeSet::new();

                    while initial_batch.len() < 10_000 {
                        if tracker_clone.is_cancelled() {
                            break;
                        }
                        if let Some(rec) = json_parser.next_record()? {
                            for k in rec.fields.keys() {
                                all_discovered_columns.insert(k.clone());
                            }
                            initial_batch.push(rec.fields);
                            tracker_clone.records_ingested.fetch_add(1, Ordering::Relaxed);
                        } else {
                            break;
                        }
                    }

                    if initial_batch.is_empty() {
                        return Err(anyhow::anyhow!("No records could be extracted from JSON feed. Please check file format or record path."));
                    }

                    let cols: Vec<String> = all_discovered_columns.into_iter().collect();
                    discovered_col_count = cols.len();
                    db.init_table(&cols)?;
                    db.insert_batch(&initial_batch)?;
                    initial_batch.clear();

                    let mut batch = Vec::with_capacity(batch_size);
                    while let Some(rec) = json_parser.next_record()? {
                        if tracker_clone.is_cancelled() {
                            break;
                        }
                        batch.push(rec.fields);
                        tracker_clone.records_ingested.fetch_add(1, Ordering::Relaxed);

                        if batch.len() >= batch_size {
                            db.insert_batch(&batch)?;
                            batch.clear();
                        }
                    }

                    if !batch.is_empty() {
                        db.insert_batch(&batch)?;
                    }
                }
            }

            Ok(())
        })();

        is_running.store(false, Ordering::SeqCst);
        let _ = ticker_handle.join();
        let elapsed = start_time.elapsed().as_secs_f64();
        *last_elapsed_ref.lock() = elapsed;

        match ingest_result {
            Ok(()) => {
                if tracker_clone.is_cancelled() {
                    let final_progress = tracker_clone.snapshot("cancelled", "Ingestion cancelled", None);
                    let _ = app_handle.emit("ingest-progress", &final_progress);
                } else {
                    let mut final_progress = tracker_clone.snapshot("ready", "Completed indexing", None);
                    final_progress.percentage = 100.0;

                    // Automatically persist database and update recent feeds catalog
                    let total_records = tracker_clone.records_ingested.load(Ordering::Relaxed);
                    let task_id = format!("feed_{}", chrono::Utc::now().timestamp_millis());
                    let db_file_name = format!("{}.duckdb", task_id);
                    let data_dir = get_feedlens_data_dir();
                    let db_file_path = data_dir.join(&db_file_name).to_string_lossy().to_string();

                    if let Err(e) = db.persist_to_file(&db_file_path) {
                        eprintln!("Warning: failed to persist duckdb file: {}", e);
                    }

                    let filename = if is_url {
                        file_path.split('/').last().unwrap_or("feed.xml").to_string()
                    } else {
                        Path::new(&file_path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "feed.xml".to_string())
                    };

                    let file_size_str = if total_bytes > 0 {
                        format_size_bytes(total_bytes)
                    } else {
                        format_size_bytes(tracker_clone.bytes_read.load(Ordering::Relaxed))
                    };

                    let recent_entry = RecentFeed {
                        id: task_id,
                        filename,
                        source_type: if is_url { "url".to_string() } else { "file".to_string() },
                        file_size: file_size_str,
                        total_records,
                        column_count: discovered_col_count,
                        db_path: db_file_path,
                        analyzed_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    };
                    let _ = save_recent_feed_entry(recent_entry);

                    let _ = app_handle.emit("ingest-progress", &final_progress);
                    let _ = app_handle.emit("ingest-complete", ());
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                eprintln!("Ingestion error: {}", err_msg);
                let err_progress = tracker_clone.snapshot("error", "Error during ingestion", Some(err_msg.clone()));
                let _ = app_handle.emit("ingest-progress", &err_progress);
                let _ = app_handle.emit("ingest-error", &err_msg);
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn get_recent_feeds() -> Result<Vec<RecentFeed>, String> {
    Ok(load_recent_catalog())
}

#[tauri::command]
pub async fn delete_recent_feed(task_id: String) -> Result<(), String> {
    delete_recent_feed_entry(&task_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_recent_feed(state: State<'_, AppState>, task_id: String) -> Result<OverviewStats, String> {
    let catalog = load_recent_catalog();
    let feed = catalog
        .iter()
        .find(|f| f.id == task_id)
        .ok_or_else(|| format!("Feed {} not found in recent catalog", task_id))?;
    state.db.load_from_db(&feed.db_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_column_frequency(
    state: State<'_, AppState>,
    column_name: String,
    export_path: String,
) -> Result<u64, String> {
    state
        .db
        .export_column_frequency(&column_name, &export_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_overview_stats(state: State<'_, AppState>) -> Result<OverviewStats, String> {
    let elapsed = *state.last_elapsed.lock();
    state.db.get_overview(elapsed).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_completeness_matrix(state: State<'_, AppState>) -> Result<Vec<ColumnCompleteness>, String> {
    state.db.get_completeness_matrix().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_column_distribution(
    state: State<'_, AppState>,
    column_name: String,
    limit: usize,
) -> Result<Vec<ValueFrequency>, String> {
    state
        .db
        .get_column_distribution(&column_name, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_duplicate_records(
    state: State<'_, AppState>,
    column_name: String,
    limit: usize,
) -> Result<Vec<DuplicateEntry>, String> {
    state
        .db
        .get_duplicates(&column_name, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn query_records(
    state: State<'_, AppState>,
    page: usize,
    page_size: usize,
    sort_col: Option<String>,
    sort_desc: bool,
    filter_text: Option<String>,
) -> Result<QueryResultPage, String> {
    state
        .db
        .query_records(page, page_size, sort_col, sort_desc, filter_text)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn export_records(
    state: State<'_, AppState>,
    export_path: String,
    is_json: bool,
    filter_text: Option<String>,
) -> Result<u64, String> {
    state
        .db
        .export_records(&export_path, is_json, filter_text)
        .map_err(|e| e.to_string())
}
