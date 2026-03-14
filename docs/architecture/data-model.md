# Data Model

## SQLite Page Store

The browser extension captures pages into a SQLite database (`pages.db`). Each page is stored with its raw HTML (zstd-compressed) and metadata.

### Schema

```sql
CREATE TABLE pages (
    source_hash TEXT PRIMARY KEY,
    url         TEXT NOT NULL,
    title       TEXT NOT NULL DEFAULT '',
    html        BLOB NOT NULL,          -- zstd-compressed raw HTML
    domain      TEXT NOT NULL DEFAULT '',
    captured_at INTEGER NOT NULL,       -- Unix timestamp
    indexed_at  INTEGER                 -- NULL until indexed into Tantivy
);
```

### Key Operations

| Operation | Description |
|-----------|-------------|
| `upsert` | Insert or replace a page (resets `indexed_at` to NULL) |
| `pending(limit)` | Fetch pages where `indexed_at IS NULL` |
| `mark_indexed_batch` | Set `indexed_at` after successful Tantivy indexing |
| `delete_all` | Truncate all pages (used by Delete History) |
| `reset_indexed` | Set all `indexed_at` to NULL (used by Reindex) |

## Tantivy Schema

| Field | Type | Indexed | Stored | Notes |
|-------|------|---------|--------|-------|
| `url` | STRING | yes | yes | Original page URL |
| `title` | TEXT | yes | yes | Extracted from `<title>` tag |
| `content` | TEXT | yes | **no** | Plaintext for search only |
| `domain` | STRING | yes | yes | Extracted from URL |
| `visited_at` | DATE | yes | yes | Timestamp when page was captured |
| `source_hash` | STRING | yes | yes | Hash for dedup |

### Field Design Rationale

- **Plaintext indexed, raw HTML in SQLite**: Indexing plaintext avoids polluting the search index with HTML markup. Raw HTML is stored in SQLite for snippet generation and future rendering.
- **STRING** fields are indexed as single tokens (exact match). Used for URLs, domains, and hashes.
- **TEXT** fields are tokenized and analyzed. Used for title and content (full-text search).
- Snippets are generated at search time by fetching stored HTML from SQLite, converting to plaintext (via HTML→Markdown→plaintext pipeline), then extracting a keyword-in-context window.

## Deduplication Strategy

Before indexing a `PageSnapshot`, the system queries the index for a document with the same `source_hash`:

1. Create a `TermQuery` on the `source_hash` field
2. If `Count > 0`, skip the document
3. Otherwise, index it

## PageSnapshot Struct

The intermediate representation between the page store and indexing:

```rust
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub content: String,     // plaintext for indexing
    pub domain: String,
    pub source_hash: String,
    pub captured_at: DateTime<Utc>,
}
```
