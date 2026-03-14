// Search interface over the Tantivy index

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::Index;
use thiserror::Error;

use crate::{extract, filter};
use crate::index::SchemaFields;
use crate::page_store::PageStore;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
    #[error("index error: {0}")]
    Index(#[from] crate::index::IndexError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content_snippet: String,
    pub domain: String,
    pub score: f32,
    pub visited_at: Option<DateTime<Utc>>,
}

pub struct SearchEngine {
    index: Index,
    fields: SchemaFields,
}

impl SearchEngine {
    pub fn new(index: Index) -> Result<Self, crate::index::IndexError> {
        let fields = SchemaFields::from_index(&index)?;
        Ok(Self { index, fields })
    }

    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
        offset: usize,
        store: Option<&PageStore>,
    ) -> Result<Vec<SearchResult>, SearchError> {
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
            let source_hash = doc
                .get_first(self.fields.source_hash)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let visited_at = doc
                .get_first(self.fields.visited_at)
                .and_then(|v| v.as_datetime())
                .map(|dt| {
                    DateTime::from_timestamp(dt.into_timestamp_secs(), 0)
                        .unwrap_or_default()
                });

            // Generate snippet from HTML stored in SQLite
            let content_snippet = store
                .and_then(|s| s.get_html(&source_hash).ok().flatten())
                .map(|html| {
                    let filtered = filter::filter_html(&html, &domain);
                    let plaintext = extract::html_to_plaintext(&filtered);
                    kwic_snippet(&plaintext, query_str, 200)
                })
                .unwrap_or_default();

            results.push(SearchResult {
                url,
                title,
                content_snippet,
                domain,
                score,
                visited_at,
            });
        }

        Ok(results)
    }
}

/// Generate a keyword-in-context snippet from `text` centered on the first
/// occurrence of any query keyword, with up to `max_len` characters.
///
/// All offsets are computed as *char* indices and converted to byte boundaries
/// only via `char_indices`, avoiding panics from slicing mid-codepoint.
fn kwic_snippet(text: &str, query: &str, max_len: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_len {
        return text.to_string();
    }

    let keywords: Vec<String> = query
        .split_whitespace()
        .filter(|w| !matches!(w.to_uppercase().as_str(), "AND" | "OR" | "NOT"))
        .map(|w| w.to_lowercase())
        .collect();

    // Find earliest keyword match as a *char* index in the original text.
    let match_char_pos: Option<usize> = keywords
        .iter()
        .filter_map(|kw| {
            // Compare char-by-char so positions stay in sync with `text`.
            let kw_chars: Vec<char> = kw.chars().collect();
            let text_chars: Vec<char> = text.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
            text_chars.windows(kw_chars.len())
                .position(|w| w == kw_chars.as_slice())
        })
        .min();

    // Helper: convert a char index to its byte offset in `text`.
    let char_to_byte = |ci: usize| -> usize {
        text.char_indices()
            .nth(ci)
            .map(|(b, _)| b)
            .unwrap_or(text.len())
    };

    let (start_char, end_char) = match match_char_pos {
        Some(pos) => {
            let half = max_len / 2;
            let raw_start = pos.saturating_sub(half);
            // Snap to word boundary (walk backwards to find a space)
            let start = if raw_start > 0 {
                let byte_start = char_to_byte(raw_start);
                text[..byte_start]
                    .rfind(' ')
                    .map(|b| text[..=b].chars().count())
                    .unwrap_or(0)
            } else {
                0
            };
            let raw_end = (start + max_len).min(char_count);
            let end = if raw_end < char_count {
                let byte_end = char_to_byte(raw_end);
                text[..byte_end]
                    .rfind(' ')
                    .map(|b| text[..b].chars().count())
                    .unwrap_or(raw_end)
            } else {
                char_count
            };
            (start, end)
        }
        None => {
            let raw_end = max_len.min(char_count);
            let end = if raw_end < char_count {
                let byte_end = char_to_byte(raw_end);
                text[..byte_end]
                    .rfind(' ')
                    .map(|b| text[..b].chars().count())
                    .unwrap_or(raw_end)
            } else {
                raw_end
            };
            (0, end)
        }
    };

    let byte_start = char_to_byte(start_char);
    let byte_end = char_to_byte(end_char);

    let mut snippet = text[byte_start..byte_end].to_string();
    if start_char > 0 {
        snippet = format!("...{snippet}");
    }
    if end_char < char_count {
        snippet.push_str("...");
    }
    snippet
}
