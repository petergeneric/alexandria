# Search Architecture

## Tantivy Configuration

- **Index directory**: `./alexandria_index/` (CLI default, configurable via `--index-dir`)
- **Writer heap**: 50 MB
- **Reader**: Reloaded on each search to pick up new commits

## Query Parsing

Queries are parsed by Tantivy's `QueryParser` across three fields:

| Field | Boost | Rationale |
|-------|-------|-----------|
| `title` | 3.0x | Title matches are most relevant |
| `domain` | 2.0x | Domain matches indicate topical relevance |
| `content` | 1.0x | Full-text body match (baseline) |

The `content` field contains plaintext (markdown stripped), so search terms match against clean text without markdown syntax noise.

### Query Syntax

Tantivy supports:
- Simple terms: `rust async`
- Phrases: `"error handling"`
- Field-specific: `title:rust`
- Boolean: `rust AND async`, `rust OR go`
- Exclusion: `rust -beginner`

## SearchResult

```rust
pub struct SearchResult {
    pub url: String,
    pub title: String,
    pub content_snippet: String,  // KWIC plaintext snippet
    pub markdown: String,         // full stored markdown
    pub domain: String,
    pub score: f32,
}
```

Results are returned sorted by relevance score (descending).

## Snippet Generation

Snippets are generated at search time using keyword-in-context (KWIC) extraction:

1. Read stored markdown from the index
2. Convert to plaintext via `markdown_to_text`
3. Find the earliest query keyword match in the plaintext
4. Extract a ~200 character window centered on the match, snapping to word boundaries
5. Add `...` ellipsis when the snippet doesn't start/end at the text boundary
6. If no keyword found in text, fall back to showing the beginning

### Keyword Highlighting

The CLI highlights matched keywords in titles and snippets using ANSI bold yellow (`\x1b[1;33m`). Boolean operators (AND, OR, NOT) are excluded from highlighting.

## Raw Output

The `--raw` flag outputs the full stored markdown instead of a KWIC snippet, useful for inspecting the complete converted content.

## Pagination

- `limit`: Maximum number of results (default: 10)
- `offset`: Number of results to skip (default: 0)
- Uses Tantivy's `TopDocs::with_limit().and_offset()`

## Future: Vector Search

Planned for Phase 6:
- Generate embeddings for page content using a local model
- Store embeddings alongside Tantivy documents
- Support semantic search queries ("pages about error handling patterns" vs exact term match)
- Hybrid scoring: combine BM25 and vector similarity
