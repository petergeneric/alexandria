![LLM Generated](../llm-generated.svg)

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
| `--store` | Path to the page store database (for snippets) | auto-detected (see below) |

### Store Discovery

If `--store` is not provided, the CLI looks for `pages.db` at:

1. `~/Library/Application Support/works.peter.alexandria/pages.db`

If found, snippets are generated from stored HTML. If not found, snippets are omitted.

**Output** (default): One result per block with title, URL, domain, relative time, and KWIC snippet with keyword highlighting (ANSI bold yellow).

**Output** (`--json`): JSON array of `SearchResult` objects.

**Exit codes**: 0 = success (even if no results), 1 = error (index not found, query parse failure)

### `import-firefox`

Import browsing history from a Firefox `places.sqlite` database. This is a one-time helper for bootstrapping your index with existing history — not part of the normal capture flow.

```
alex import-firefox [OPTIONS]
```

| Argument/Flag | Description | Default |
|---------------|-------------|---------|
| `--places <PATH>` | Path to Firefox `places.sqlite` | `/tmp/places.sqlite` |
| `--store` | Path to the page store database | auto-detected |

Pages are filtered through the shared URL blocklist (banks, auth pages, checkout flows) and deduplicated by content hash before insertion.

