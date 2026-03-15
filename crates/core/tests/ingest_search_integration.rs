//! Integration tests covering the full pipeline:
//! PageStore insert → Tantivy indexing → search → snippet generation.

use alexandria_core::extract;
use alexandria_core::filter;
use alexandria_core::index::{index_snapshots, open_or_create_index, SchemaFields};
use alexandria_core::ingest::PageSnapshot;
use alexandria_core::page_store::PageStore;
use alexandria_core::search::SearchEngine;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a temp directory with a unique name for each test.
fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "alexandria-integration-{}-{}",
        std::process::id(),
        id
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Mirrors the native host's storage logic: HTML for filtered domains, plaintext otherwise.
fn store_content(html: &str, domain: &str) -> String {
    if filter::has_filter(domain) {
        if html.starts_with('<') {
            html.to_string()
        } else {
            format!("<!doctype html>{html}")
        }
    } else {
        let plaintext = extract::html_to_plaintext(html);
        if plaintext.starts_with('<') {
            format!(" {plaintext}")
        } else {
            plaintext
        }
    }
}

/// Mirrors the indexer's read-back logic: HTML content gets filtered+extracted,
/// plaintext content is used as-is.
fn content_to_plaintext(stored: &str, domain: &str) -> String {
    if stored.starts_with('<') {
        let filtered = filter::filter_html(stored, domain);
        extract::html_to_plaintext(&filtered)
    } else {
        stored.to_string()
    }
}

fn content_hash(data: &[u8]) -> [u8; 16] {
    xxhash_rust::xxh3::xxh3_128(data).to_le_bytes()
}

struct TestHarness {
    store: PageStore,
    engine: SearchEngine,
    index: tantivy::Index,
    _dir: PathBuf,
}

impl TestHarness {
    fn new() -> Self {
        let dir = temp_dir();
        let store = PageStore::open(&dir.join("pages.db")).unwrap();
        let index = open_or_create_index(&dir.join("index")).unwrap();
        let engine = SearchEngine::new(index.clone()).unwrap();
        Self {
            store,
            engine,
            index,
            _dir: dir,
        }
    }

    /// Insert a page using the same logic as the native host.
    /// The native host hashes the raw incoming HTML (before transformation),
    /// so we do the same here.
    fn insert_page(&self, url: &str, title: &str, html: &str, domain: &str, captured_at: i64) {
        let hash = content_hash(html.as_bytes());
        let content = store_content(html, domain);
        self.store
            .insert(url, title, content.as_bytes(), domain, captured_at, &hash)
            .unwrap();
    }

    /// Index all pages after the given watermark, mirroring ffi.rs logic.
    fn index_pages(&self, watermark: i64) -> usize {
        let pages = self.store.pages_after(watermark, 1000).unwrap();
        let snapshots: Vec<PageSnapshot> = pages
            .iter()
            .map(|p| {
                let content = content_to_plaintext(&p.html, &p.domain);
                PageSnapshot {
                    page_id: p.id,
                    url: p.url.clone(),
                    title: p.title.clone(),
                    content,
                    domain: p.domain.clone(),
                    captured_at: p.captured_at,
                }
            })
            .collect();

        let fields = SchemaFields::from_index(&self.index).unwrap();
        let mut writer = self.index.writer(15_000_000).unwrap();
        let indexed = index_snapshots(&mut writer, &fields, snapshots).unwrap();
        indexed
    }

    fn search(&self, query: &str) -> Vec<alexandria_core::search::SearchResult> {
        self.engine
            .search(query, 10, 0, Some(&self.store))
            .unwrap()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn test_plaintext_site_roundtrip() {
    let h = TestHarness::new();

    let html = r#"<html><head><title>Rust Guide</title></head>
        <body><p>Rust is a systems programming language focused on safety and performance.</p></body></html>"#;

    h.insert_page(
        "https://example.com/rust",
        "Rust Guide",
        html,
        "example.com",
        1000,
    );
    let indexed = h.index_pages(0);
    assert_eq!(indexed, 1);

    let results = h.search("systems programming safety");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://example.com/rust");
    assert_eq!(results[0].title, "Rust Guide");
    assert_eq!(results[0].domain, "example.com");
    assert!(
        results[0].content_snippet.contains("safety"),
        "snippet should contain search keyword, got: {:?}",
        results[0].content_snippet
    );
}

#[test]
fn test_filtered_site_roundtrip() {
    let h = TestHarness::new();

    let html = r#"<html><body>
        <span class="votelinks"><a href="vote">▲</a></span>
        <span class="comhead"><a class="hnuser">user1</a> <span class="age">2h</span></span>
        <span class="commtext c00">Interesting discussion about memory safety in Rust</span>
        <span class="navs"><a>parent</a></span>
    </body></html>"#;

    h.insert_page(
        "https://news.ycombinator.com/item?id=123",
        "HN Discussion",
        html,
        "news.ycombinator.com",
        2000,
    );
    let indexed = h.index_pages(0);
    assert_eq!(indexed, 1);

    // Should find the comment content
    let results = h.search("memory safety");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].domain, "news.ycombinator.com");

    // Snippet should contain the comment text, not the chrome
    let snippet = &results[0].content_snippet;
    assert!(
        snippet.contains("memory safety"),
        "snippet should contain keyword, got: {:?}",
        snippet
    );
    assert!(
        !snippet.contains("▲"),
        "snippet should not contain vote arrows"
    );
    assert!(
        !snippet.contains("hnuser"),
        "snippet should not contain user chrome"
    );
}

#[test]
fn test_mixed_sites_search() {
    let h = TestHarness::new();

    // A regular site (stored as plaintext)
    h.insert_page(
        "https://blog.example.com/post",
        "Blog Post",
        "<html><body><p>Concurrency patterns in modern software development</p></body></html>",
        "blog.example.com",
        1000,
    );

    // A filtered site (stored as HTML)
    let hn_html = r#"<html><body>
        <span class="votelinks"><a>▲</a></span>
        <span class="commtext c00">Concurrency is hard but Rust makes it easier</span>
    </body></html>"#;
    h.insert_page(
        "https://news.ycombinator.com/item?id=456",
        "HN on Concurrency",
        hn_html,
        "news.ycombinator.com",
        2000,
    );

    h.index_pages(0);

    let results = h.search("concurrency");
    assert_eq!(results.len(), 2, "both pages should match 'concurrency'");

    // Both snippets should contain the keyword
    for r in &results {
        assert!(
            r.content_snippet.to_lowercase().contains("concurrency"),
            "snippet for {} should contain 'concurrency', got: {:?}",
            r.url,
            r.content_snippet
        );
    }
}

#[test]
fn test_incremental_indexing() {
    let h = TestHarness::new();

    h.insert_page(
        "https://a.com/1",
        "Page A",
        "<html><body><p>Alpha content here</p></body></html>",
        "a.com",
        1000,
    );
    h.index_pages(0);

    let pages = h.store.pages_after(0, 10).unwrap();
    let watermark = pages.last().unwrap().id;

    // Insert more pages after the first batch
    h.insert_page(
        "https://b.com/2",
        "Page B",
        "<html><body><p>Beta content here</p></body></html>",
        "b.com",
        2000,
    );
    let indexed = h.index_pages(watermark);
    assert_eq!(indexed, 1, "only the new page should be indexed");

    // Both should be searchable
    let results = h.search("alpha");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://a.com/1");

    let results = h.search("beta");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://b.com/2");
}

#[test]
fn test_snippet_from_stored_plaintext() {
    let h = TestHarness::new();

    // A long page so the snippet has to extract a KWIC window
    let body = "Lorem ipsum dolor sit amet. ".repeat(50);
    let keyword_section = "The quantum computing revolution will transform cryptography forever.";
    let trailing = " Vestibulum ante ipsum primis. ".repeat(50);
    let html = format!(
        "<html><body><p>{body}</p><p>{keyword_section}</p><p>{trailing}</p></body></html>"
    );

    h.insert_page(
        "https://example.com/long",
        "Long Article",
        &html,
        "example.com",
        1000,
    );
    h.index_pages(0);

    let results = h.search("quantum cryptography");
    assert_eq!(results.len(), 1);
    let snippet = &results[0].content_snippet;
    assert!(
        snippet.contains("quantum"),
        "KWIC snippet should center on keyword, got: {:?}",
        snippet
    );
}

#[test]
fn test_snippet_from_stored_html_with_filter() {
    let h = TestHarness::new();

    // Reddit page with boilerplate — snippet should come from filtered content
    let html = r#"<html><body>
        <div id="header"><a href="/">reddit</a></div>
        <div class="side"><h1>Subreddit rules</h1><p>Be nice</p></div>
        <div class="midcol"><div class="arrow up"></div></div>
        <div class="entry"><div class="md">
            <p>Functional programming with algebraic data types provides strong guarantees</p>
        </div></div>
        <ul class="flat-list buttons"><li>reply</li><li>share</li></ul>
    </body></html>"#;

    h.insert_page(
        "https://www.reddit.com/r/programming/comments/abc",
        "Reddit Discussion",
        html,
        "www.reddit.com",
        1000,
    );
    h.index_pages(0);

    let results = h.search("algebraic data types");
    assert_eq!(results.len(), 1);
    let snippet = &results[0].content_snippet;
    assert!(
        snippet.contains("algebraic"),
        "snippet should contain keyword from filtered content, got: {:?}",
        snippet
    );
    assert!(
        !snippet.contains("Subreddit rules"),
        "snippet should not contain sidebar content"
    );
}

#[test]
fn test_plaintext_not_starting_with_angle_bracket() {
    let h = TestHarness::new();

    // Edge case: HTML whose plaintext extraction starts with '<'
    // The native host should prefix with a space
    let html = "<html><body><p>&lt;script&gt; is a dangerous tag</p></body></html>";

    h.insert_page(
        "https://example.com/edge",
        "Edge Case",
        html,
        "example.com",
        1000,
    );

    // Verify what was stored doesn't start with '<' (would be misidentified as HTML)
    let pages = h.store.pages_after(0, 10).unwrap();
    assert!(
        !pages[0].html.starts_with('<'),
        "stored plaintext should not start with '<', got: {:?}",
        &pages[0].html[..20.min(pages[0].html.len())]
    );

    h.index_pages(0);
    let results = h.search("dangerous tag");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_visited_at_populated() {
    let h = TestHarness::new();

    h.insert_page(
        "https://example.com/time",
        "Time Test",
        "<html><body><p>Temporal content</p></body></html>",
        "example.com",
        1700000000, // 2023-11-14
    );
    h.index_pages(0);

    let results = h.search("temporal");
    assert_eq!(results.len(), 1);
    assert!(
        results[0].visited_at.is_some(),
        "visited_at should be populated from captured_at"
    );
    assert_eq!(results[0].visited_at.unwrap().timestamp(), 1700000000);
}

#[test]
fn test_bluesky_filtered_site_roundtrip() {
    let h = TestHarness::new();

    let html = r#"<html><head><style>body{color:red}</style></head><body>
        <nav role="navigation"><a href="/home">Home</a></nav>
        <svg viewBox="0 0 24 24"><path d="M0 0"></path></svg>
        <div data-testid="postText">Distributed systems are fascinating to study</div>
        <button data-testid="likeBtn">Like</button>
        <button data-testid="replyBtn">Reply</button>
        <div data-testid="likeCount">42</div>
        <img data-testid="userAvatarImage" src="avatar.jpg">
    </body></html>"#;

    h.insert_page(
        "https://bsky.app/profile/user/post/abc",
        "Bluesky Post",
        html,
        "bsky.app",
        3000,
    );
    h.index_pages(0);

    let results = h.search("distributed systems");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].domain, "bsky.app");

    let snippet = &results[0].content_snippet;
    assert!(
        snippet.contains("distributed") || snippet.contains("Distributed"),
        "snippet should contain post content, got: {:?}",
        snippet
    );
    assert!(
        !snippet.contains("Like"),
        "snippet should not contain button chrome"
    );
    assert!(
        !snippet.contains("avatar"),
        "snippet should not contain avatar references"
    );
}

#[test]
fn test_filtered_domain_html_without_leading_angle_bracket() {
    let h = TestHarness::new();

    // HTML that doesn't start with '<' — native host prepends <!doctype html>
    let html = " <html><body>\
        <span class=\"commtext c00\">Compilers are underappreciated tools</span>\
    </body></html>";

    h.insert_page(
        "https://news.ycombinator.com/item?id=789",
        "HN Compilers",
        html,
        "news.ycombinator.com",
        4000,
    );

    // Verify the stored content starts with '<' (the doctype prefix)
    let pages = h.store.pages_after(0, 10).unwrap();
    assert!(
        pages[0].html.starts_with('<'),
        "stored HTML for filtered site should start with '<', got: {:?}",
        &pages[0].html[..30.min(pages[0].html.len())]
    );

    h.index_pages(0);
    let results = h.search("compilers underappreciated");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_unicode_kwic_snippet() {
    let h = TestHarness::new();

    // Mix of CJK, emoji, and accented characters to exercise char-boundary handling
    let body_prefix = "日本語のテキスト。".repeat(30);
    let keyword_section = "Résumé: the naïve café served crème brûlée 🎉 with açaí bowls";
    let body_suffix = "更多中文内容在这里。".repeat(30);
    let html = format!(
        "<html><body><p>{body_prefix}</p><p>{keyword_section}</p><p>{body_suffix}</p></body></html>"
    );

    h.insert_page(
        "https://example.com/unicode",
        "Unicode Test",
        &html,
        "example.com",
        5000,
    );
    h.index_pages(0);

    // Search for a term that Tantivy's tokenizer will match
    let results = h.search("naïve brûlée");
    assert_eq!(results.len(), 1);
    // The key assertion: no panic from slicing mid-codepoint in kwic_snippet.
    // The snippet may or may not center on the keyword (depends on Tantivy's
    // tokenization of accented chars), but it must not crash.
    let snippet = &results[0].content_snippet;
    assert!(
        !snippet.is_empty(),
        "snippet should be non-empty for multibyte content"
    );
}

#[test]
fn test_search_without_page_store() {
    let h = TestHarness::new();

    h.insert_page(
        "https://example.com/no-store",
        "No Store",
        "<html><body><p>Ephemeral content for testing</p></body></html>",
        "example.com",
        1000,
    );
    h.index_pages(0);

    // Search without a page store — snippets should be empty
    let results = h.engine.search("ephemeral", 10, 0, None).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].url, "https://example.com/no-store");
    assert!(
        results[0].content_snippet.is_empty(),
        "snippet should be empty without page store, got: {:?}",
        results[0].content_snippet
    );
}

#[test]
fn test_no_results_for_absent_term() {
    let h = TestHarness::new();

    h.insert_page(
        "https://example.com/page",
        "A Page",
        "<html><body><p>Some ordinary content</p></body></html>",
        "example.com",
        1000,
    );
    h.index_pages(0);

    let results = h.search("xylophone");
    assert!(results.is_empty(), "should find no results for absent term");
}

#[test]
fn test_mixed_sites_search_identifies_sources() {
    let h = TestHarness::new();

    h.insert_page(
        "https://blog.example.com/post",
        "Blog Post",
        "<html><body><p>Monomorphization in compiled languages</p></body></html>",
        "blog.example.com",
        1000,
    );

    let hn_html = r#"<html><body>
        <span class="commtext c00">Monomorphization eliminates virtual dispatch overhead</span>
    </body></html>"#;
    h.insert_page(
        "https://news.ycombinator.com/item?id=999",
        "HN on Monomorphization",
        hn_html,
        "news.ycombinator.com",
        2000,
    );

    h.index_pages(0);

    let results = h.search("monomorphization");
    assert_eq!(results.len(), 2);

    let urls: Vec<&str> = results.iter().map(|r| r.url.as_str()).collect();
    assert!(
        urls.contains(&"https://blog.example.com/post"),
        "should find blog post"
    );
    assert!(
        urls.contains(&"https://news.ycombinator.com/item?id=999"),
        "should find HN page"
    );

    let domains: Vec<&str> = results.iter().map(|r| r.domain.as_str()).collect();
    assert!(domains.contains(&"blog.example.com"));
    assert!(domains.contains(&"news.ycombinator.com"));
}

#[test]
fn test_snippet_length_bounded() {
    let h = TestHarness::new();

    let body = "Extremely verbose content about serialization frameworks. ".repeat(200);
    let html = format!("<html><body><p>{body}</p></body></html>");

    h.insert_page(
        "https://example.com/verbose",
        "Verbose Article",
        &html,
        "example.com",
        1000,
    );
    h.index_pages(0);

    let results = h.search("serialization");
    assert_eq!(results.len(), 1);
    let snippet = &results[0].content_snippet;
    // KWIC targets ~200 chars + possible "..." ellipsis (6 chars)
    assert!(
        snippet.len() <= 300,
        "snippet should be bounded, got {} chars: {:?}",
        snippet.len(),
        snippet
    );
}
