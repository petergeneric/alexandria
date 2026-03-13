# Ingestion Architecture

## IngestSource Trait

```rust
pub trait IngestSource {
    fn scan(&self) -> Result<Vec<PageSnapshot>, IngestError>;
}
```

All ingestion sources implement this trait. The engine doesn't care where pages come from -- it receives `PageSnapshot` structs through a uniform interface.

### Implementations

| Source | Status | Description |
|--------|--------|-------------|
| `RecollFileSource` | Done | Reads paired webcache files from a directory |
| Native Messaging | Future | Receives pages directly from browser extension |
| HTTP Server | Future | Accepts page snapshots via HTTP POST |

## RecollFileSource

The initial implementation reads from Recoll's webcache directory.

### Configuration

```rust
pub struct RecollFileSource {
    pub cache_dir: PathBuf,
    pub modified_since: Option<SystemTime>,  // for incremental indexing
}
```

### Scan Algorithm

1. **First pass**: Collect all metadata files (`recoll-we-m-*.rclwe`) into a HashMap keyed by MD5 hash
2. **Second pass**: For each content file (`recoll-we-c-*.rclwe`):
   - If `modified_since` is set, skip files with mtime ≤ the threshold
   - Extract the MD5 hash from the filename
   - Look up the paired metadata file (skip if missing)
   - Parse metadata for URL (line 0) and MIME type (line 2)
   - Read raw HTML content
   - Convert HTML to Markdown via `htmd` (skipping script/style/nav/footer/header, stripping bold/italic)
   - Extract title from `<title>` tag
   - Extract domain from URL
   - Convert markdown to plaintext via `markdown_to_text`
   - Build a `PageSnapshot` with both markdown and plaintext

### Paired-File Detection

Both files must exist for processing:

```
recoll-we-m-a1b2c3d4e5f6...rclwe  (metadata)
recoll-we-c-a1b2c3d4e5f6...rclwe  (content)
```

The hash is extracted by stripping the prefix (`recoll-we-m-` or `recoll-we-c-`) and suffix (`.rclwe`).

### Extraction Pipeline

```
Raw HTML
  → htmd (HTML to Markdown, skip nav/script/style, strip bold/italic)
  → markdown_to_text (Markdown to plaintext, clean table pipes)
  → extract_title (simple <title> tag parser)
  → extract_domain (url crate)
```

## FileWatcher Design (Phase 2)

For the `watch` command, a filesystem watcher monitors the webcache directory:

1. Use `notify` crate to watch for new file creation events
2. On file creation, check if the paired file exists
3. When a complete pair is detected, build a `PageSnapshot` and enqueue it
4. On startup, perform a bulk scan of existing files first

## Queue Architecture (Phase 2)

Ingestion and indexing are decoupled via a bounded crossbeam channel:

```
[IngestSource] --scan()--> [PageSnapshot] --try_send()--> [Queue] --recv()--> [Indexer]
```

- **Capacity**: Configurable (default: 1000)
- **Backpressure**: If the queue is full, `try_send` returns false and the snapshot is dropped with a warning
- **Power-aware**: The consumer side can pause/resume based on power state without losing enqueued data
