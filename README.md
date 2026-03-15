# Alexandria

<p align="center">
  <img src="docs/icon.svg" alt="Alexandria" width="128" height="128">
</p>

Full-text search for browsing history. Index web pages captured from Firefox and search them instantly.

## Architecture

<p align="center">
  <img src="docs/architecture.svg" alt="Architecture diagram" width="720">
</p>

- **Capture**: Firefox extension grabs page HTML and sends it via native messaging to a Rust host that deduplicates and stores it in `pages.db` (zstd-compressed)
- **Ingestion**: Core library reads stored pages, filters boilerplate (site-specific CSS selectors), converts HTML to plaintext, and batch-indexes into Tantivy
- **Search**: Queries run against the Tantivy index with field boosting (title 3x, domain 2x, content 1x); snippets are generated at search time via KWIC
- **Frontends**: macOS app (SwiftUI + UniFFI) and CLI (`alex`) both use the core library
- **Power-aware**: macOS app pauses indexing on low battery and Low Power Mode
- **Shared blocklist**: `blocklist.json` filters sensitive URLs in both the extension and the indexer

## Quick Start

```bash
# Build
cargo build --workspace

# Search
./target/debug/alex search "rust async"

# Show help
./target/debug/alex --help
```

## Project Structure

```
alexandria/
  crates/
    core/             # Library: ingestion, indexing, search, FFI
    cli/              # CLI binary (`alex`)
    browser-native-host/ # Native messaging host for Firefox
  macos/              # Swift macOS app (SwiftUI + UniFFI)
  extension/          # Firefox extension
  docs/
    architecture/     # Design docs
    api/              # API reference
    guides/           # Development guides
```

## Documentation

- [Architecture Overview](docs/architecture/README.md)
- [Data Model](docs/architecture/data-model.md)
- [Ingestion](docs/architecture/ingestion.md)
- [Search](docs/architecture/search.md)
- [FFI](docs/architecture/ffi.md)
- [Power Management](docs/architecture/power-management.md)
- [Rust Public API](docs/api/rust-public-api.md)
- [CLI Reference](docs/api/cli-reference.md)
- [Development Setup](docs/guides/development-setup.md)
- [Roadmap](docs/roadmap.md)


## License

[GNU Affero General Public License v3.0](LICENSE.txt)
