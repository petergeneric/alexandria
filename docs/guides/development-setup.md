# Development Setup

## Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- macOS 13+ (for Swift app)
- Xcode 15+ (for Swift app)

## Build

```bash
# Build all crates
cargo build --workspace

# Build in release mode
cargo build --workspace --release
```

## Test

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p alexandria-core
```

## Run

```bash
# Show help
./target/debug/alex --help

# Search
./target/debug/alex search "rust error handling"

# Search with JSON output
./target/debug/alex --json search "rust" --limit 5

# Paginate results
./target/debug/alex search "rust" --limit 5 --offset 10
```

## Project Structure

```
alexandria/
  CLAUDE.md                   # Project context for Claude Code
  Cargo.toml                  # Workspace root
  crates/
    alexandria-core/
      Cargo.toml
      src/
        lib.rs                # Module declarations
        ingest.rs             # PageSnapshot type
        extract.rs            # HTML->Markdown->plaintext extraction
        filter.rs             # Site-specific HTML filtering
        index.rs              # Tantivy schema and indexing
        search.rs             # Search engine with KWIC snippets
        page_store.rs         # SQLite page storage
        queue.rs              # Bounded channel queue
        ffi.rs                # UniFFI bindings for Swift
    alexandria-cli/
      Cargo.toml
      src/
        main.rs               # CLI: search command
  alexandria-app/             # Swift macOS app (SwiftUI + UniFFI)
  extension/                  # Firefox extension
  docs/
    architecture/             # Design documents
    api/                      # API reference
    guides/                   # This file and others
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `htmd` | HTML to Markdown conversion |
| `markdown_to_text` | Markdown to plaintext stripping |
| `tantivy` | Full-text search engine |
| `scraper` | HTML parsing for site-specific filtering |
| `uniffi` | Rust-to-Swift FFI binding generator |
| `clap` | CLI argument parsing |
| `dirs` | Home directory expansion for `~` paths |
| `zstd` | HTML compression in SQLite page store |

## Conventions

- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace`
- Error handling: Use `thiserror` for library errors
- Logging: Use `tracing` macros (`tracing::info!`, `tracing::debug!`, etc.)
- Environment variable `RUST_LOG` controls log level (e.g. `RUST_LOG=debug`)
