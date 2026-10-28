use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use anyhow::Result;
use duckdb::Connection;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub mod query_parser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnCompleteness {
    pub column_name: String,
    pub original_name: String,
    pub total_records: u64,
    pub non_null_count: u64,
    pub empty_count: u64,
    pub valid_count: u64,
    pub fill_rate: f64,
    pub unique_count: u64,
    pub sample_values: Vec<String>,
    pub min_length: u64,
    pub max_length: u64,
    pub avg_length: f64,
    pub inferred_type: String,
    pub is_candidate_key: bool,
    pub has_collision_risk: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueFrequency {
    pub value: String,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateEntry {
    pub key: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewStats {
    pub total_records: u64,
    pub column_count: usize,
    pub columns: Vec<String>,
    pub elapsed_secs: f64,
    pub records_per_sec: f64,
    pub memory_usage_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResultPage {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, Option<String>>>,
    pub page: usize,
    pub page_size: usize,
    pub total_records: u64,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossTabulationResult {
    pub primary_col: String,
    pub secondary_col: String,
    pub secondary_values: Vec<String>,
    pub rows: Vec<CrossTabulationRow>,
    pub total_primary_uniques: usize,
    pub total_records: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossTabulationRow {
    pub primary_value: String,
    pub counts: HashMap<String, u64>,
    pub total: u64,
}

pub struct DuckDbManager {
    conn: Arc<Mutex<Connection>>,
    known_columns: Arc<Mutex<Vec<String>>>,
    column_mapping: Arc<Mutex<HashMap<String, String>>>, // sanitized -> original
    known_raw_keys: Arc<Mutex<HashSet<String>>>,          // fast set of all seen raw un-sanitized keys
    cached_query_keys: Arc<Mutex<Vec<String>>>,           // pre-mapped original keys in column order for O(1) appender
}

impl DuckDbManager {
    pub fn new() -> Result<Self> {
        let _ = std::fs::create_dir_all(".duckdb_temp");
        let conn = Connection::open_in_memory()?;
        // Optimize DuckDB vector engine for peak uninterrupted throughput
        conn.execute_batch(
            "PRAGMA preserve_insertion_order = false;
             PRAGMA threads = 8;
             PRAGMA memory_limit = '6GB';
             PRAGMA checkpoint_threshold = '10GB';
             PRAGMA wal_autocheckpoint = '10GB';
             PRAGMA temp_directory = '.duckdb_temp';",
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            known_columns: Arc::new(Mutex::new(Vec::new())),
            column_mapping: Arc::new(Mutex::new(HashMap::new())),
            known_raw_keys: Arc::new(Mutex::new(HashSet::new())),
            cached_query_keys: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn sanitize_column_name(name: &str) -> String {
        let mut clean = name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect::<String>()
            .to_lowercase();
        
        if clean.is_empty() {
            clean = "col".to_string();
        }
        if clean.chars().next().unwrap().is_numeric() {
            clean = format!("col_{}", clean);
        }
        clean
    }

    /// Reset database and create table with initial columns
    pub fn init_table(&self, initial_columns: &[String]) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch("DROP TABLE IF EXISTS feed_records;")?;

        let mut sanitized_cols = Vec::new();
        let mut mapping = self.column_mapping.lock();
        mapping.clear();

        let mut raw_keys = self.known_raw_keys.lock();
        raw_keys.clear();

        for col in initial_columns {
            raw_keys.insert(col.clone());
            let sanitized = Self::sanitize_column_name(col);
            if !sanitized_cols.contains(&sanitized) {
                mapping.insert(sanitized.clone(), col.clone());
                sanitized_cols.push(sanitized);
            }
        }

        if sanitized_cols.is_empty() {
            sanitized_cols.push("id".to_string());
            mapping.insert("id".to_string(), "id".to_string());
            raw_keys.insert("id".to_string());
        }

        let col_defs = sanitized_cols
            .iter()
            .map(|c| format!("\"{}\" VARCHAR", c))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!("CREATE TABLE feed_records ({});", col_defs);
        conn.execute_batch(&sql)?;

        let mut cached = self.cached_query_keys.lock();
        *cached = sanitized_cols.iter().map(|c| mapping.get(c).cloned().unwrap_or_else(|| c.clone())).collect();

        let mut known = self.known_columns.lock();
        *known = sanitized_cols;
        Ok(())
    }

    /// Dynamically add new column to table if it doesn't exist
    pub fn ensure_columns(&self, columns: &[String]) -> Result<()> {
        let mut known = self.known_columns.lock();
        let mut mapping = self.column_mapping.lock();
        let mut raw_keys = self.known_raw_keys.lock();
        let conn = self.conn.lock();
        let mut altered = false;

        for col in columns {
            raw_keys.insert(col.clone());
            let sanitized = Self::sanitize_column_name(col);
            if !known.contains(&sanitized) {
                let sql = format!("ALTER TABLE feed_records ADD COLUMN \"{}\" VARCHAR;", sanitized);
                if conn.execute_batch(&sql).is_ok() {
                    mapping.insert(sanitized.clone(), col.clone());
                    known.push(sanitized);
                    altered = true;
                }
            }
        }

        if altered {
            let mut cached = self.cached_query_keys.lock();
            *cached = known.iter().map(|c| mapping.get(c).cloned().unwrap_or_else(|| c.clone())).collect();
        }

        Ok(())
    }

    /// Batch insert records using DuckDB Appender for peak uninterrupted performance
    pub fn insert_batch(&self, records: &[HashMap<String, String>]) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        // Fast zero-allocation check: check if any unknown keys exist in this batch
        let mut new_keys = Vec::new();
        {
            let raw_keys = self.known_raw_keys.lock();
            for rec in records {
                for k in rec.keys() {
                    if !raw_keys.contains(k) {
                        new_keys.push(k.clone());
                    }
                }
            }
        }

        if !new_keys.is_empty() {
            self.ensure_columns(&new_keys)?;
        }

        let cached_keys = self.cached_query_keys.lock().clone();
        let conn = self.conn.lock();

        let mut appender = conn.appender("feed_records")?;
        for rec in records {
            let mut row_slice: Vec<Option<&str>> = Vec::with_capacity(cached_keys.len());
            for orig_key in &cached_keys {
                row_slice.push(rec.get(orig_key).map(|s| s.as_str()));
            }
            appender.append_row(duckdb::appender_params_from_iter(row_slice.into_iter()))?;
        }
        appender.flush()?;
        Ok(())
    }

    /// Get quick overview stats
    pub fn get_overview(&self, elapsed_secs: f64) -> Result<OverviewStats> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM feed_records;")?;
        let total: u64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);

        let cols = self.known_columns.lock().clone();
        let mapping = self.column_mapping.lock().clone();
        let original_cols: Vec<String> = cols.iter().map(|c| mapping.get(c).cloned().unwrap_or(c.clone())).collect();

        let rec_per_sec = if elapsed_secs > 0.05 {
            total as f64 / elapsed_secs
        } else {
            0.0
        };

        Ok(OverviewStats {
            total_records: total,
            column_count: cols.len(),
            columns: original_cols,
            elapsed_secs,
            records_per_sec: rec_per_sec,
            memory_usage_mb: 0.0,
        })
    }

    /// Helper to infer intuitive data types from column name and samples
    pub fn infer_data_type(name: &str, samples: &[String]) -> String {
        let low_name = name.to_lowercase();
        if samples.is_empty() {
            return "Empty".to_string();
        }

        // URL check
        if samples.iter().any(|s| s.starts_with("http://") || s.starts_with("https://")) || low_name.contains("url") || low_name.contains("link") {
            return "URL".to_string();
        }

        // Email check
        if samples.iter().any(|s| s.contains('@') && s.contains('.')) || low_name.contains("email") {
            return "Email".to_string();
        }

        // HTML check
        if samples.iter().any(|s| (s.contains('<') && s.contains('>')) || s.contains("&lt;") || s.contains("<![CDATA[")) || low_name.contains("html") || low_name.contains("description") {
            return "HTML".to_string();
        }

        // Identifier check (check before Date so refcode/job_id/req_id are never misclassified as Date)
        if low_name.contains("id") || low_name.contains("guid") || low_name.contains("code") || low_name.contains("ref") || low_name.contains("key") {
            return "Identifier".to_string();
        }

        // Date check - require dates to actually start with numbers and have valid calendar separators
        let looks_like_date = samples.iter().all(|s| {
            let t = s.trim();
            if t.len() < 8 || t.len() > 30 {
                return false;
            }
            let first_char = t.chars().next().unwrap_or(' ');
            if !first_char.is_ascii_digit() {
                return false;
            }
            let hyphens = t.chars().filter(|&c| c == '-').count();
            let slashes = t.chars().filter(|&c| c == '/').count();
            (hyphens == 2 || slashes == 2) && t.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '/' || c == ':' || c == 'T' || c == 'Z' || c == ' ')
        });

        if looks_like_date || low_name.contains("date") || low_name.contains("time") {
            return "Date".to_string();
        }

        // Boolean check
        if samples.iter().all(|s| s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("false") || s == "0" || s == "1") {
            return "Boolean".to_string();
        }

        // Numeric check
        if samples.iter().all(|s| {
            let clean = s.trim().trim_start_matches(|c| c == '$' || c == '€' || c == '£').replace(',', "");
            clean.parse::<f64>().is_ok()
        }) || low_name.contains("salary") || low_name.contains("price") || low_name.contains("amount") || low_name.contains("count") {
            return "Numeric".to_string();
        }

        "Text".to_string()
    }

    /// Helper to tokenize search input respecting quoted phrases
    pub fn tokenize_search_query(text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in text.chars() {
            if ch == '"' {
                in_quotes = !in_quotes;
            } else if ch.is_whitespace() && !in_quotes {
                if !current.is_empty() {
                    tokens.push(current);
                    current = String::new();
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            tokens.push(current);
        }
        tokens
    }

    /// Helper to resolve target column name with fuzzy matching and common domain aliases
    pub fn resolve_column_name(target: &str, cols: &[String]) -> Option<String> {
        let sanitized = Self::sanitize_column_name(target);
        let target_low = target.to_lowercase();
        let sanitized_low = sanitized.to_lowercase();

        // 1. Exact match (case-insensitive)
        if let Some(c) = cols.iter().find(|c| c.to_lowercase() == target_low || c.to_lowercase() == sanitized_low) {
            return Some(c.clone());
        }

        // 2. Suffix match (e.g. "job_remote" or "job.remote" for "remote")
        if let Some(c) = cols.iter().find(|c| {
            let clow = c.to_lowercase();
            clow.ends_with(&format!("_{}", sanitized_low)) || clow.ends_with(&format!(".{}", sanitized_low))
        }) {
            return Some(c.clone());
        }

        // 3. Known domain aliases for job feeds
        let aliases: &[(&str, &[&str])] = &[
            ("company", &["source_name", "source.name", "name", "brand_name", "company", "company_name", "employer", "brand"]),
            ("company_name", &["source_name", "source.name", "name", "brand_name", "company", "company_name", "employer", "brand"]),
            ("employer", &["source_name", "source.name", "name", "brand_name", "employer", "company", "company_name"]),
            ("name", &["source_name", "source.name", "name", "brand_name", "company", "company_name", "employer"]),
            ("brand", &["brand_name", "source_name", "name", "company"]),
            ("remote", &["remote", "work_model", "is_remote", "telecommute"]),
            ("work_model", &["work_model", "remote", "is_remote"]),
            ("title", &["title", "job_title", "role", "jobtitle", "headline"]),
            ("city", &["city", "job_city", "location", "place"]),
            ("state", &["state", "region", "province"]),
            ("country", &["country", "country_code", "nation"]),
            ("salary", &["salary", "pay", "compensation", "wage"]),
            ("id", &["id", "job_id", "refcode", "jobid", "req_id", "reference"]),
            ("refcode", &["refcode", "id", "job_id", "jobid", "req_id", "reference"]),
        ];

        for (alias_key, candidates) in aliases {
            if target_low == *alias_key {
                for cand in *candidates {
                    if let Some(c) = cols.iter().find(|c| c.to_lowercase() == *cand || c.to_lowercase().ends_with(&format!("_{}", cand))) {
                        return Some(c.clone());
                    }
                }
            }
        }

        None
    }

    /// Helper to build WHERE clause supporting:
    /// - col:val / col=val (e.g. remote:true, company:Balfour, city:London)
    /// - col!=val / -col:val / !col:val
    /// - comparisons: salary:>80k, salary:<50000, date:>=2026-01-01
    /// - ranges: salary:50k..100k, date:2026-01-01..2026-06-30
    /// - boolean logic: AND, OR, NOT, and parentheses (city:Austin OR city:Dallas)
    /// - missing:col / is:empty:col
    /// - populated:col / is:valid:col
    /// - free-text global search across columns
    pub fn build_filter_where_clause(filter_text: &Option<String>, cols: &[String]) -> String {
        query_parser::QueryParser::build_where_clause(filter_text, cols)
    }

    /// Ultra-fast completeness matrix calculation (<150ms on millions of rows)
    pub fn get_completeness_matrix(&self) -> Result<Vec<ColumnCompleteness>> {
        let cols = self.known_columns.lock().clone();
        let mapping = self.column_mapping.lock().clone();
        let conn = self.conn.lock();

        let mut count_stmt = conn.prepare("SELECT COUNT(*) FROM feed_records;")?;
        let total: u64 = count_stmt.query_row([], |row| row.get(0)).unwrap_or(0);

        if total == 0 {
            return Ok(Vec::new());
        }

        let mut results = Vec::new();

        // Process columns in batches of 8 to minimize table scans (e.g., 176 columns -> 22 scans instead of 176)
        for chunk in cols.chunks(8) {
            let mut select_clauses = Vec::with_capacity(chunk.len());
            for col in chunk {
                select_clauses.push(format!(
                    "COUNT(\"{col}\") as non_nulls,
                    COUNT(CASE WHEN \"{col}\" = '' OR TRIM(\"{col}\") = '' THEN 1 END) as empty_str,
                    COUNT(DISTINCT \"{col}\") as uniques,
                    COALESCE(MIN(LENGTH(TRIM(\"{col}\"))), 0) as min_len,
                    COALESCE(MAX(LENGTH(TRIM(\"{col}\"))), 0) as max_len,
                    COALESCE(ROUND(AVG(LENGTH(TRIM(\"{col}\"))), 1), 0.0) as avg_len"
                ));
            }

            let batch_query = format!("SELECT {} FROM feed_records;", select_clauses.join(", "));
            let metrics_row: Vec<(u64, u64, u64, u64, u64, f64)> = if let Ok(mut stmt) = conn.prepare(&batch_query) {
                stmt.query_row([], |row| {
                    let mut vals = Vec::with_capacity(chunk.len());
                    for i in 0..chunk.len() {
                        let base = i * 6;
                        let non_nulls: u64 = row.get(base)?;
                        let empty_str: u64 = row.get(base + 1)?;
                        let uniques: u64 = row.get(base + 2)?;
                        let min_len: u64 = row.get(base + 3)?;
                        let max_len: u64 = row.get(base + 4)?;
                        let avg_len: f64 = row.get(base + 5)?;
                        vals.push((non_nulls, empty_str, uniques, min_len, max_len, avg_len));
                    }
                    Ok(vals)
                }).unwrap_or_else(|_| vec![(0, 0, 0, 0, 0, 0.0); chunk.len()])
            } else {
                vec![(0, 0, 0, 0, 0, 0.0); chunk.len()]
            };

            for (idx, col) in chunk.iter().enumerate() {
                let original_name = mapping.get(col).cloned().unwrap_or_else(|| col.clone());
                let (non_nulls, empty_str, uniques, min_len, max_len, avg_len) = metrics_row.get(idx).copied().unwrap_or((0, 0, 0, 0, 0, 0.0));

                let valid_count = non_nulls.saturating_sub(empty_str);
                let missing_count = total.saturating_sub(valid_count);
                let fill_rate = if total > 0 { (valid_count as f64 / total as f64) * 100.0 } else { 0.0 };

                // Fetch top 10 sample values with index scan
                let sample_query = format!(
                    "SELECT \"{col}\" FROM feed_records WHERE \"{col}\" IS NOT NULL AND TRIM(\"{col}\") != '' LIMIT 10;"
                );
                let samples: Vec<String> = if let Ok(mut sample_stmt) = conn.prepare(&sample_query) {
                    sample_stmt
                        .query_map([], |row| row.get(0))
                        .map(|iter| iter.filter_map(|r| r.ok()).collect())
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };

                let inferred_type = Self::infer_data_type(&original_name, &samples);
                let low_name = original_name.to_lowercase();
                let is_id_name = low_name == "id"
                    || low_name == "refcode"
                    || low_name == "job_id"
                    || low_name == "jobid"
                    || low_name.ends_with(".id")
                    || low_name.ends_with(".refcode")
                    || low_name.ends_with(".job_id")
                    || low_name.contains("reference")
                    || low_name.contains("requisition");

                let is_identifier_like = is_id_name || (inferred_type == "Identifier" && !low_name.contains("country") && !low_name.contains("currency"));

                let is_candidate_key = if is_identifier_like {
                    fill_rate >= 99.0 && (uniques as f64 / total as f64) >= 0.95
                } else if inferred_type != "URL" && inferred_type != "HTML" && inferred_type != "Date" && inferred_type != "Boolean" {
                    valid_count == total && uniques >= total && valid_count > 0
                } else {
                    false
                };

                results.push(ColumnCompleteness {
                    column_name: col.clone(),
                    original_name,
                    total_records: total,
                    non_null_count: non_nulls,
                    empty_count: missing_count,
                    valid_count,
                    fill_rate,
                unique_count: uniques,
                sample_values: samples,
                min_length: min_len,
                max_length: max_len,
                avg_length: avg_len,
                inferred_type,
                    is_candidate_key,
                    has_collision_risk: false,
                });
            }
        }

        // Sort by fill_rate descending
        results.sort_by(|a, b| b.fill_rate.partial_cmp(&a.fill_rate).unwrap_or(std::cmp::Ordering::Equal));
        Ok(results)
    }

    /// Get value frequency distribution for a column
    pub fn get_column_distribution(&self, col_name: &str, limit: usize) -> Result<Vec<ValueFrequency>> {
        let sanitized = Self::sanitize_column_name(col_name);
        let conn = self.conn.lock();

        let mut count_stmt = conn.prepare("SELECT COUNT(*) FROM feed_records;")?;
        let total: u64 = count_stmt.query_row([], |row| row.get(0)).unwrap_or(0);

        if total == 0 {
            return Ok(Vec::new());
        }

        let query = format!(
            "SELECT COALESCE(\"{}\", '[NULL]') as val, COUNT(*) as cnt
             FROM feed_records
             GROUP BY \"{}\"
             ORDER BY cnt DESC
             LIMIT {};",
            sanitized, sanitized, limit
        );

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            let val: String = row.get(0)?;
            let cnt: u64 = row.get(1)?;
            let pct = (cnt as f64 / total as f64) * 100.0;
            Ok(ValueFrequency {
                value: if val.trim().is_empty() { "[EMPTY]".to_string() } else { val },
                count: cnt,
                percentage: pct,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Check for duplicate keys in a column
    pub fn get_duplicates(&self, col_name: &str, limit: usize) -> Result<Vec<DuplicateEntry>> {
        let sanitized = Self::sanitize_column_name(col_name);
        let conn = self.conn.lock();

        let query = format!(
            "SELECT \"{}\" as val, COUNT(*) as cnt
             FROM feed_records
             WHERE \"{}\" IS NOT NULL AND TRIM(\"{}\") != ''
             GROUP BY \"{}\"
             HAVING COUNT(*) > 1
             ORDER BY cnt DESC
             LIMIT {};",
            sanitized, sanitized, sanitized, sanitized, limit
        );

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            Ok(DuplicateEntry {
                key: row.get(0)?,
                count: row.get(1)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    /// Paginated search & sort query
    pub fn query_records(
        &self,
        page: usize,
        page_size: usize,
        sort_col: Option<String>,
        sort_desc: bool,
        filter_text: Option<String>,
    ) -> Result<QueryResultPage> {
        let cols = self.known_columns.lock().clone();
        let mapping = self.column_mapping.lock().clone();
        let conn = self.conn.lock();

        let where_clause = Self::build_filter_where_clause(&filter_text, &cols);

        let count_query = format!("SELECT COUNT(*) FROM feed_records {};", where_clause);
        let mut count_stmt = conn.prepare(&count_query)?;
        let total_records: u64 = count_stmt.query_row([], |row| row.get(0)).unwrap_or(0);

        let order_clause = if let Some(ref sc) = sort_col {
            let sanitized = Self::sanitize_column_name(sc);
            let dir = if sort_desc { "DESC" } else { "ASC" };
            format!("ORDER BY \"{}\" {} NULLS LAST", sanitized, dir)
        } else {
            "".to_string()
        };

        let offset = page.saturating_sub(1) * page_size;
        let select_cols = cols.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ");

        let data_query = format!(
            "SELECT {} FROM feed_records {} {} LIMIT {} OFFSET {};",
            select_cols, where_clause, order_clause, page_size, offset
        );

        let mut data_stmt = conn.prepare(&data_query)?;
        let rows_iter = data_stmt.query_map([], |row| {
            let mut map = HashMap::new();
            for (idx, col) in cols.iter().enumerate() {
                let val: Option<String> = row.get(idx).ok();
                let original_key = mapping.get(col).cloned().unwrap_or_else(|| col.clone());
                map.insert(original_key, val);
            }
            Ok(map)
        })?;

        let mut rows = Vec::new();
        for r in rows_iter {
            rows.push(r?);
        }

        let total_pages = if page_size > 0 {
            ((total_records as f64 / page_size as f64).ceil() as usize).max(1)
        } else {
            1
        };

        let original_cols: Vec<String> = cols.iter().map(|c| mapping.get(c).cloned().unwrap_or(c.clone())).collect();

        Ok(QueryResultPage {
            columns: original_cols,
            rows,
            page,
            page_size,
            total_records,
            total_pages,
        })
    }

    /// Fast native export to CSV or JSON using DuckDB's vectorized COPY
    pub fn export_records(&self, export_path: &str, is_json: bool, filter_text: Option<String>) -> Result<u64> {
        let cols = self.known_columns.lock().clone();
        let conn = self.conn.lock();

        let where_clause = Self::build_filter_where_clause(&filter_text, &cols);

        let clean_path = export_path.replace('\\', "/").replace('\'', "''");
        let copy_sql = if is_json {
            format!(
                "COPY (SELECT * FROM feed_records {}) TO '{}' (FORMAT JSON, ARRAY TRUE);",
                where_clause, clean_path
            )
        } else {
            format!(
                "COPY (SELECT * FROM feed_records {}) TO '{}' (HEADER, DELIMITER ',');",
                where_clause, clean_path
            )
        };

        conn.execute_batch(&copy_sql)?;

        let mut stmt = conn.prepare(&format!("SELECT COUNT(*) FROM feed_records {};", where_clause))?;
        let count: u64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    /// Export frequency breakdown of a single column to CSV (supporting search filters)
    pub fn export_column_frequency(&self, col_name: &str, export_path: &str, filter_text: Option<String>) -> Result<u64> {
        let cols = self.known_columns.lock().clone();
        let sanitized = Self::sanitize_column_name(col_name);
        let conn = self.conn.lock();
        let clean_path = export_path.replace('\\', "/").replace('\'', "''");
        let where_clause = Self::build_filter_where_clause(&filter_text, &cols);

        let copy_sql = format!(
            "COPY (
                WITH stats AS (
                    SELECT COUNT(*) as total_count FROM feed_records {where_clause}
                ),
                freq AS (
                    SELECT 
                        COALESCE(CAST(\"{sanitized}\" AS VARCHAR), '[NULL / EMPTY]') as \"{col_name}\",
                        COUNT(*) as count,
                        ROUND(COUNT(*) * 100.0 / NULLIF((SELECT total_count FROM stats), 0), 2) as percentage
                    FROM feed_records
                    {where_clause}
                    GROUP BY \"{sanitized}\"
                    ORDER BY count DESC
                )
                SELECT * FROM freq
            ) TO '{clean_path}' (HEADER, DELIMITER ',');",
            col_name = col_name.replace('"', "\"\"")
        );
        conn.execute_batch(&copy_sql)?;

        let mut stmt = conn.prepare(&format!("SELECT COUNT(DISTINCT \"{}\") FROM feed_records {};", sanitized, where_clause))?;
        let count: u64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        Ok(count)
    }

    /// Ultra-fast dynamic cross-tabulation / pivot query (e.g. Company x Remote or Company x Status)
    pub fn get_cross_tabulation(
        &self,
        primary_col: &str,
        secondary_col: &str,
        limit: usize,
        filter_text: Option<String>,
    ) -> Result<CrossTabulationResult> {
        let cols = self.known_columns.lock().clone();
        let pri_sanitized = Self::sanitize_column_name(primary_col);
        let sec_sanitized = Self::sanitize_column_name(secondary_col);
        let where_clause = Self::build_filter_where_clause(&filter_text, &cols);
        let conn = self.conn.lock();

        // Total records matching filter
        let mut tot_stmt = conn.prepare(&format!("SELECT COUNT(*) FROM feed_records {};", where_clause))?;
        let total_records: u64 = tot_stmt.query_row([], |row| row.get(0)).unwrap_or(0);

        // Total unique primary values
        let mut uniq_stmt = conn.prepare(&format!(
            "SELECT COUNT(DISTINCT \"{}\") FROM feed_records {};",
            pri_sanitized, where_clause
        ))?;
        let total_primary_uniques: usize = uniq_stmt.query_row([], |row| row.get(0)).unwrap_or(0);

        // Get top distinct secondary values (up to 20 to avoid excessive columns)
        let sec_val_query = format!(
            "SELECT COALESCE(NULLIF(TRIM(\"{}\"), ''), '[Empty]') as val, COUNT(*) as cnt
             FROM feed_records {}
             GROUP BY val
             ORDER BY cnt DESC
             LIMIT 20;",
            sec_sanitized, where_clause
        );

        let mut sec_stmt = conn.prepare(&sec_val_query)?;
        let sec_vals: Vec<String> = sec_stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        if sec_vals.is_empty() || total_records == 0 {
            return Ok(CrossTabulationResult {
                primary_col: primary_col.to_string(),
                secondary_col: secondary_col.to_string(),
                secondary_values: Vec::new(),
                rows: Vec::new(),
                total_primary_uniques: 0,
                total_records: 0,
            });
        }

        // Check if there are other distinct secondary values beyond the top 20
        let mut sec_cnt_stmt = conn.prepare(&format!(
            "SELECT COUNT(DISTINCT COALESCE(NULLIF(TRIM(\"{}\"), ''), '[Empty]')) FROM feed_records {};",
            sec_sanitized, where_clause
        ))?;
        let total_sec_uniques: usize = sec_cnt_stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        let has_other = total_sec_uniques > sec_vals.len();

        let mut select_clauses = Vec::new();
        select_clauses.push(format!(
            "COALESCE(NULLIF(TRIM(\"{}\"), ''), '[Unknown]') as pri_val",
            pri_sanitized
        ));

        for val in &sec_vals {
            let escaped_val = val.replace('\'', "''");
            if val == "[Empty]" {
                select_clauses.push(format!(
                    "COUNT(CASE WHEN \"{}\" IS NULL OR TRIM(\"{}\") = '' THEN 1 END)",
                    sec_sanitized, sec_sanitized
                ));
            } else {
                select_clauses.push(format!(
                    "COUNT(CASE WHEN TRIM(\"{}\") = '{}' THEN 1 END)",
                    sec_sanitized, escaped_val
                ));
            }
        }
        select_clauses.push("COUNT(*) as tot".to_string());

        let query = format!(
            "SELECT {}
             FROM feed_records
             {}
             GROUP BY \"{}\"
             ORDER BY tot DESC
             LIMIT {};",
            select_clauses.join(", "),
            where_clause,
            pri_sanitized,
            limit
        );

        let mut stmt = conn.prepare(&query)?;
        let sec_vals_clone = sec_vals.clone();
        let num_sec = sec_vals.len();

        let rows = stmt.query_map([], |row| {
            let pri_val: String = row.get(0)?;
            let mut counts = HashMap::new();
            let mut top_sum: u64 = 0;
            for i in 0..num_sec {
                let cnt: u64 = row.get(i + 1)?;
                counts.insert(sec_vals_clone[i].clone(), cnt);
                top_sum += cnt;
            }
            let total: u64 = row.get(num_sec + 1)?;
            if has_other {
                let other_cnt = total.saturating_sub(top_sum);
                counts.insert("[Other]".to_string(), other_cnt);
            }
            Ok(CrossTabulationRow {
                primary_value: pri_val,
                counts,
                total,
            })
        })?;

        let mut result_rows = Vec::new();
        for r in rows {
            result_rows.push(r?);
        }

        let mut final_sec_vals = sec_vals;
        if has_other {
            final_sec_vals.push("[Other]".to_string());
        }

        Ok(CrossTabulationResult {
            primary_col: primary_col.to_string(),
            secondary_col: secondary_col.to_string(),
            secondary_values: final_sec_vals,
            rows: result_rows,
            total_primary_uniques,
            total_records,
        })
    }

    /// Fast native export of dynamic cross-tabulation / pivot to CSV
    pub fn export_cross_tabulation(
        &self,
        primary_col: &str,
        secondary_col: &str,
        export_path: &str,
        filter_text: Option<String>,
    ) -> Result<u64> {
        let cols = self.known_columns.lock().clone();
        let pri_sanitized = Self::sanitize_column_name(primary_col);
        let sec_sanitized = Self::sanitize_column_name(secondary_col);
        let where_clause = Self::build_filter_where_clause(&filter_text, &cols);
        let conn = self.conn.lock();
        let clean_path = export_path.replace('\\', "/").replace('\'', "''");

        // 1. Get top distinct secondary values
        let sec_val_query = format!(
            "SELECT COALESCE(NULLIF(TRIM(\"{}\"), ''), '[Empty]') as val, COUNT(*) as cnt
             FROM feed_records {}
             GROUP BY val
             ORDER BY cnt DESC
             LIMIT 20;",
            sec_sanitized, where_clause
        );

        let mut sec_stmt = conn.prepare(&sec_val_query)?;
        let sec_vals: Vec<String> = sec_stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        if sec_vals.is_empty() {
            return Ok(0);
        }

        let mut sec_cnt_stmt = conn.prepare(&format!(
            "SELECT COUNT(DISTINCT COALESCE(NULLIF(TRIM(\"{}\"), ''), '[Empty]')) FROM feed_records {};",
            sec_sanitized, where_clause
        ))?;
        let total_sec_uniques: usize = sec_cnt_stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        let has_other = total_sec_uniques > sec_vals.len();

        let mut select_clauses = Vec::new();
        select_clauses.push(format!(
            "COALESCE(NULLIF(TRIM(\"{}\"), ''), '[Unknown]') as \"{}\"",
            pri_sanitized, primary_col.replace('"', "\"\"")
        ));

        let mut count_exprs = Vec::new();
        for val in &sec_vals {
            let escaped_val = val.replace('\'', "''");
            let col_alias = val.replace('"', "\"\"");
            let expr = if val == "[Empty]" {
                format!("COUNT(CASE WHEN \"{}\" IS NULL OR TRIM(\"{}\") = '' THEN 1 END)", sec_sanitized, sec_sanitized)
            } else {
                format!("COUNT(CASE WHEN TRIM(\"{}\") = '{}' THEN 1 END)", sec_sanitized, escaped_val)
            };
            select_clauses.push(format!("{} as \"{}\"", expr, col_alias));
            count_exprs.push(expr);
        }

        if has_other && !count_exprs.is_empty() {
            select_clauses.push(format!("(COUNT(*) - ({})) as \"[Other]\"", count_exprs.join(" + ")));
        }
        select_clauses.push("COUNT(*) as \"Total\"".to_string());

        let copy_sql = format!(
            "COPY (
                SELECT {}
                FROM feed_records
                {}
                GROUP BY \"{}\"
                ORDER BY \"Total\" DESC
            ) TO '{}' (HEADER, DELIMITER ',');",
            select_clauses.join(", "),
            where_clause,
            pri_sanitized,
            clean_path
        );

        conn.execute_batch(&copy_sql)?;

        let mut count_stmt = conn.prepare(&format!(
            "SELECT COUNT(DISTINCT \"{}\") FROM feed_records {};",
            pri_sanitized, where_clause
        ))?;
        let count: u64 = count_stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        Ok(count)
    }

    /// Fast native export of complete 3-column breakdown (all pairs: primary_col, secondary_col, job_count)
    pub fn export_breakdown_pairs(
        &self,
        primary_col: &str,
        secondary_col: &str,
        export_path: &str,
        filter_text: Option<String>,
    ) -> Result<u64> {
        let cols = self.known_columns.lock().clone();
        let pri_sanitized = Self::sanitize_column_name(primary_col);
        let sec_sanitized = Self::sanitize_column_name(secondary_col);
        let where_clause = Self::build_filter_where_clause(&filter_text, &cols);
        let conn = self.conn.lock();
        let clean_path = export_path.replace('\\', "/").replace('\'', "''");

        let copy_sql = format!(
            "COPY (
                SELECT 
                    COALESCE(NULLIF(TRIM(\"{pri}\"), ''), '[Unspecified / None]') as \"{pri_header}\",
                    COALESCE(NULLIF(TRIM(\"{sec}\"), ''), '[Unspecified / None]') as \"{sec_header}\",
                    COUNT(*) as job_count
                FROM feed_records
                {where_clause}
                GROUP BY \"{pri}\", \"{sec}\"
                ORDER BY job_count DESC, \"{pri_header}\" ASC
            ) TO '{clean_path}' (HEADER, DELIMITER ',');",
            pri = pri_sanitized,
            sec = sec_sanitized,
            pri_header = primary_col.replace('"', "\"\""),
            sec_header = secondary_col.replace('"', "\"\""),
            where_clause = where_clause,
            clean_path = clean_path
        );

        conn.execute_batch(&copy_sql)?;

        let mut count_stmt = conn.prepare(&format!(
            "SELECT COUNT(*) FROM (SELECT 1 FROM feed_records {} GROUP BY \"{}\", \"{}\");",
            where_clause, pri_sanitized, sec_sanitized
        ))?;
        let count: u64 = count_stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        Ok(count)
    }

    /// Fast native export of user-selected custom columns (with optional deduplication and filtering)
    /// Export custom subset of columns to CSV, optionally deduplicating and/or including count & percentage
    pub fn export_custom_columns(
        &self,
        columns: &[String],
        export_path: &str,
        deduplicate: bool,
        include_counts: bool,
        filter_text: Option<String>,
    ) -> Result<u64> {
        if columns.is_empty() {
            return Ok(0);
        }

        let all_cols = self.known_columns.lock().clone();
        let conn = self.conn.lock();
        let clean_path = export_path.replace('\\', "/").replace('\'', "''");
        let where_clause = Self::build_filter_where_clause(&filter_text, &all_cols);

        let mut select_parts = Vec::new();
        let mut group_parts = Vec::new();
        for col in columns {
            let sanitized = Self::sanitize_column_name(col);
            let header = col.replace('"', "\"\"");
            if include_counts {
                select_parts.push(format!("COALESCE(CAST(\"{}\" AS VARCHAR), '[NULL / EMPTY]') as \"{}\"", sanitized, header));
            } else {
                select_parts.push(format!("\"{}\" as \"{}\"", sanitized, header));
            }
            group_parts.push(format!("\"{}\"", sanitized));
        }

        let copy_sql = if include_counts {
            let select_cols = select_parts.join(", ");
            let group_cols = group_parts.join(", ");
            format!(
                "COPY (
                    WITH stats AS (
                        SELECT COUNT(*) as total_count FROM feed_records {where_clause}
                    )
                    SELECT 
                        {select_cols},
                        COUNT(*) as count,
                        ROUND(COUNT(*) * 100.0 / NULLIF((SELECT total_count FROM stats), 0), 2) as percentage
                    FROM feed_records
                    {where_clause}
                    GROUP BY {group_cols}
                    ORDER BY count DESC
                ) TO '{clean_path}' (HEADER, DELIMITER ',');"
            )
        } else {
            let distinct_kw = if deduplicate { "DISTINCT " } else { "" };
            let select_clause = select_parts.join(", ");
            format!(
                "COPY (
                    SELECT {distinct}{select_clause}
                    FROM feed_records
                    {where_clause}
                ) TO '{clean_path}' (HEADER, DELIMITER ',');",
                distinct = distinct_kw,
                select_clause = select_clause,
                where_clause = where_clause,
                clean_path = clean_path
            )
        };

        conn.execute_batch(&copy_sql)?;

        let count_sql = if include_counts {
            let group_cols = group_parts.join(", ");
            format!(
                "SELECT COUNT(*) FROM (SELECT 1 FROM feed_records {where_clause} GROUP BY {group_cols});"
            )
        } else {
            let distinct_kw = if deduplicate { "DISTINCT " } else { "" };
            let select_clause = select_parts.join(", ");
            format!(
                "SELECT COUNT(*) FROM (SELECT {distinct}{select_clause} FROM feed_records {where_clause});",
                distinct = distinct_kw,
                select_clause = select_clause,
                where_clause = where_clause
            )
        };
        let mut stmt = conn.prepare(&count_sql)?;
        let count: u64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
        Ok(count)
    }

    /// Persist current in-memory feed_records table to a standalone .duckdb file
    pub fn persist_to_file(&self, db_file_path: &str) -> Result<()> {
        let conn = self.conn.lock();
        let clean_path = db_file_path.replace('\\', "/").replace('\'', "''");
        let _ = std::fs::remove_file(db_file_path);

        let sql = format!(
            "ATTACH '{}' AS disk_db;
             CREATE TABLE disk_db.feed_records AS SELECT * FROM feed_records;
             DETACH disk_db;",
            clean_path
        );
        conn.execute_batch(&sql)?;
        Ok(())
    }

    /// Load feed_records table from an existing .duckdb file into memory
    pub fn load_from_db(&self, db_file_path: &str) -> Result<OverviewStats> {
        let conn = self.conn.lock();
        let clean_path = db_file_path.replace('\\', "/").replace('\'', "''");

        let sql = format!(
            "DROP TABLE IF EXISTS feed_records;
             ATTACH '{}' AS disk_db (READ_ONLY);
             CREATE TABLE feed_records AS SELECT * FROM disk_db.feed_records;
             DETACH disk_db;",
            clean_path
        );
        conn.execute_batch(&sql)?;

        // Discover columns
        let mut pragma_stmt = conn.prepare("PRAGMA table_info('feed_records');")?;
        let col_names = pragma_stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<String>, _>>()?;

        let mut known = self.known_columns.lock();
        *known = col_names.clone();

        let mut mapping = self.column_mapping.lock();
        mapping.clear();
        for c in &col_names {
            mapping.insert(c.clone(), c.clone());
        }

        // Count total records
        let mut count_stmt = conn.prepare("SELECT COUNT(*) FROM feed_records;")?;
        let total: u64 = count_stmt.query_row([], |row| row.get(0))?;

        Ok(OverviewStats {
            total_records: total,
            column_count: col_names.len(),
            columns: col_names,
            elapsed_secs: 0.05,
            records_per_sec: 0.0,
            memory_usage_mb: 25.0,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentFeed {
    pub id: String,
    pub filename: String,
    pub source_type: String, // "file" or "url"
    pub file_size: String,
    pub total_records: u64,
    pub column_count: usize,
    pub db_path: String,
    pub analyzed_at: String,
}

pub fn get_feedlens_data_dir() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(".feedlens_data");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn get_recent_catalog_path() -> std::path::PathBuf {
    get_feedlens_data_dir().join("recent_catalog.json")
}

pub fn load_recent_catalog() -> Vec<RecentFeed> {
    let path = get_recent_catalog_path();
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(feeds) = serde_json::from_slice::<Vec<RecentFeed>>(&bytes) {
            return feeds;
        }
    }
    Vec::new()
}

pub fn save_recent_feed_entry(feed: RecentFeed) -> Result<()> {
    let mut catalog = load_recent_catalog();
    catalog.retain(|f| f.id != feed.id && f.db_path != feed.db_path);
    catalog.insert(0, feed);
    if catalog.len() > 30 {
        catalog.truncate(30);
    }
    let data = serde_json::to_vec_pretty(&catalog)?;
    std::fs::write(get_recent_catalog_path(), data)?;
    Ok(())
}

pub fn delete_recent_feed_entry(id: &str) -> Result<()> {
    let mut catalog = load_recent_catalog();
    if let Some(pos) = catalog.iter().position(|f| f.id == id) {
        let feed = catalog.remove(pos);
        let _ = std::fs::remove_file(&feed.db_path);
        let _ = std::fs::remove_file(format!("{}.wal", feed.db_path));
    }
    let data = serde_json::to_vec_pretty(&catalog)?;
    std::fs::write(get_recent_catalog_path(), data)?;
    Ok(())
}
