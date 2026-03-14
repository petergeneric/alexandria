# Rust Public API Reference

## alexandria-core

### `extract` module

```rust
pub fn html_to_markdown(html: &str) -> String;
pub fn markdown_to_plaintext(md: &str) -> String;
pub fn html_to_plaintext(html: &str) -> String;
pub fn extract_title(html: &str) -> String;
pub fn extract_domain(url: &str) -> String;
```

### `filter` module

```rust
pub fn filter_html(html: &str, url: &str) -> String;
```

Site-specific HTML filtering using CSS selectors. Strips boilerplate elements (navigation, ads, sidebars) for known sites including Hacker News, Reddit, and Bluesky.

### `ingest` module

```rust
// Error type
pub enum IngestError { Io, MissingPair, MetadataParse }

// Page data
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub content: String,       // plaintext for indexing
    pub html: String,          // raw HTML for storage
    pub domain: String,
    pub source_hash: String,
    pub captured_at: DateTime<Utc>,
}

// Trait
pub trait IngestSource {
    fn scan(&self) -> Result<Vec<PageSnapshot>, IngestError>;
}

// File-based source
pub struct RecollFileSource {
    pub cache_dir: PathBuf,
    pub modified_since: Option<SystemTime>,
}
impl RecollFileSource {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self;
}
impl IngestSource for RecollFileSource { .. }
```

### `index` module

```rust
pub enum IndexError { Tantivy, Io }

pub struct SchemaFields { pub url, title, content, html, domain, visited_at, source_hash: Field }

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
    pub html: String,             // full stored raw HTML
    pub domain: String,
    pub score: f32,
    pub visited_at: Option<DateTime<Utc>>,
}

pub struct SearchEngine { .. }
impl SearchEngine {
    pub fn new(index: Index) -> Self;
    pub fn search(&self, query: &str, limit: usize, offset: usize) -> Result<Vec<SearchResult>, SearchError>;
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
    pub fn search(&self, query: String, limit: u32, offset: u32)
        -> Result<Vec<AlexandriaSearchResult>, AlexandriaError>;
    pub fn ingest(&self, source_dir: String) -> Result<u64, AlexandriaError>;
    pub fn doc_count(&self) -> Result<u64, AlexandriaError>;
    pub fn clear_index(&self) -> Result<(), AlexandriaError>;
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
