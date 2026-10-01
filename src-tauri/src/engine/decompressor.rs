use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
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
