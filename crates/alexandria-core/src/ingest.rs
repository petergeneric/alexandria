// Ingestion module: core types for page snapshots

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
