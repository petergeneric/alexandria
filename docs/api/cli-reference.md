# CLI Reference

## Usage

```
alex [OPTIONS] <COMMAND>
```

## Global Options

| Flag | Description | Default |
|------|-------------|---------|
| `--index-dir <PATH>` | Path to the Tantivy index directory | `./alexandria_index` |
| `--json` | Output results as JSON | off |
| `--help` | Show help | |
| `--version` | Show version | |

## Commands

### `index`

Index new pages from a Recoll webcache directory. Skips files already processed (tracked via `.last-indexed` timestamp in the index directory). Also skips documents already in the index by `source_hash`.

```
alex index [SOURCE]
```

| Argument | Description | Default |
|----------|-------------|---------|
| `SOURCE` | Path to the webcache directory | `~/Downloads/webcache` |

**Exit codes**: 0 = success, 1 = error (source not found, index write failure)

### `reindex`

Delete the existing index and rebuild from scratch.

```
alex reindex [SOURCE]
```

| Argument | Description | Default |
|----------|-------------|---------|
| `SOURCE` | Path to the webcache directory | `~/Downloads/webcache` |

Removes the entire index directory (including `.last-indexed`), then performs a full index.

**Exit codes**: 0 = success, 1 = error

### `search`

Search indexed pages.

```
alex search [OPTIONS] <QUERY>
```

| Argument/Flag | Description | Default |
|---------------|-------------|---------|
| `QUERY` | Search query (required) | |
| `-l, --limit` | Maximum results | 10 |
| `-o, --offset` | Result offset for pagination | 0 |
| `--raw` | Show full stored markdown instead of snippet | off |

**Output** (default): One result per block with title, URL, domain, score, and KWIC snippet with keyword highlighting (ANSI bold yellow).

**Output** (`--raw`): Full stored markdown content per result instead of snippet.

**Output** (`--json`): JSON array of `SearchResult` objects (includes both `content_snippet` and `markdown` fields).

**Exit codes**: 0 = success (even if no results), 1 = error (index not found, query parse failure)

### `watch` (Phase 2)

Watch a directory for new webcache files and index them continuously.

```
alex watch [SOURCE]
```

Performs a bulk scan on startup, then watches for new files. Respects Low Power Mode.

### `info` (Phase 2)

Show index statistics (document count, index size, index directory path).
