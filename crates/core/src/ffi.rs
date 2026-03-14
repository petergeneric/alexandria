//! UniFFI interface for the macOS Swift app.
//!
//! Exposes [`AlexandriaEngine`] as the main entry point for search, ingestion,
//! and index management via proc-macro-generated Swift bindings.

use std::path::Path;
use std::sync::Arc;

use crate::index::{index_snapshots, open_or_create_index, SchemaFields};
use crate::ingest::PageSnapshot;
use crate::page_store::PageStore;
use crate::search::SearchEngine;
use crate::{extract, filter};

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
    pub visited_at_secs: Option<i64>,
}

#[derive(uniffi::Record)]
pub struct PendingStatus {
    pub count: u64,
    pub oldest_captured_at_secs: Option<i64>,
}

#[derive(uniffi::Object)]
pub struct AlexandriaEngine {
    engine: SearchEngine,
    index: tantivy::Index,
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

        let engine = SearchEngine::new(index.clone()).map_err(|e| {
            AlexandriaError::IndexOpen {
                reason: e.to_string(),
            }
        })?;
        Ok(Arc::new(Self {
            engine,
            index,
        }))
    }

    pub fn search(
        &self,
        query: String,
        limit: u32,
        offset: u32,
        store_path: String,
    ) -> Result<Vec<AlexandriaSearchResult>, AlexandriaError> {
        let store = if !store_path.is_empty() {
            PageStore::open(Path::new(&store_path)).ok()
        } else {
            None
        };

        let results = self
            .engine
            .search(&query, limit as usize, offset as usize, store.as_ref())
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
                visited_at_secs: r.visited_at.map(|dt| dt.timestamp()),
            })
            .collect())
    }

    pub fn doc_count(&self) -> Result<u64, AlexandriaError> {
        let reader = self.index.reader().map_err(|e| AlexandriaError::SearchFailed {
            reason: e.to_string(),
        })?;
        let searcher = reader.searcher();
        Ok(searcher.num_docs())
    }

    pub fn delete_history(&self, store_path: String) -> Result<(), AlexandriaError> {
        let mut writer: tantivy::IndexWriter<tantivy::TantivyDocument> = self
            .index
            .writer(50_000_000)
            .map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        writer.delete_all_documents().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;
        writer.commit().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        // Truncate the SQLite page store
        if !store_path.is_empty() {
            let store =
                PageStore::open(Path::new(&store_path)).map_err(|e| AlexandriaError::IngestFailed {
                    reason: e.to_string(),
                })?;
            store.delete_all().map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        }

        Ok(())
    }

    pub fn reindex(&self, store_path: String) -> Result<u64, AlexandriaError> {
        // Reset all pages in SQLite to pending
        if store_path.is_empty() {
            return Ok(0);
        }
        let store =
            PageStore::open(Path::new(&store_path)).map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        store.reset_indexed().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        // Clear the Tantivy index and re-ingest using a single writer
        let fields = SchemaFields::from_index(&self.index).map_err(|e| {
            AlexandriaError::IngestFailed {
                reason: e.to_string(),
            }
        })?;
        let mut writer = self
            .index
            .writer(50_000_000)
            .map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        writer.delete_all_documents().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;
        writer.commit().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        // Re-ingest everything in batches using the same writer
        let mut total = 0u64;
        loop {
            let pages = store.pending(500).map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
            if pages.is_empty() {
                break;
            }

            let snapshots: Vec<PageSnapshot> = pages
                .iter()
                .map(|p| {
                    let filtered_html = filter::filter_html(&p.html, &p.domain);
                    let content = extract::html_to_plaintext(&filtered_html);
                    let captured_at =
                        chrono::DateTime::from_timestamp(p.captured_at, 0).unwrap_or_else(chrono::Utc::now);
                    PageSnapshot {
                        url: p.url.clone(),
                        title: p.title.clone(),
                        content,
                        domain: p.domain.clone(),
                        source_hash: p.source_hash.clone(),
                        captured_at,
                    }
                })
                .collect();

            let hashes: Vec<&str> = pages.iter().map(|p| p.source_hash.as_str()).collect();
            let indexed =
                index_snapshots(&mut writer, &fields, &self.index, snapshots).map_err(|e| {
                    AlexandriaError::IngestFailed {
                        reason: e.to_string(),
                    }
                })?;

            store
                .mark_indexed_batch(&hashes)
                .map_err(|e| AlexandriaError::IngestFailed {
                    reason: e.to_string(),
                })?;

            total += indexed as u64;
        }
        Ok(total)
    }

    pub fn pending_status(&self, store_path: String) -> Result<PendingStatus, AlexandriaError> {
        let store =
            PageStore::open(Path::new(&store_path)).map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        let (count, oldest) = store.pending_summary().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;
        Ok(PendingStatus {
            count,
            oldest_captured_at_secs: oldest,
        })
    }

    pub fn ingest_from_store(&self, store_path: String) -> Result<u64, AlexandriaError> {
        let store =
            PageStore::open(Path::new(&store_path)).map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;

        let pages = store.pending(500).map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        if pages.is_empty() {
            return Ok(0);
        }

        let snapshots: Vec<PageSnapshot> = pages
            .iter()
            .map(|p| {
                let filtered_html = filter::filter_html(&p.html, &p.domain);
                let content = extract::html_to_plaintext(&filtered_html);
                let captured_at =
                    chrono::DateTime::from_timestamp(p.captured_at, 0).unwrap_or_else(chrono::Utc::now);
                PageSnapshot {
                    url: p.url.clone(),
                    title: p.title.clone(),
                    content,
                    domain: p.domain.clone(),
                    source_hash: p.source_hash.clone(),
                    captured_at,
                }
            })
            .collect();

        let fields = SchemaFields::from_index(&self.index).map_err(|e| {
            AlexandriaError::IngestFailed {
                reason: e.to_string(),
            }
        })?;
        let mut writer = self
            .index
            .writer(50_000_000)
            .map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;

        let hashes: Vec<&str> = pages.iter().map(|p| p.source_hash.as_str()).collect();

        let indexed =
            index_snapshots(&mut writer, &fields, &self.index, snapshots).map_err(|e| {
                AlexandriaError::IngestFailed {
                    reason: e.to_string(),
                }
            })?;

        store
            .mark_indexed_batch(&hashes)
            .map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;

        Ok(indexed as u64)
    }

}
