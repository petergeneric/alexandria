# Data Model

## SQLite Page Store (`pages.db`)

The browser extension captures pages into a SQLite database. Each page is stored with zstd-compressed HTML and a content hash for deduplication.

### Schema

```sql
CREATE TABLE pages (
    id            INTEGER PRIMARY KEY,
    url           TEXT NOT NULL,
    title         TEXT NOT NULL DEFAULT '',
    html          BLOB NOT NULL,          -- zstd-compressed raw HTML
    domain        TEXT NOT NULL DEFAULT '',
    captured_at   INTEGER NOT NULL,       -- Unix timestamp
    content_hash  BLOB NOT NULL           -- 16-byte xxhash3_128
);
```

## Application Database (`app.db`)

Bookkeeping metadata is stored separately from page data. Managed by the `app_db` module.

### `meta` table

Key-value store for indexing state:

| Key | Value | Purpose |
|-----|-------|---------|
| `watermark` | rowid (integer as text) | Highest `pages.id` that has been indexed into Tantivy |
| `schema_revision` | integer as text | Tantivy schema version for migration detection |

### `ingest_log` table

Tracks pages that failed during indexing, surfaced in the macOS app UI.

```sql
CREATE TABLE ingest_log (
    id        INTEGER PRIMARY KEY,
    timestamp TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    page_id   INTEGER NOT NULL,
    url       TEXT NOT NULL,
    domain    TEXT NOT NULL,
    reason    TEXT NOT NULL
);
```

## Tantivy Schema

| Field | Type | Indexed | Stored | Notes |
|-------|------|---------|--------|-------|
| `url` | STRING | yes | yes | Original page URL |
| `title` | TEXT | yes | yes | Extracted from `<title>` tag |
| `content` | TEXT | yes | **no** | Plaintext for search only |
| `domain` | STRING | yes | yes | Extracted from URL |
| `visited_at` | DATE | yes | yes | Timestamp when page was captured |
| `page_id` | U64 | no | yes | SQLite rowid, used to fetch HTML for snippets |

### Field Design Rationale

- **Plaintext indexed, raw HTML in SQLite**: Indexing plaintext avoids polluting the search index with HTML markup. Raw HTML is stored in SQLite for snippet generation.
- **STRING** fields are indexed as single tokens (exact match). Used for URLs, domains.
- **TEXT** fields are tokenized and analyzed. Used for title and content (full-text search).

## Deduplication

Pages are deduplicated by `content_hash` (xxhash3_128 of the raw HTML). The page store checks for an existing hash before inserting.

## Indexing Progress

Instead of marking individual rows as indexed, the system uses a **watermark** — the highest `pages.id` that has been processed. Each ingestion cycle reads pages with `id > watermark`, indexes them, then advances the watermark. This lives in `app.db`, keeping `pages.db` owned entirely by the browser extension.
