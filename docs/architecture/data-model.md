# Data Model

## Recoll Webcache Format (Source Data)

The initial ingestion source reads from Recoll's webcache directory (typically `~/Downloads/webcache/`).

### File Structure

Flat directory containing paired files identified by MD5 hash:

| File Pattern | Content |
|-------------|---------|
| `recoll-we-m-{MD5}.rclwe` | Metadata (positional line format) |
| `recoll-we-c-{MD5}.rclwe` | Raw HTML content |

The `circache.crch` tar archive index is also present but ignored initially.

### Metadata File Format

Positional, one field per line:

```
https://example.com/page
WebHistory
text/html
k:_unindexed:encoding=UTF-8
```

| Line | Content |
|------|---------|
| 0 | URL |
| 1 | Source label (ignored) |
| 2 | MIME type |
| 3 | Encoding metadata (ignored) |

### Pairing Logic

Files are paired by their MD5 hash. Both the `-m-` (metadata) and `-c-` (content) files must exist for a page to be ingested. Unpaired files are skipped.

## Tantivy Schema

| Field | Type | Indexed | Stored | Notes |
|-------|------|---------|--------|-------|
| `url` | STRING | yes | yes | Original page URL |
| `title` | TEXT | yes | yes | Extracted from `<title>` tag |
| `content` | TEXT | yes | **no** | Plaintext (markdown stripped), for search only |
| `html` | TEXT | no | yes | Raw HTML, for snippet generation and rendering |
| `domain` | STRING | yes | yes | Extracted from URL |
| `visited_at` | DATE | yes | yes | Timestamp when page was captured |
| `source_hash` | STRING | yes | yes | MD5 from Recoll filename, used for dedup |

### Field Design Rationale

- **Plaintext indexed, raw HTML stored**: Indexing plaintext avoids polluting the search index with HTML markup. Storing raw HTML preserves the original page for snippet generation and future rendering (e.g. in the macOS app).
- **STRING** fields are indexed as single tokens (exact match). Used for URLs, domains, and hashes.
- **TEXT** fields are tokenized and analyzed. Used for title and content (full-text search).
- Snippets are generated at search time by converting stored HTML to plaintext (via HTML→Markdown→plaintext pipeline), then extracting a keyword-in-context window.

## Deduplication Strategy

Before indexing a `PageSnapshot`, the system queries the index for a document with the same `source_hash`:

1. Create a `TermQuery` on the `source_hash` field
2. If `Count > 0`, skip the document
3. Otherwise, index it

This prevents re-indexing the same webcache file on subsequent scans.

### Incremental Indexing

The `index` command also uses a `.last-indexed` timestamp file in the index directory. Files with modification times older than this marker are skipped entirely, avoiding unnecessary HTML parsing and conversion.

## PageSnapshot Struct

The intermediate representation between ingestion and indexing:

```rust
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub content: String,     // plaintext for indexing
    pub html: String,        // raw HTML for storage
    pub domain: String,
    pub source_hash: String, // MD5 from filename
    pub captured_at: DateTime<Utc>,
}
```
