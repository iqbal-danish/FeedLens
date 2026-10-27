export interface SelectedFileInfo {
  path: string;
  filename: string;
  size_bytes: number;
  format: string;
}

export interface IngestionProgress {
  bytes_read: number;
  total_bytes: number;
  records_ingested: number;
  percentage: number;
  mb_per_sec: number;
  records_per_sec: number;
  elapsed_secs: number;
  estimated_remaining_secs: number | null;
  status: "detecting" | "indexing" | "aggregating" | "ready" | "cancelled" | "error";
  current_phase: string;
  error_message: string | null;
}

export interface OverviewStats {
  total_records: number;
  column_count: number;
  columns: string[];
  elapsed_secs: number;
  records_per_sec: number;
  memory_usage_mb: number;
}

export interface ColumnCompleteness {
  column_name: string;
  original_name: string;
  total_records: number;
  non_null_count: number;
  empty_count: number;
  valid_count: number;
  fill_rate: number;
  unique_count: number;
  sample_values: string[];
  min_length: number;
  max_length: number;
  avg_length: number;
  inferred_type: string;
  is_candidate_key: boolean;
  has_collision_risk: boolean;
}

export interface ValueFrequency {
  value: string;
  count: number;
  percentage: number;
}

export interface DuplicateEntry {
  key: string;
  count: number;
}

export interface QueryResultPage {
  columns: string[];
  rows: Record<string, string | null>[];
  page: number;
  page_size: number;
  total_records: number;
  total_pages: number;
}

export interface RecentFeed {
  id: string;
  filename: string;
  source_type: "file" | "url";
  file_size: string;
  total_records: number;
  column_count: number;
  db_path: string;
  analyzed_at: string;
}

export interface CrossTabulationRow {
  primary_value: string;
  counts: Record<string, number>;
  total: number;
}

export interface CrossTabulationResult {
  primary_col: string;
  secondary_col: string;
  secondary_values: string[];
  rows: CrossTabulationRow[];
  total_primary_uniques: number;
  total_records: number;
}
