# Roadmap

## Phase 0: Documentation & Scaffolding (done)

- Project structure and workspace setup
- Architecture documentation
- Stub implementations for all modules

## Phase 1: MVP CLI (done)

- HTML to Markdown conversion via `htmd` (skipping script/style/nav, stripping bold/italic)
- Markdown to plaintext via `markdown_to_text` for search indexing
- Site-specific HTML filtering via `scraper` (Hacker News, Reddit, Bluesky)
- Tantivy indexing: plaintext indexed for search
- Deduplication via `source_hash` field
- KWIC snippet generation at search time
- ANSI keyword highlighting in terminal output
- CLI `search` command with `--limit`, `--offset`, `--json`

## Phase 2: SQLite Page Store (done)

- Browser extension captures pages into SQLite with zstd compression
- Background ingestion from SQLite into Tantivy in batches
- Pending page tracking with `indexed_at` column
- Snippet generation from stored HTML at search time

## Phase 3: Power Management (done)

- Low Power Mode detection via NSProcessInfo
- Battery level monitoring via IOKit
- Automatic pause/resume of indexing

## Phase 4: macOS App (done)

- SwiftUI app for search interface
- UniFFI bindings for Rust-to-Swift communication
- Static library linking via Swift Package Manager
- Search results display with URL, title, snippet, domain, relative time
- Settings view with Delete History and Reindex
- Ingestion support from within the app

## Phase 5: Browser Extension (done)

- Firefox extension for capturing page content
- Native messaging host for direct communication with backend
- Auto-save with domain allow/block lists

## Phase 6: Vector Search

- Local embedding model integration
- Vector storage alongside Tantivy
- Hybrid BM25 + vector similarity scoring
- Semantic search queries
