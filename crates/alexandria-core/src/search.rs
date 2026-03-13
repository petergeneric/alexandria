// Search interface over the Tantivy index

use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::Index;
use thiserror::Error;

use crate::{extract, filter};
use crate::index::{build_schema, SchemaFields};

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content_snippet: String,
    pub html: String,
    pub domain: String,
    pub score: f32,
}

pub struct SearchEngine {
    index: Index,
    fields: SchemaFields,
}

impl SearchEngine {
    pub fn new(index: Index) -> Self {
        let (_schema, fields) = build_schema();
        Self { index, fields }
    }

    pub fn search(&self, query_str: &str, limit: usize, offset: usize) -> Result<Vec<SearchResult>, SearchError> {
        let reader = self.index.reader()?;
        let searcher = reader.searcher();

        // Field boosting: title 3x, domain 2x, content 1x
        let mut query_parser = QueryParser::for_index(
            &self.index,
            vec![self.fields.title, self.fields.content, self.fields.domain],
        );
        query_parser.set_field_boost(self.fields.title, 3.0);
        query_parser.set_field_boost(self.fields.domain, 2.0);

        let query = query_parser.parse_query(query_str)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit).and_offset(offset))?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            let url = doc
                .get_first(self.fields.url)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let title = doc
                .get_first(self.fields.title)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let domain = doc
                .get_first(self.fields.domain)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let html = doc
                .get_first(self.fields.html)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let filtered = filter::filter_html(&html, &domain);
            let plaintext = extract::html_to_plaintext(&filtered);
            let content_snippet = kwic_snippet(&plaintext, query_str, 200);

            results.push(SearchResult {
                url,
                title,
                content_snippet,
                html,
                domain,
                score,
            });
        }

        Ok(results)
    }
}

/// Generate a keyword-in-context snippet from `text` centered on the first
/// occurrence of any query keyword, with up to `max_len` characters.
fn kwic_snippet(text: &str, query: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }

    let lower_text = text.to_lowercase();
    let keywords: Vec<String> = query
        .split_whitespace()
        .filter(|w| !matches!(w.to_uppercase().as_str(), "AND" | "OR" | "NOT"))
        .map(|w| w.to_lowercase())
        .collect();

    // Find the earliest keyword match (byte position)
    let match_pos = keywords
        .iter()
        .filter_map(|kw| lower_text.find(kw.as_str()))
        .min();

    let (start, end) = match match_pos {
        Some(pos) => {
            // Center the window around the match
            let half = max_len / 2;
            let raw_start = pos.saturating_sub(half);
            // Snap to word boundary
            let start = if raw_start > 0 {
                text[..raw_start]
                    .rfind(' ')
                    .map(|p| p + 1)
                    .unwrap_or(0)
            } else {
                0
            };
            let raw_end = (start + max_len).min(text.len());
            let end = if raw_end < text.len() {
                text[..raw_end].rfind(' ').unwrap_or(raw_end)
            } else {
                text.len()
            };
            (start, end)
        }
        None => {
            // No keyword match — show beginning
            let raw_end = max_len.min(text.len());
            let end = if raw_end < text.len() {
                text[..raw_end].rfind(' ').unwrap_or(raw_end)
            } else {
                raw_end
            };
            (0, end)
        }
    };

    let mut snippet = text[start..end].to_string();
    if start > 0 {
        snippet = format!("...{snippet}");
    }
    if end < text.len() {
        snippet.push_str("...");
    }
    snippet
}
