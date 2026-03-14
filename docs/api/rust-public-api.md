# Rust Public API Reference

## alexandria-core

### `extract` module

```rust
pub fn html_to_markdown(html: &str) -> String;
pub fn markdown_to_plaintext(md: &str) -> String;
pub fn html_to_plaintext(html: &str) -> String;
pub fn extract_title(html: &str) -> String;
pub fn extract_domain(url: &str) -> String;
pub fn extract_url_from_html(html: &str) -> Option<String>;
```

### `filter` module

```rust
pub fn filter_html(html: &str, url: &str) -> String;
```

Site-specific HTML filtering using CSS selectors. Strips boilerplate elements (navigation, ads, sidebars) for known sites including Hacker News, Reddit, and Bluesky.

### `ingest` module

```rust
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub content: String,       // plaintext for indexing
    pub domain: String,
    pub source_hash: String,
    pub captured_at: DateTime<Utc>,
}
```

### `page_store` module

```rust
pub struct PageStore { .. }
impl PageStore {
    pub fn open(path: &Path) -> Result<Self, PageStoreError>;
    pub fn upsert(&self, url: &str, title: &str, html: &[u8], domain: &str,
                  source_hash: &str, captured_at: i64) -> Result<(), PageStoreError>;
    pub fn pending(&self, limit: usize) -> Result<Vec<StoredPage>, PageStoreError>;
    pub fn pending_summary(&self) -> Result<(u64, Option<i64>), PageStoreError>;
    pub fn get_html(&self, source_hash: &str) -> Result<Option<String>, PageStoreError>;
    pub fn mark_indexed(&self, source_hash: &str) -> Result<(), PageStoreError>;
    pub fn mark_indexed_batch(&self, hashes: &[&str]) -> Result<(), PageStoreError>;
    pub fn delete_all(&self) -> Result<(), PageStoreError>;
    pub fn reset_indexed(&self) -> Result<(), PageStoreError>;
}
```

### `index` module

```rust
pub enum IndexError { Tantivy, Io }

pub struct SchemaFields { pub url, title, content, domain, visited_at, source_hash: Field }

pub fn build_schema() -> (Schema, SchemaFields);
pub fn open_or_create_index(index_dir: &Path) -> Result<Index, IndexError>;
pub fn is_already_indexed(index: &Index, fields: &SchemaFields, hash: &str) -> Result<bool, IndexError>;
pub fn index_snapshots(writer: &mut IndexWriter, fields: &SchemaFields, index: &Index, snapshots: Vec<PageSnapshot>) -> Result<usize, IndexError>;
```

### `search` module

```rust
pub enum SearchError { Tantivy, QueryParse }

pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content_snippet: String,  // KWIC plaintext snippet
    pub domain: String,
    pub score: f32,
    pub visited_at: Option<DateTime<Utc>>,
}

pub struct SearchEngine { .. }
impl SearchEngine {
    pub fn new(index: Index) -> Self;
    pub fn search(&self, query: &str, limit: usize, offset: usize, store: Option<&PageStore>)
        -> Result<Vec<SearchResult>, SearchError>;
}
```

### `queue` module

```rust
pub struct IngestQueue { .. }
impl IngestQueue {
    pub fn new(capacity: usize) -> Self;
    pub fn sender(&self) -> &Sender<PageSnapshot>;
    pub fn receiver(&self) -> &Receiver<PageSnapshot>;
    pub fn try_send(&self, snapshot: PageSnapshot) -> bool;
}
```

### `power` module

```rust
pub fn is_low_power_mode() -> bool;
```

### `ffi` module

UniFFI-based Swift bindings. See [FFI Architecture](../architecture/ffi.md).

```rust
pub struct AlexandriaEngine { .. }
impl AlexandriaEngine {
    pub fn open(index_path: String) -> Result<Arc<Self>, AlexandriaError>;
    pub fn search(&self, query: String, limit: u32, offset: u32, store_path: String)
        -> Result<Vec<AlexandriaSearchResult>, AlexandriaError>;
    pub fn doc_count(&self) -> Result<u64, AlexandriaError>;
    pub fn delete_history(&self, store_path: String) -> Result<(), AlexandriaError>;
    pub fn reindex(&self, store_path: String) -> Result<u64, AlexandriaError>;
    pub fn pending_status(&self, store_path: String) -> Result<PendingStatus, AlexandriaError>;
    pub fn ingest_from_store(&self, store_path: String) -> Result<u64, AlexandriaError>;
}

pub struct AlexandriaSearchResult {
    pub url: String,
    pub title: String,
    pub content_snippet: String,
    pub domain: String,
    pub score: f32,
    pub visited_at_secs: Option<i64>,
}

pub enum AlexandriaError { IndexOpen, SearchFailed, IngestFailed }
```
