# Alexandria

<p align="center">
  <img src="docs/icon.svg" alt="Alexandria" width="128" height="128">
</p>

Full-text search for browsing history. Index web pages captured from Firefox and search them instantly.

## Architecture

```
+-----------------+     +----------------+     +------------------+
| Browser Ext.    |---->|  Rust Backend  |<--->|  macOS App       |
| (Firefox)       |     |  (Tantivy)     |     |  (Swift, C FFI)  |
+-----------------+     +----------------+     +------------------+
         |                      ^
         v                      |
   +----------------+    +-------------+
   | SQLite         |    | Tantivy     |
   | Page Store     |--->| Search Index|
   +----------------+    +-------------+
```

- **Backend**: Rust with Tantivy full-text search engine
- **Frontend**: Swift macOS app communicating via C FFI
- **Capture**: Firefox extension saves pages to SQLite; background ingestion indexes them into Tantivy
- **Power-aware**: Pauses indexing on low battery and Low Power Mode

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
