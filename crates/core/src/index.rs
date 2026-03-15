//! Tantivy index management: schema definition, index creation, and batch indexing.

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
    pub page_id: Field,
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
            page_id: field("page_id")?,
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
    builder.add_u64_field("page_id", STORED);

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

/// Index a batch of page snapshots.
pub fn index_snapshots(
    writer: &mut IndexWriter,
    fields: &SchemaFields,
    snapshots: Vec<PageSnapshot>,
) -> Result<usize, IndexError> {
    let mut indexed = 0;

    for snapshot in snapshots {
        let visited = tantivy::DateTime::from_timestamp_secs(snapshot.captured_at);
        let mut doc = TantivyDocument::new();
        doc.add_text(fields.url, &snapshot.url);
        doc.add_text(fields.title, &snapshot.title);
        doc.add_text(fields.content, &snapshot.content);
        doc.add_text(fields.domain, &snapshot.domain);
        doc.add_date(fields.visited_at, visited);
        doc.add_u64(fields.page_id, snapshot.page_id as u64);

        writer.add_document(doc)?;
        indexed += 1;
    }

    writer.commit()?;
    Ok(indexed)
}
