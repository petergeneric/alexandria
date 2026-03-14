//! Tantivy index management: schema definition, index creation, dedup, and batch indexing.

use std::path::Path;
use tantivy::schema::*;
use tantivy::{Index, IndexWriter};
use thiserror::Error;

use crate::ingest::PageSnapshot;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("missing schema field: {0}")]
    MissingField(String),
}

/// Fields in our Tantivy schema.
pub struct SchemaFields {
    pub url: Field,
    pub title: Field,
    pub content: Field,
    pub domain: Field,
    pub visited_at: Field,
    pub source_hash: Field,
}

impl SchemaFields {
    /// Resolve field handles from an existing index schema.
    pub fn from_index(index: &Index) -> Result<Self, IndexError> {
        let schema = index.schema();
        let field = |name: &str| {
            schema
                .get_field(name)
                .map_err(|_| IndexError::MissingField(name.to_string()))
        };
        Ok(Self {
            url: field("url")?,
            title: field("title")?,
            content: field("content")?,
            domain: field("domain")?,
            visited_at: field("visited_at")?,
            source_hash: field("source_hash")?,
        })
    }
}

/// Build the Tantivy schema for new indexes.
pub fn build_schema() -> Schema {
    let mut builder = Schema::builder();

    builder.add_text_field("url", STRING | STORED);
    builder.add_text_field("title", TEXT | STORED);
    builder.add_text_field("content", TEXT);
    builder.add_text_field("domain", STRING | STORED);
    builder.add_date_field("visited_at", INDEXED | STORED);
    builder.add_text_field("source_hash", STRING | STORED);

    builder.build()
}

/// Open or create a Tantivy index at the given path.
pub fn open_or_create_index(index_dir: &Path) -> Result<Index, IndexError> {
    std::fs::create_dir_all(index_dir)?;

    let index = if index_dir.join("meta.json").exists() {
        Index::open_in_dir(index_dir)?
    } else {
        let schema = build_schema();
        Index::create_in_dir(index_dir, schema)?
    };

    Ok(index)
}

/// Check if a document with the given source_hash already exists.
pub fn is_already_indexed(index: &Index, fields: &SchemaFields, hash: &str) -> Result<bool, IndexError> {
    let reader = index.reader()?;
    let searcher = reader.searcher();
    let term = tantivy::Term::from_field_text(fields.source_hash, hash);
    let query = tantivy::query::TermQuery::new(term, IndexRecordOption::Basic);
    let count = searcher.search(&query, &tantivy::collector::Count)?;
    Ok(count > 0)
}

/// Index a batch of page snapshots, skipping duplicates.
pub fn index_snapshots(
    writer: &mut IndexWriter,
    fields: &SchemaFields,
    index: &Index,
    snapshots: Vec<PageSnapshot>,
) -> Result<usize, IndexError> {
    let mut indexed = 0;

    for snapshot in snapshots {
        if is_already_indexed(index, fields, &snapshot.source_hash)? {
            tracing::debug!(hash = %snapshot.source_hash, "skipping already-indexed document");
            continue;
        }

        let visited = tantivy::DateTime::from_timestamp_secs(snapshot.captured_at.timestamp());
        let mut doc = TantivyDocument::new();
        doc.add_text(fields.url, &snapshot.url);
        doc.add_text(fields.title, &snapshot.title);
        doc.add_text(fields.content, &snapshot.content);
        doc.add_text(fields.domain, &snapshot.domain);
        doc.add_date(fields.visited_at, visited);
        doc.add_text(fields.source_hash, &snapshot.source_hash);

        writer.add_document(doc)?;
        indexed += 1;
    }

    writer.commit()?;
    Ok(indexed)
}
