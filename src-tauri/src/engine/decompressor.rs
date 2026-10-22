use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::Arc;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use bzip2::read::BzDecoder;

use super::progress::ProgressReader;

pub enum CompressionFormat {
    None,
    Gzip,
    Bzip2,
    Zip,
    TarGz,
}

pub fn detect_compression_format<P: AsRef<Path>>(path: P) -> CompressionFormat {
    let p = path.as_ref();
    let lower = p.to_string_lossy().to_lowercase();
    if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        CompressionFormat::TarGz
    } else if lower.ends_with(".gz") || lower.ends_with(".gzip") {
        CompressionFormat::Gzip
    } else if lower.ends_with(".bz2") {
        CompressionFormat::Bzip2
    } else if lower.ends_with(".zip") {
        CompressionFormat::Zip
    } else {
        CompressionFormat::None
    }
}

pub fn open_streaming_reader<P: AsRef<Path>>(
    path: P,
    tracker: &super::progress::ProgressTracker,
) -> Result<Box<dyn Read + Send>> {
    let path = path.as_ref();
    let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let progress_file = ProgressReader::new(file, tracker.bytes_read.clone(), tracker.cancel_flag.clone());
    let buffered = BufReader::with_capacity(1024 * 1024 * 4, progress_file);

    let format = detect_compression_format(path);

    match format {
        CompressionFormat::None => Ok(Box::new(buffered)),
        CompressionFormat::Gzip => {
            let decoder = GzDecoder::new(buffered);
            Ok(Box::new(BufReader::with_capacity(1024 * 1024 * 2, decoder)))
        }
        CompressionFormat::Bzip2 => {
            let decoder = BzDecoder::new(buffered);
            Ok(Box::new(BufReader::with_capacity(1024 * 1024 * 2, decoder)))
        }
        CompressionFormat::Zip => {
            let raw_file = File::open(path)?;
            let mut archive = zip::ZipArchive::new(raw_file)?;
            
            let mut best_index = 0;
            let mut max_size = 0;
            for i in 0..archive.len() {
                if let Ok(file_entry) = archive.by_index(i) {
                    let name = file_entry.name().to_lowercase();
                    if name.ends_with(".xml") || name.ends_with(".json") || name.ends_with(".jsonl") {
                        if file_entry.size() > max_size {
                            max_size = file_entry.size();
                            best_index = i;
                        }
                    }
                }
            }

            let mut zip_entry = archive.by_index(best_index)?;
            let mut temp_path = std::env::temp_dir();
            let sanitized_name = zip_entry.name().replace(['/', '\\'], "_");
            temp_path.push(format!("feedlens_extracted_{}_{}", std::process::id(), sanitized_name));
            
            let mut temp_file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&temp_path)?;

            io::copy(&mut zip_entry, &mut temp_file)?;
            temp_file.seek(SeekFrom::Start(0))?;

            let progress_entry = ProgressReader::new(temp_file, tracker.bytes_read.clone(), tracker.cancel_flag.clone());
            Ok(Box::new(BufReader::with_capacity(1024 * 1024 * 2, progress_entry)))
        }
        CompressionFormat::TarGz => {
            let gz_decoder = GzDecoder::new(buffered);
            let mut archive = tar::Archive::new(gz_decoder);
            let entries = archive.entries()?;
            
            for entry_res in entries {
                if let Ok(mut entry) = entry_res {
                    let path_str = entry.path()?.to_string_lossy().to_lowercase();
                    if path_str.ends_with(".xml") || path_str.ends_with(".json") || path_str.ends_with(".jsonl") {
                        let mut buffer = Vec::new();
                        entry.read_to_end(&mut buffer)?;
                        return Ok(Box::new(io::Cursor::new(buffer)));
                    }
                }
            }
            anyhow::bail!("No XML or JSON entry found inside tar.gz archive")
        }
    }
}

/// Bounded streaming reader that receives chunks from an asynchronous network downloader thread.
/// Memory footprint is strictly bounded to the channel capacity (e.g. 128 chunks * 128KB = 16MB max).
pub struct ChannelStreamReader {
    rx: Receiver<std::result::Result<Vec<u8>, String>>,
    current_chunk: Option<Vec<u8>>,
    cursor: usize,
    cancel_flag: Arc<AtomicBool>,
}

impl ChannelStreamReader {
    pub fn new(rx: Receiver<std::result::Result<Vec<u8>, String>>, cancel_flag: Arc<AtomicBool>) -> Self {
        Self {
            rx,
            current_chunk: None,
            cursor: 0,
            cancel_flag,
        }
    }
}

impl Read for ChannelStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.cancel_flag.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Ingestion cancelled by user"));
        }
        if buf.is_empty() {
            return Ok(0);
        }

        loop {
            if let Some(ref chunk) = self.current_chunk {
                if self.cursor < chunk.len() {
                    let available = &chunk[self.cursor..];
                    let to_copy = std::cmp::min(buf.len(), available.len());
                    buf[..to_copy].copy_from_slice(&available[..to_copy]);
                    self.cursor += to_copy;
                    return Ok(to_copy);
                } else {
                    self.current_chunk = None;
                    self.cursor = 0;
                }
            }

            match self.rx.recv() {
                Ok(Ok(next_chunk)) => {
                    self.current_chunk = Some(next_chunk);
                    self.cursor = 0;
                }
                Ok(Err(err_msg)) => {
                    return Err(io::Error::new(io::ErrorKind::Other, err_msg));
                }
                Err(_) => {
                    // Channel closed (downloader reached EOF)
                    return Ok(0);
                }
            }
        }
    }
}

pub fn open_streaming_url_reader(
    url: &str,
    tracker: &super::progress::ProgressTracker,
) -> Result<(Box<dyn Read + Send>, u64)> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(20))
        .timeout_read(std::time::Duration::from_secs(60))
        .build();

    let resp = match agent
        .get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36",
        )
        .set(
            "Accept",
            "text/xml,application/xml,application/json,application/octet-stream,*/*",
        )
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let status_text = r.status_text().to_string();
            if code == 404 {
                anyhow::bail!("Feed not found (HTTP 404). Please verify that the feed URL is active and accessible.");
            } else if code == 403 {
                anyhow::bail!("Access denied (HTTP 403 Forbidden). The feed provider may require authentication or IP whitelisting.");
            } else if code == 401 {
                anyhow::bail!("Authentication required (HTTP 401 Unauthorized).");
            } else {
                anyhow::bail!("Feed server returned error: HTTP {} {}", code, status_text);
            }
        }
        Err(ureq::Error::Transport(e)) => {
            anyhow::bail!(
                "Network connection failed: {}. Please check your internet connection and the feed URL.",
                e
            );
        }
    };

    let content_len = resp
        .header("Content-Length")
        .and_then(|h| h.parse::<u64>().ok())
        .unwrap_or(0);

    let content_type = resp.header("Content-Type").unwrap_or("").to_lowercase();
    let content_encoding = resp.header("Content-Encoding").unwrap_or("").to_lowercase();
    let url_lower = url.to_lowercase();

    let is_gzip_declared = url_lower.ends_with(".gz")
        || url_lower.ends_with(".gzip")
        || content_encoding.contains("gzip")
        || content_type.contains("gzip")
        || content_type.contains("application/x-gzip");

    // Decoupled streaming ring buffer: 128 chunks * 128KB = 16MB maximum memory cap
    let (tx, rx) = sync_channel::<std::result::Result<Vec<u8>, String>>(128);
    let bytes_counter = tracker.bytes_read.clone();
    let cancel_flag = tracker.cancel_flag.clone();
    let mut net_reader = resp.into_reader();

    std::thread::Builder::new()
        .name("feedlens-url-downloader".to_string())
        .spawn(move || {
            let mut chunk = vec![0u8; 128 * 1024];
            loop {
                if cancel_flag.load(Ordering::Relaxed) {
                    let _ = tx.send(Err("Ingestion cancelled".to_string()));
                    break;
                }
                match net_reader.read(&mut chunk) {
                    Ok(0) => break, // EOF reached cleanly
                    Ok(n) => {
                        bytes_counter.fetch_add(n as u64, Ordering::Relaxed);
                        if tx.send(Ok(chunk[..n].to_vec())).is_err() {
                            break; // Consumer dropped/cancelled
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        break;
                    }
                }
            }
        })?;

    let channel_reader = ChannelStreamReader::new(rx, tracker.cancel_flag.clone());
    let mut buffered = BufReader::with_capacity(1024 * 1024 * 2, channel_reader);

    // Also check magic bytes (0x1F, 0x8B) in case neither URL nor headers explicitly indicate gzip
    let has_gzip_magic = {
        let peek = buffered.fill_buf().unwrap_or(&[]);
        peek.len() >= 2 && peek[0] == 0x1F && peek[1] == 0x8B
    };

    if is_gzip_declared || has_gzip_magic {
        let decoder = GzDecoder::new(buffered);
        Ok((
            Box::new(BufReader::with_capacity(1024 * 1024 * 2, decoder)),
            content_len,
        ))
    } else {
        Ok((Box::new(buffered), content_len))
    }
}
