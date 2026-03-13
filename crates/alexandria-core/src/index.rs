// Tantivy index management

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
}

/// Fields in our Tantivy schema.
pub struct SchemaFields {
    pub url: Field,
    pub title: Field,
    pub content: Field,
    pub html: Field,
    pub domain: Field,
    pub indexed_at: Field,
    pub source_hash: Field,
}

/// Build the Tantivy schema for history documents.
pub fn build_schema() -> (Schema, SchemaFields) {
    let mut builder = Schema::builder();

    let url = builder.add_text_field("url", STRING | STORED);
    let title = builder.add_text_field("title", TEXT | STORED);
    // plaintext indexed for search, not stored
    let content = builder.add_text_field("content", TEXT);
    // raw HTML stored for display
    let html = builder.add_text_field("html", STORED);
    let domain = builder.add_text_field("domain", STRING | STORED);
    let indexed_at = builder.add_date_field("indexed_at", INDEXED | STORED);
    let source_hash = builder.add_text_field("source_hash", STRING | STORED);

    let schema = builder.build();
    let fields = SchemaFields {
        url,
        title,
        content,
        html,
        domain,
        indexed_at,
        source_hash,
    };

    (schema, fields)
}

/// Open or create a Tantivy index at the given path.
pub fn open_or_create_index(index_dir: &Path) -> Result<Index, IndexError> {
    std::fs::create_dir_all(index_dir)?;
    let (schema, _fields) = build_schema();

    let index = if index_dir.join("meta.json").exists() {
        Index::open_in_dir(index_dir)?
    } else {
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

        let now = tantivy::DateTime::from_timestamp_secs(chrono::Utc::now().timestamp());
        let mut doc = TantivyDocument::new();
        doc.add_text(fields.url, &snapshot.url);
        doc.add_text(fields.title, &snapshot.title);
        doc.add_text(fields.content, &snapshot.content);
        doc.add_text(fields.html, &snapshot.html);
        doc.add_text(fields.domain, &snapshot.domain);
        doc.add_date(fields.indexed_at, now);
        doc.add_text(fields.source_hash, &snapshot.source_hash);

        writer.add_document(doc)?;
        indexed += 1;
    }

    writer.commit()?;
    Ok(indexed)
}
