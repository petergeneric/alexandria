# Alexandria - Browsing History Search Engine

## Project Overview

A tool that produces a fulltext index of every page you read on your browser.

## Build & Run

```bash
cargo build --workspace
./target/debug/alex index ~/Downloads/webcache
./target/debug/alex search "query"
./target/debug/alex reindex ~/Downloads/webcache
```

## Architecture

- **HTML → Plaintext**: `htmd` (HTML→Markdown) then `markdown_to_text` (Markdown→plaintext), used for search indexing and snippet generation
- **HTML filtering**: Site-specific CSS selectors (`scraper`) strip boilerplate (HN, Reddit, Bluesky) before text extraction
- **Search index**: Tantivy — plaintext is indexed, raw HTML is stored
- **Title extraction**: Simple `<title>` tag parser (no external dependency)
- **Snippets**: KWIC (keyword-in-context) generated at search time from stored HTML → plaintext conversion
- **FFI**: UniFFI proc-macros generate Swift bindings for the macOS app

## Key Design Decisions

- Index plaintext, store raw HTML: better search relevance + original page preserved
- `.last-indexed` timestamp file in index dir for incremental indexing
- `reindex` deletes index dir (and `.last-indexed`) before rebuilding
- Supporting ingesting Recoll webcache folders. Recoll metadata files use positional line format (line 0: URL, line 2: MIME type), not key-value

## Workspace Layout

- `crates/alexandria-core/` — library: extract, ingest, index, search, filter, queue, power, ffi modules
- `crates/alexandria-cli/` — CLI binary (`alex`) with clap subcommands
- `alexandria-app/` — Swift macOS native app (SwiftUI + UniFFI bindings)
- `extension/` — Firefox extension (not yet implemented)
- `docs/` — architecture and API documentation

## Dependencies

- `htmd` — HTML to Markdown (intermediate step for plaintext extraction)
- `markdown_to_text` — Markdown to plaintext (final step for plaintext extraction)
- `tantivy` — full-text search engine
- `scraper` — HTML parsing for site-specific content filtering
- `uniffi` — Rust-to-Swift FFI binding generator
- `notify` — cross-platform filesystem watching
- `crossbeam-channel` — bounded channel for ingestion queue
- `clap` — CLI argument parsing
- `dirs` — home directory expansion
- `md-5` — MD5 hashing for source deduplication
- `chrono`, `url`, `serde`, `serde_json`, `tracing`, `thiserror` — utilities

## License

AGPL-3.0-or-later
