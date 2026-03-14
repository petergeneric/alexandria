// UniFFI interface for Swift integration

use std::path::Path;
use std::sync::Arc;

use crate::index::{build_schema, index_snapshots, open_or_create_index};
use crate::ingest::{IngestSource, RecollFileSource};
use crate::search::SearchEngine;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum AlexandriaError {
    #[error("Failed to open index: {reason}")]
    IndexOpen { reason: String },
    #[error("Search failed: {reason}")]
    SearchFailed { reason: String },
    #[error("Ingestion failed: {reason}")]
    IngestFailed { reason: String },
}

#[derive(uniffi::Record)]
pub struct AlexandriaSearchResult {
    pub url: String,
    pub title: String,
    pub content_snippet: String,
    pub domain: String,
    pub score: f32,
}

#[derive(uniffi::Object)]
pub struct AlexandriaEngine {
    engine: SearchEngine,
    index: tantivy::Index,
    index_path: String,
}

#[uniffi::export]
impl AlexandriaEngine {
    #[uniffi::constructor]
    pub fn open(index_path: String) -> Result<Arc<Self>, AlexandriaError> {
        let index = open_or_create_index(Path::new(&index_path)).map_err(|e| {
            AlexandriaError::IndexOpen {
                reason: e.to_string(),
            }
        })?;

        let engine = SearchEngine::new(index.clone());
        Ok(Arc::new(Self {
            engine,
            index,
            index_path,
        }))
    }

    pub fn search(
        &self,
        query: String,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<AlexandriaSearchResult>, AlexandriaError> {
        let results = self
            .engine
            .search(&query, limit as usize, offset as usize)
            .map_err(|e| AlexandriaError::SearchFailed {
                reason: e.to_string(),
            })?;

        Ok(results
            .into_iter()
            .map(|r| AlexandriaSearchResult {
                url: r.url,
                title: r.title,
                content_snippet: r.content_snippet,
                domain: r.domain,
                score: r.score,
            })
            .collect())
    }

    pub fn ingest(&self, source_dir: String) -> Result<u64, AlexandriaError> {
        let source_path = Path::new(&source_dir);
        if !source_path.is_dir() {
            return Err(AlexandriaError::IngestFailed {
                reason: format!("Not a directory: {source_dir}"),
            });
        }

        let index_path = Path::new(&self.index_path);
        let last_indexed = index_path
            .join(".last-indexed")
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok());

        let mut file_source = RecollFileSource::new(source_path);
        file_source.modified_since = last_indexed;

        let snapshots = file_source.scan().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        if snapshots.is_empty() {
            return Ok(0);
        }

        let (_schema, fields) = build_schema();
        let mut writer = self
            .index
            .writer(50_000_000)
            .map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;

        let indexed =
            index_snapshots(&mut writer, &fields, &self.index, snapshots).map_err(|e| {
                AlexandriaError::IngestFailed {
                    reason: e.to_string(),
                }
            })?;

        // Update .last-indexed marker
        let marker = index_path.join(".last-indexed");
        let _ = std::fs::create_dir_all(index_path);
        let _ = std::fs::write(&marker, "");

        Ok(indexed as u64)
    }
}
