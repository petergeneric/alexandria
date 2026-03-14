# Roadmap

## Phase 0: Documentation & Scaffolding (done)

- Project structure and workspace setup
- Architecture documentation
- API reference documentation
- Stub implementations for all modules

## Phase 1: MVP CLI (done)

Minimal viable tool: ingest webcache files and search them.

- HTML to Markdown conversion via `htmd` (skipping script/style/nav, stripping bold/italic)
- Markdown to plaintext via `markdown_to_text` for search indexing
- Site-specific HTML filtering via `scraper` (Hacker News, Reddit, Bluesky)
- Fixed Recoll metadata parser (positional line-based format, not key-value)
- Tantivy indexing: plaintext indexed for search, raw HTML stored for retrieval
- Deduplication via `source_hash` field
- KWIC snippet generation at search time (HTML -> plaintext -> centered on keyword matches)
- ANSI keyword highlighting in terminal output
- Incremental indexing via `.last-indexed` timestamp file
- CLI commands: `index`, `reindex`, `search`
- Search flags: `--limit`, `--offset`, `--raw` (full HTML), `--json`
- Default index location: `./alexandria_index/`

## Phase 2: Watch & Queue

- File watcher for `watch` command (bulk scan on startup, then watch for new files)
- Bounded queue connecting ingestion to indexing (crossbeam channel)
- `info` command (document count, index size)

## Phase 3: Power Management

- Low Power Mode detection via NSProcessInfo
- Queue consumer pause/resume
- Power state change notifications

## Phase 4: macOS App (done)

- SwiftUI app for search interface
- UniFFI bindings for Rust-to-Swift communication
- Static library linking via Swift Package Manager
- Search results display with URL, title, snippet, domain, relative time
- Faceting sidebar with date range and domain filters
- Settings view for index path configuration
- Ingestion support from within the app

## Phase 5: Browser Extension

- Firefox extension for capturing page content
- Native messaging host for direct communication with backend
- Extension popup UI for quick search

## Phase 6: Vector Search

- Local embedding model integration
- Vector storage alongside Tantivy
- Hybrid BM25 + vector similarity scoring
- Semantic search queries

## Phase 7: Multi-Ingestion

- HTTP server ingestion source
- Multiple concurrent ingestion sources
- Source-specific deduplication
