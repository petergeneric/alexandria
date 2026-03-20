![LLM Generated](../llm-generated.svg)

# Architecture Overview

<p align="center">
  <img src="../icon.svg" alt="Alexandria" width="96" height="96">
</p>

## System Context

```
+-------------------+          +-------------------+
|   Firefox         |          |   macOS App       |
|   Extension       |          |   (Swift UI)      |
+--------+----------+          +--------+----------+
         |                              |
         | native messaging             | UniFFI
         v                              v
+--------------------------------------------------+
|                  alexandria-core                  |
|                                                   |
|  +----------+  +---------+  +--------+  +------+  |
|  | Ingestion|->| Queue   |->| Index  |  |Search|  |
|  +----------+  +---------+  +--------+  +------+  |
|       ^                        |            |      |
|       |                     Tantivy      Tantivy   |
|  +---------+  +--------+                          |
|  | Extract |  | Filter |  (HTML -> text)          |
|  +---------+  +--------+                          |
+--------------------------------------------------+
         ^
         |
+--------+----------+
| SQLite Page Store |
| (pages.db)        |
+-------------------+
```

## Components

| Component | Crate | Purpose |
|-----------|-------|---------|
| Ingestion | `alexandria-core::ingest` | Core types for page snapshots |
| Extraction | `alexandria-core::extract` | HTML to plain text, title, domain, URL extraction |
| Filtering | `alexandria-core::filter` | Site-specific HTML boilerplate removal |
| Queue | `alexandria-core::queue` | Bounded channel decoupling ingestion from indexing |
| Index | `alexandria-core::index` | Tantivy schema, document storage, dedup |
| Search | `alexandria-core::search` | Query parsing, field boosting, pagination |
| Page Store | `alexandria-core::page_store` | SQLite storage for captured pages |
| App DB | `alexandria-core::app_db` | Indexing watermark, schema revision, ingest failure log |
| Blocklist | `alexandria-core::blocklist` | URL filtering (banks, auth, checkout pages) |
| FFI | `alexandria-core::ffi` | UniFFI bindings for Swift integration |
| CLI | `alexandria-cli` | Command-line interface (`alex`) |

## Design Principles

1. **Browser extension capture**: Pages are captured by the Firefox extension and stored in SQLite with compressed HTML
2. **Dedup by source hash**: Each document has a unique `source_hash` checked before indexing
3. **Plaintext indexed, HTML in SQLite**: Tantivy indexes plaintext for search; raw HTML lives in SQLite for snippet generation
4. **Power-aware processing**: The macOS app pauses indexing on low battery and Low Power Mode
5. **Thread-safe pipeline**: Ingestion and indexing run on separate threads, connected by a bounded crossbeam channel

## Data Flow

1. **Capture**: Firefox extension saves page HTML to the SQLite page store
2. **Ingest**: Background process reads pending pages from SQLite
3. **Filter**: HTML is filtered through site-specific CSS selectors to remove boilerplate
4. **Extract**: Filtered HTML is converted to plaintext, title, and domain
5. **Index**: Snapshots are indexed into Tantivy with dedup checking
6. **Search**: Queries run against the Tantivy index with field boosting; snippets generated from SQLite HTML

## Technology Choices

| Choice | Rationale |
|--------|-----------|
| Tantivy | Rust-native full-text search, no external dependencies, fast |
| SQLite | Reliable page storage with compression (zstd) |
| UniFFI | Mozilla's Rust-to-Swift binding generator with proc-macro annotations |
| crossbeam-channel | Bounded, backpressure-aware channel for producer/consumer |
| scraper | HTML parsing, site-specific filtering, and iterative plaintext extraction |
| clap | CLI argument parsing with derive macros |
