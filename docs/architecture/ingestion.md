![LLM Generated](../llm-generated.svg)

# Ingestion Architecture

## Overview

Pages are captured by the Firefox browser extension and stored in a SQLite database (the page store). A background ingestion process reads pending pages from the store, converts them to plaintext, and indexes them into Tantivy.

## Ingestion Flow

```
[Browser Extension] --native messaging--> [SQLite Page Store]
                                                |
                                          pending(500)
                                                |
                                                v
                                    [filter_html + html_to_plaintext]
                                                |
                                                v
                                       [Tantivy Indexing]
                                                |
                                       mark_indexed_batch()
```

1. The browser extension captures page HTML and writes it to the SQLite page store via the native messaging host
2. The macOS app's `Ingester` runs on an hourly timer (and on power state changes)
3. Each cycle calls `ingest_from_store`, which:
   - Reads up to 500 pending pages (`indexed_at IS NULL`)
   - Filters HTML through site-specific CSS selectors
   - Converts to plaintext via HTML → Markdown → plaintext pipeline
   - Indexes into Tantivy
   - Marks the batch as indexed in SQLite

## Extraction Pipeline

```
Raw HTML
  -> filter_html (site-specific CSS selector removal)
  -> html_to_plaintext (iterative DOM text extraction via scraper, skip script/style/noscript)
  -> extract_title (simple <title> tag parser)
  -> extract_domain (url crate)
```

## Queue Architecture

Ingestion and indexing are decoupled via a bounded crossbeam channel:

```
[PageSnapshot] --try_send()--> [Queue] --recv()--> [Indexer]
```

- **Capacity**: Configurable (default: 1000)
- **Backpressure**: If the queue is full, `try_send` returns false and the snapshot is dropped with a warning
- **Power-aware**: The consumer side can pause/resume based on power state without losing enqueued data

## Indexing Progress

Progress is tracked via a watermark in `app.db` (the highest `pages.id` that has been indexed). Each cycle reads pages above the watermark, indexes them, then advances it. Pages that fail indexing are logged to the `ingest_log` table for visibility in the macOS app.
