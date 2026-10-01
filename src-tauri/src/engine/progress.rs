use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IngestionProgress {
    pub bytes_read: u64,
    pub total_bytes: u64,
    pub records_ingested: u64,
    pub percentage: f64,
    pub mb_per_sec: f64,
    pub records_per_sec: f64,
    pub elapsed_secs: f64,
    pub estimated_remaining_secs: Option<f64>,
    pub status: String, // "detecting", "indexing", "aggregating", "ready", "cancelled", "error"
    pub current_phase: String,
    pub error_message: Option<String>,
}

pub struct ProgressTracker {
    pub bytes_read: Arc<AtomicU64>,
    pub records_ingested: Arc<AtomicU64>,
    pub total_bytes: u64,
    pub cancel_flag: Arc<AtomicBool>,
    pub start_time: Instant,
}

impl ProgressTracker {
    pub fn new(total_bytes: u64, cancel_flag: Arc<AtomicBool>) -> Self {
        let now = Instant::now();
        Self {
            bytes_read: Arc::new(AtomicU64::new(0)),
            records_ingested: Arc::new(AtomicU64::new(0)),
            total_bytes,
            cancel_flag,
            start_time: now,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    pub fn snapshot(&self, status: &str, phase: &str, error: Option<String>) -> IngestionProgress {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let bytes = self.bytes_read.load(Ordering::Relaxed);
        let records = self.records_ingested.load(Ordering::Relaxed);

        let percentage = if self.total_bytes > 0 {
            ((bytes as f64 / self.total_bytes as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        let mb_per_sec = if elapsed > 0.05 {
            (bytes as f64 / (1024.0 * 1024.0)) / elapsed
        } else {
            0.0
        };

        let records_per_sec = if elapsed > 0.05 {
            records as f64 / elapsed
        } else {
            0.0
        };

        let estimated_remaining_secs = if bytes > 0 && mb_per_sec > 0.01 && self.total_bytes > bytes {
            let remaining_bytes = self.total_bytes - bytes;
            let byte_rate = bytes as f64 / elapsed;
            Some(remaining_bytes as f64 / byte_rate)
        } else {
            None
        };

        IngestionProgress {
            bytes_read: bytes,
            total_bytes: self.total_bytes,
            records_ingested: records,
            percentage,
            mb_per_sec,
            records_per_sec,
            elapsed_secs: elapsed,
            estimated_remaining_secs,
            status: status.to_string(),
            current_phase: phase.to_string(),
            error_message: error,
        }
    }
}

pub struct ProgressReader<R: Read> {
    inner: R,
    bytes_counter: Arc<AtomicU64>,
    cancel_flag: Arc<AtomicBool>,
}

impl<R: Read> ProgressReader<R> {
    pub fn new(inner: R, bytes_counter: Arc<AtomicU64>, cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            bytes_counter,
            cancel_flag,
        }
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cancel_flag.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Ingestion cancelled by user"));
        }
        let n = self.inner.read(buf)?;
        self.bytes_counter.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}
