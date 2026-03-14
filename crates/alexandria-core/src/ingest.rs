// Ingestion module: pluggable sources for page snapshots

use crate::{extract, filter};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing paired file for hash {0}")]
    MissingPair(String),
    #[error("failed to parse metadata: {0}")]
    MetadataParse(String),
}

/// A snapshot of a web page, ready for indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    /// Plaintext for search indexing
    pub content: String,
    pub domain: String,
    pub source_hash: String,
    pub captured_at: DateTime<Utc>,
}

/// Trait for pluggable ingestion sources.
pub trait IngestSource {
    fn scan(&self) -> Result<Vec<PageSnapshot>, IngestError>;
}

/// File-based ingestion from Recoll webcache directory.
pub struct RecollFileSource {
    pub cache_dir: std::path::PathBuf,
    pub modified_since: Option<SystemTime>,
}

impl RecollFileSource {
    pub fn new(cache_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
            modified_since: None,
        }
    }

    /// Parse a Recoll metadata file (positional line-based format).
    /// Line 0: URL, Line 1: source label, Line 2: MIME type, Line 3: encoding
    fn parse_metadata(path: &Path) -> Result<RecollMetadata, IngestError> {
        let content = std::fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().collect();

        let url = lines
            .first()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| IngestError::MetadataParse("missing url on line 1".into()))?
            .to_string();

        let mime_type = lines
            .get(2)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "text/html".into());

        Ok(RecollMetadata { url, mime_type })
    }

    /// Extract the MD5 hash from a Recoll filename.
    /// Format: recoll-we-{m|c}-{MD5}.rclwe
    fn extract_hash(filename: &str) -> Option<String> {
        let stem = filename.strip_suffix(".rclwe")?;
        let hash = stem.strip_prefix("recoll-we-m-")
            .or_else(|| stem.strip_prefix("recoll-we-c-"))?;
        Some(hash.to_string())
    }
}

struct RecollMetadata {
    url: String,
    #[allow(dead_code)]
    mime_type: String,
}

impl IngestSource for RecollFileSource {
    fn scan(&self) -> Result<Vec<PageSnapshot>, IngestError> {
        let mut snapshots = Vec::new();
        let mut meta_files = std::collections::HashMap::new();

        // First pass: collect all metadata files by hash
        let entries = std::fs::read_dir(&self.cache_dir)?;
        for entry in entries {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with("recoll-we-m-") && filename.ends_with(".rclwe") {
                if let Some(hash) = Self::extract_hash(&filename) {
                    meta_files.insert(hash, entry.path());
                }
            }
        }

        // Second pass: match content files to metadata
        let entries = std::fs::read_dir(&self.cache_dir)?;
        for entry in entries {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.starts_with("recoll-we-c-") && filename.ends_with(".rclwe") {
                if let Some(ref since) = self.modified_since {
                    if let Ok(mtime) = entry.metadata().and_then(|m| m.modified()) {
                        if mtime <= *since {
                            continue;
                        }
                    }
                }
                if let Some(hash) = Self::extract_hash(&filename) {
                    let meta_path = match meta_files.get(&hash) {
                        Some(p) => p,
                        None => continue, // skip unpaired content files
                    };

                    let metadata = Self::parse_metadata(meta_path)?;
                    let raw_html = std::fs::read_to_string(entry.path())?;
                    let title = extract::extract_title(&raw_html);
                    let domain = extract::extract_domain(&metadata.url);
                    let filtered_html = filter::filter_html(&raw_html, &domain);
                    let content = extract::html_to_plaintext(&filtered_html);

                    let captured_at = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| {
                            let dur = t.duration_since(std::time::UNIX_EPOCH).ok()?;
                            DateTime::from_timestamp(dur.as_secs() as i64, 0)
                        })
                        .unwrap_or_else(Utc::now);

                    snapshots.push(PageSnapshot {
                        url: metadata.url,
                        title,
                        content,
                        domain,
                        source_hash: hash,
                        captured_at,
                    });
                }
            }
        }

        Ok(snapshots)
    }
}
