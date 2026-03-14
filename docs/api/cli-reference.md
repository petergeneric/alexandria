# CLI Reference

## Usage

```
alex [OPTIONS] <COMMAND>
```

## Global Options

| Flag | Description | Default |
|------|-------------|---------|
| `--index-dir <PATH>` | Path to the index directory | auto-detected (see below) |
| `--json` | Output results as JSON | off |
| `--help` | Show help | |
| `--version` | Show version | |

### Index Discovery

If `--index-dir` is not provided, the CLI searches for an existing index (identified by a `meta.json` file) in this order:

1. Current working directory
2. `~/Library/Application Support/works.peter.alexandria/index`

If no index is found, the CLI exits with an error listing the locations it checked and a hint to use `--index-dir`.

## Commands

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
| `--raw` | Show full stored HTML instead of snippet | off |

**Output** (default): One result per block with title, URL, domain, relative time, score, and KWIC snippet with keyword highlighting (ANSI bold yellow).

**Output** (`--raw`): Full stored HTML content per result instead of snippet.

**Output** (`--json`): JSON array of `SearchResult` objects (includes both `content_snippet` and `html` fields).

**Exit codes**: 0 = success (even if no results), 1 = error (index not found, query parse failure)

