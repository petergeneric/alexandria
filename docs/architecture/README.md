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
         | (future: native messaging)   | C FFI
         v                              v
+--------------------------------------------------+
|                  alexandria-core                  |
|                                                   |
|  +----------+  +---------+  +--------+  +------+  |
|  | Ingestion|->| Queue   |->| Index  |  |Search|  |
|  +----------+  +---------+  +--------+  +------+  |
|       ^                        |            |      |
|       |                     Tantivy      Tantivy   |
|  +---------+                                       |
|  | Extract |  (HTML -> text)                       |
|  +---------+                                       |
+--------------------------------------------------+
         ^
         |
+--------+----------+
| Recoll Webcache   |
| (file-based)      |
+-------------------+
```

## Components

| Component | Crate | Purpose |
|-----------|-------|---------|
| Ingestion | `alexandria-core::ingest` | Pluggable sources for page snapshots |
| Extraction | `alexandria-core::extract` | HTML to plain text, title, domain extraction |
| Queue | `alexandria-core::queue` | Bounded channel decoupling ingestion from indexing |
| Index | `alexandria-core::index` | Tantivy schema, document storage, dedup |
| Search | `alexandria-core::search` | Query parsing, field boosting, pagination |
| Power | `alexandria-core::power` | Low Power Mode detection, queue pause/resume |
| FFI | `alexandria-core::ffi` | C API for Swift integration |
| CLI | `alexandria-cli` | Command-line interface (`alex`) |

## Design Principles

1. **Pluggable ingestion**: The `IngestSource` trait allows adding new page capture methods without changing the core engine
2. **Dedup by source hash**: Each document has a unique `source_hash` (from Recoll filename MD5) checked before indexing
3. **Disk-efficient storage**: `content` field is indexed but not stored; only `content_snippet` is stored for display
4. **Power-aware processing**: Indexing queue pauses in Low Power Mode, resumes when power is restored
5. **Thread-safe pipeline**: Ingestion and indexing run on separate threads, connected by a bounded crossbeam channel

## Data Flow

1. **Scan**: `IngestSource` reads source data (webcache files) and produces `PageSnapshot` structs
2. **Extract**: HTML is parsed into plain text, title, and domain
3. **Enqueue**: Snapshots are pushed onto a bounded channel
4. **Index**: Consumer thread reads from queue, checks for duplicates, indexes into Tantivy
5. **Search**: Queries run against the Tantivy index with field boosting

## Technology Choices

| Choice | Rationale |
|--------|-----------|
| Tantivy | Rust-native full-text search, no external dependencies, fast |
| crossbeam-channel | Bounded, backpressure-aware channel for producer/consumer |
| scraper | HTML parsing and text extraction |
| cbindgen | Generates C headers from Rust for Swift FFI |
| notify | Cross-platform filesystem watching |
| clap | CLI argument parsing with derive macros |
