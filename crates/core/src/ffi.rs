//! UniFFI interface for the macOS Swift app.
//!
//! Exposes [`AlexandriaEngine`] as the main entry point for search, ingestion,
//! and index management via proc-macro-generated Swift bindings.

use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::app_db::AppDb;
use crate::index::{index_snapshots, open_or_create_index, SchemaFields};
use crate::ingest::PageSnapshot;
use crate::page_store::{PageStore, StoredPage};
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

fn snapshots_from_pages(pages: &[StoredPage]) -> Vec<PageSnapshot> {
    pages
        .iter()
        .map(|p| {
            let filtered_html = filter::filter_html(&p.html, &p.domain);
            let content = extract::html_to_plaintext(&filtered_html);
            PageSnapshot {
                page_id: p.id,
                url: p.url.clone(),
                title: p.title.clone(),
                content,
                domain: p.domain.clone(),
                captured_at: p.captured_at,
            }
        })
        .collect()
}

#[derive(uniffi::Object)]
pub struct AlexandriaEngine {
    engine: SearchEngine,
    index: tantivy::Index,
    app_db: Mutex<AppDb>,
}

#[uniffi::export]
impl AlexandriaEngine {
    #[uniffi::constructor]
    pub fn open(index_path: String, app_db_path: String) -> Result<Arc<Self>, AlexandriaError> {
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

        let app_db = AppDb::open(Path::new(&app_db_path)).map_err(|e| {
            AlexandriaError::IndexOpen {
                reason: e.to_string(),
            }
        })?;

        Ok(Arc::new(Self {
            engine,
            index,
            app_db: Mutex::new(app_db),
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

        if !store_path.is_empty() {
            let store =
                PageStore::open(Path::new(&store_path)).map_err(|e| AlexandriaError::IngestFailed {
                    reason: e.to_string(),
                })?;
            store.delete_all().map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        }

        let app_db = self.app_db.lock().unwrap();
        app_db.set_watermark(0).map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        Ok(())
    }

    pub fn reindex(&self, store_path: String) -> Result<u64, AlexandriaError> {
        if store_path.is_empty() {
            return Ok(0);
        }
        let store =
            PageStore::open(Path::new(&store_path)).map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
        let app_db = self.app_db.lock().unwrap();

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

        app_db.set_watermark(0).map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        let mut total = 0u64;
        let mut watermark: i64 = 0;
        loop {
            let pages = store.pages_after(watermark, 500).map_err(|e| AlexandriaError::IngestFailed {
                reason: e.to_string(),
            })?;
            if pages.is_empty() {
                break;
            }

            let max_id = pages.iter().map(|p| p.id).max().unwrap_or(watermark);
            let snapshots = snapshots_from_pages(&pages);

            let indexed =
                index_snapshots(&mut writer, &fields, snapshots).map_err(|e| {
                    AlexandriaError::IngestFailed {
                        reason: e.to_string(),
                    }
                })?;

            watermark = max_id;
            app_db.set_watermark(watermark).map_err(|e| AlexandriaError::IngestFailed {
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
        let app_db = self.app_db.lock().unwrap();
        let watermark = app_db.get_watermark().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;
        let (count, oldest) = store.pages_after_count(watermark).map_err(|e| AlexandriaError::IngestFailed {
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
        let app_db = self.app_db.lock().unwrap();

        let watermark = app_db.get_watermark().map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        let pages = store.pages_after(watermark, 500).map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        if pages.is_empty() {
            return Ok(0);
        }

        let max_id = pages.iter().map(|p| p.id).max().unwrap_or(watermark);
        let snapshots = snapshots_from_pages(&pages);

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

        let indexed =
            index_snapshots(&mut writer, &fields, snapshots).map_err(|e| {
                AlexandriaError::IngestFailed {
                    reason: e.to_string(),
                }
            })?;

        app_db.set_watermark(max_id).map_err(|e| AlexandriaError::IngestFailed {
            reason: e.to_string(),
        })?;

        Ok(indexed as u64)
    }
}
