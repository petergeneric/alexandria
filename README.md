# Alexandria

<p align="center">
  <img src="docs/icon.svg" alt="Alexandria" width="128" height="128">
</p>

Full-text search for browsing history. Index web pages captured from Firefox and search them instantly.

## Architecture

```
+-----------------+     +----------------+     +------------------+
| Browser Ext.    |---->|  Rust Backend   |<--->|  macOS App       |
| (Firefox)       |     |  (Tantivy)      |     |  (Swift, C FFI)  |
+-----------------+     +----------------+     +------------------+
                              ^
                              |
                        +----------------+
                        | Recoll         |
                        | Webcache Files |
                        +----------------+
```

- **Backend**: Rust with Tantivy full-text search engine
- **Frontend**: Swift macOS app communicating via C FFI
- **Ingestion**: Pluggable sources. Initial: Recoll webcache files. Future: native messaging, HTTP
- **Power-aware**: Pauses indexing queue in Low Power Mode

## Quick Start

```bash
# Build
cargo build --workspace

# Index from webcache directory
cargo run -p alexandria-cli -- index ~/Downloads/webcache

# Search
cargo run -p alexandria-cli -- search "rust async"

# Search with custom index location
cargo run -p alexandria-cli -- --index-dir /tmp/my-index index ~/Downloads/webcache

# Show help
cargo run -p alexandria-cli -- --help
```

Default index location: `./alexandria_index/`

## Project Structure

```
alexandria/
  crates/
    alexandria-core/  # Library: ingestion, indexing, search, FFI
    alexandria-cli/   # CLI binary (`alex`)
  alexandria-app/        # Swift macOS app (future)
  extension/          # Firefox extension (future)
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
