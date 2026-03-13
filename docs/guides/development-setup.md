# Development Setup

## Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- macOS 13+ (for Swift app, future)
- Xcode 15+ (for Swift app, future)

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
cargo run -p alexandria-cli -- --help

# Index webcache files (incremental, skips already-indexed)
cargo run -p alexandria-cli -- index ~/Downloads/webcache

# Reindex from scratch (deletes existing index first)
cargo run -p alexandria-cli -- reindex ~/Downloads/webcache

# Search
cargo run -p alexandria-cli -- search "rust error handling"

# Search with JSON output
cargo run -p alexandria-cli -- --json search "rust" --limit 5

# Search with full markdown output
cargo run -p alexandria-cli -- search "rust" --raw

# Paginate results
cargo run -p alexandria-cli -- search "rust" --limit 5 --offset 10
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
        ingest.rs             # IngestSource trait + RecollFileSource
        extract.rs            # HTML→Markdown→plaintext extraction
        index.rs              # Tantivy schema and indexing
        search.rs             # Search engine with KWIC snippets
        queue.rs              # Bounded channel queue (Phase 2)
        power.rs              # Low Power Mode detection (Phase 3)
        ffi.rs                # C FFI (future)
    alexandria-cli/
      Cargo.toml
      src/
        main.rs               # CLI: index, reindex, search commands
  alexandria-app/                # Swift macOS app (future)
  extension/                  # Firefox extension (future)
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
| `clap` | CLI argument parsing |
| `dirs` | Home directory expansion for `~` paths |

## Conventions

- Format: `cargo fmt --all`
- Lint: `cargo clippy --workspace`
- Error handling: Use `thiserror` for library errors
- Logging: Use `tracing` macros (`tracing::info!`, `tracing::debug!`, etc.)
- Environment variable `RUST_LOG` controls log level (e.g. `RUST_LOG=debug`)
