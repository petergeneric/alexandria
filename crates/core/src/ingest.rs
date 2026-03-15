//! Core types for page snapshots.

use serde::{Deserialize, Serialize};

/// A web page snapshot ready for Tantivy indexing.
///
/// Created from a [`crate::page_store::StoredPage`] by filtering HTML and
/// converting to plaintext via the HTML → Markdown → plaintext pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSnapshot {
    pub page_id: i64,
    pub url: String,
    pub title: String,
    /// Plaintext for search indexing
    pub content: String,
    pub domain: String,
    pub site_group: String,
    pub captured_at: i64,
}
