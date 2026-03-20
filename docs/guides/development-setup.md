![LLM Generated](../llm-generated.svg)

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

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tantivy` | Full-text search engine |
| `scraper` | HTML parsing for site-specific filtering and plaintext extraction |
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
