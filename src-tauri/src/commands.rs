use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::db::{ColumnCompleteness, DuplicateEntry, DuckDbManager, OverviewStats, QueryResultPage, ValueFrequency};
use crate::engine::decompressor::open_streaming_reader;
use crate::engine::json_stream::JsonStreamParser;
use crate::engine::progress::ProgressTracker;
use crate::engine::xml_stream::XmlStreamParser;
use crate::engine::{detect_feed_format, FeedFormat};

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
        let format_desc = match detect_feed_format(&filename) {
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
pub async fn generate_demo_feed(record_count: usize) -> Result<SelectedFileInfo, String> {
    use std::io::Write;
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("feedlens_demo_{}.xml", record_count));

    let titles = [
        "Senior Rust Engineer", "Frontend Architect", "Site Reliability Engineer",
        "Data Scientist", "Full Stack Developer", "Machine Learning Specialist",
        "DevOps Engineer", "Product Manager", "Security Analyst", "Cloud Architect"
    ];
    let companies = [
        "Acme Corp", "TechWave", "CloudMatrix", "NovaScale", "CyberDynamics",
        "HyperFlow", "QuantumLogic", "PeakSystems", "OmniData", "Vertex AI"
    ];
    let cities = ["San Francisco", "New York", "London", "Berlin", "Tokyo", "Austin", "Seattle", "Toronto", "Remote"];
    let categories = ["Engineering", "Data & AI", "Product", "Security", "Operations"];
    let job_types = ["Full-Time", "Contract", "Part-Time", "Remote"];

    let file = fs::File::create(&temp_path).map_err(|e| e.to_string())?;
    let mut writer = std::io::BufWriter::with_capacity(1024 * 1024 * 4, file);

    writer.write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<source>\n  <publisher>FeedLens Demo Feed</publisher>\n").map_err(|e| e.to_string())?;

    for i in 1..=record_count {
        let title = titles[i % titles.len()];
        let company = companies[i % companies.len()];
        let city = cities[i % cities.len()];
        let category = categories[i % categories.len()];
        let job_type = job_types[i % job_types.len()];
        let salary_min = 70000 + ((i * 350) % 80000);
        let salary_max = salary_min + 30000;

        // Introduce realistic nulls/empty fields for fill rate testing
        let country_val = if i % 15 == 0 { "" } else { "US" };
        let department = if i % 4 == 0 { "" } else { "Technology" };

        let record_str = format!(
            "  <job>\n    <id>JOB-{:06}</id>\n    <title>{}</title>\n    <company>{}</company>\n    <location>\n      <city>{}</city>\n      <country>{}</country>\n    </location>\n    <category>{}</category>\n    <department>{}</department>\n    <job_type>{}</job_type>\n    <salary_min>{}</salary_min>\n    <salary_max>{}</salary_max>\n    <posted_at>2026-09-{:02}</posted_at>\n    <url>https://jobs.example.com/{:06}</url>\n  </job>\n",
            i, title, company, city, country_val, category, department, job_type, salary_min, salary_max, (i % 28) + 1, i
        );
        writer.write_all(record_str.as_bytes()).map_err(|e| e.to_string())?;
    }

    writer.write_all(b"</source>\n").map_err(|e| e.to_string())?;
    writer.flush().map_err(|e| e.to_string())?;

    let path_str = temp_path.to_string_lossy().to_string();
    let size_bytes = fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0);

    Ok(SelectedFileInfo {
        path: path_str,
        filename: format!("feedlens_demo_{}.xml", record_count),
        size_bytes,
        format: "XML".to_string(),
    })
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
    file_path: String,
    record_tag: Option<String>,
) -> Result<(), String> {
    let path = Path::new(&file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    let total_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    let cancel_flag = Arc::new(AtomicBool::new(false));

    {
        let mut flag_lock = state.current_cancel_flag.lock();
        *flag_lock = Some(cancel_flag.clone());
    }

    let tracker = Arc::new(ProgressTracker::new(total_bytes, cancel_flag.clone()));
    let db = state.db.clone();
    let app_handle = app.clone();
    let last_elapsed_ref = state.last_elapsed.clone();

    // Spawn ingestion thread
    std::thread::spawn(move || {
        let start_time = Instant::now();
        let tracker_clone = tracker.clone();
        let app_handle_progress = app_handle.clone();

        // Spawn progress ticker thread
        let is_running = Arc::new(AtomicBool::new(true));
        let is_running_ticker = is_running.clone();
        let ticker_tracker = tracker.clone();

        std::thread::spawn(move || {
            while is_running_ticker.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(150));
                let progress = ticker_tracker.snapshot("indexing", "Processing feed records...", None);
                let _ = app_handle_progress.emit("ingest-progress", &progress);
            }
        });

        let ingest_result = (|| -> anyhow::Result<()> {
            let reader = open_streaming_reader(&file_path, &tracker_clone)?;
            let format = detect_feed_format(&file_path);

            let mut batch = Vec::with_capacity(10_000);
            let mut is_first_batch = true;

            match format {
                FeedFormat::Xml | FeedFormat::Auto => {
                    let mut xml_parser = XmlStreamParser::new(BufReader::new(reader), record_tag);
                    while let Some(rec) = xml_parser.next_record()? {
                        if tracker_clone.is_cancelled() {
                            break;
                        }

                        batch.push(rec.fields);
                        tracker_clone.records_ingested.fetch_add(1, Ordering::Relaxed);

                        if batch.len() >= 10_000 {
                            if is_first_batch {
                                let initial_cols: Vec<String> = batch[0].keys().cloned().collect();
                                db.init_table(&initial_cols)?;
                                is_first_batch = false;
                            }
                            db.insert_batch(&batch)?;
                            batch.clear();
                        }
                    }
                }
                FeedFormat::Json => {
                    let mut json_parser = JsonStreamParser::new(BufReader::new(reader));
                    while let Some(rec) = json_parser.next_record()? {
                        if tracker_clone.is_cancelled() {
                            break;
                        }

                        batch.push(rec.fields);
                        tracker_clone.records_ingested.fetch_add(1, Ordering::Relaxed);

                        if batch.len() >= 10_000 {
                            if is_first_batch {
                                let initial_cols: Vec<String> = batch[0].keys().cloned().collect();
                                db.init_table(&initial_cols)?;
                                is_first_batch = false;
                            }
                            db.insert_batch(&batch)?;
                            batch.clear();
                        }
                    }
                }
            }

            // Flush remaining records
            if !batch.is_empty() {
                if is_first_batch {
                    let initial_cols: Vec<String> = batch[0].keys().cloned().collect();
                    db.init_table(&initial_cols)?;
                }
                db.insert_batch(&batch)?;
            }

            Ok(())
        })();

        is_running.store(false, Ordering::Relaxed);
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
                    let _ = app_handle.emit("ingest-progress", &final_progress);
                    let _ = app_handle.emit("ingest-complete", ());
                }
            }
            Err(e) => {
                let err_msg = e.to_string();
                let err_progress = tracker_clone.snapshot("error", "Error during ingestion", Some(err_msg.clone()));
                let _ = app_handle.emit("ingest-progress", &err_progress);
                let _ = app_handle.emit("ingest-error", &err_msg);
            }
        }
    });

    Ok(())
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
