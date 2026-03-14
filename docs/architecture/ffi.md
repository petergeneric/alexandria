# UniFFI Architecture

## Overview

The macOS Swift app communicates with `alexandria-core` through [UniFFI](https://mozilla.github.io/uniffi-rs/), Mozilla's Rust-to-Swift binding generator. UniFFI uses proc-macro annotations on Rust types to automatically generate Swift bindings, a C FFI header, and a module map.

## Rust Interface

The FFI surface is defined in `crates/core/src/ffi.rs` using UniFFI proc-macros:

- `AlexandriaEngine` — `#[derive(uniffi::Object)]` wrapping the search engine and index
- `AlexandriaSearchResult` — `#[derive(uniffi::Record)]` (plain data struct) for results
- `PendingStatus` — `#[derive(uniffi::Record)]` for pending page counts
- `AlexandriaError` — `#[derive(uniffi::Error)]` enum for error propagation

## Generated Swift API

UniFFI generates a native Swift class with automatic memory management. Errors are thrown as `AlexandriaError` Swift enums with associated values. ARC handles deallocation — no manual memory management needed.

## Regenerating Bindings

After changing the Rust FFI interface, regenerate Swift bindings:

```bash
cargo build -p alexandria-core
cargo run --bin uniffi-bindgen generate \
    --library target/debug/libalexandria_core.dylib \
    --language swift \
    --out-dir macos/Sources/Alexandria

# Copy header to module map location
cp macos/Sources/Alexandria/alexandria_coreFFI.h \
   macos/Sources/alexandria_coreFFI/
```
