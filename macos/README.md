# Alexandria macOS App

Swift macOS frontend for Alexandria. Communicates with the Rust backend via C FFI.

**Status**: Not yet implemented (Phase 2).

## Architecture

See [FFI Architecture](../docs/architecture/ffi.md) for the C API surface and Swift integration details.

## Setup (future)

1. Build the Rust static library: `cargo build -p alexandria-core --release`
2. Copy `target/release/libalexandria_core.a` and `target/alexandria.h` to the Xcode project
3. Add `alexandria.h` as a bridging header
4. Build and run from Xcode
