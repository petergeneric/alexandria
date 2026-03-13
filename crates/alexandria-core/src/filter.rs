// Site-specific HTML filtering to remove navigation chrome before indexing.

use scraper::{Html, Selector};

/// Apply site-specific filtering to HTML based on the page's domain.
/// Returns cleaned HTML with noise elements removed.
pub fn filter_html(html: &str, domain: &str) -> String {
    match domain {
        "news.ycombinator.com" => filter_by_selectors(html, &HACKERNEWS_REMOVE),
        "www.reddit.com" | "old.reddit.com" => filter_by_selectors(html, &REDDIT_REMOVE),
        "bsky.app" => filter_by_selectors(html, &BLUESKY_REMOVE),
        _ => html.to_string(),
    }
}

/// Hacker News: remove vote links, navigation, user links,
/// timestamps, reply links, and other non-content chrome.
const HACKERNEWS_REMOVE: &[&str] = &[
    ".votelinks",  // up/down vote arrows
    ".navs",       // per-comment navigation (parent, prev, next)
    ".reply",      // reply links
    ".comhead",    // comment headers (user, age, actions)
    ".pagetop",    // top navigation bar
    ".yclinks",    // footer links
    ".morelink",   // "More" pagination link
    ".ind",        // indentation spacers
    ".rank",       // ranking numbers
    ".onstory",    // "on: <story>" links
];

/// Reddit (old): remove sidebar, header, vote arrows, action buttons,
/// report forms, and other non-content chrome.
const REDDIT_REMOVE: &[&str] = &[
    "#header",            // site header, subreddit bar, user nav
    ".side",              // sidebar (subreddit info, rules, submit buttons)
    ".midcol",            // vote arrows column
    ".arrow",             // vote arrows
    ".score",             // score display
    ".tagline",           // user/time metadata per comment
    ".flat-list.buttons", // comment action buttons (reply, share, save, report)
    ".clearleft",         // spacer divs
    ".reportform",        // report forms
    ".numchildren",       // child count indicators
    ".expand",            // expand/collapse buttons
    ".bottommenu",        // footer
    ".usertext-edit",     // reply editor forms
    ".morelink",          // "submit" sidebar buttons
    ".report-button",     // report buttons
];

/// Bluesky: remove navigation, feed tabs, interaction buttons,
/// avatars, SVG icons, and CSS/JS bloat.
const BLUESKY_REMOVE: &[&str] = &[
    "head",                                // CSS, JS, meta tags
    "style",                               // inline style elements
    "script",                              // script elements
    "svg",                                 // icon SVGs
    "nav",                                 // sidebar navigation
    "[data-testid=\"homeScreenFeedTabs\"]", // feed tab bar
    "[data-testid=\"composeFAB\"]",         // compose button
    "[data-testid=\"replyBtn\"]",           // reply buttons
    "[data-testid=\"likeBtn\"]",            // like buttons
    "[data-testid=\"repostBtn\"]",          // repost buttons
    "[data-testid=\"postDropdownBtn\"]",    // dropdown menu buttons
    "[data-testid=\"postShareBtn\"]",       // share buttons
    "[data-testid=\"postBookmarkBtn\"]",    // bookmark buttons
    "[data-testid=\"likeCount\"]",          // like counts
    "[data-testid=\"repostCount\"]",        // repost counts
    "[data-testid=\"userAvatarImage\"]",    // user avatar images
    "[data-testid=\"userAvatarFallback\"]", // user avatar fallbacks
    "[data-testid=\"altTextButton\"]",      // alt text overlay buttons
];

/// Parse HTML and remove all elements matching the given CSS selectors.
fn filter_by_selectors(html: &str, selectors: &[&str]) -> String {
    let document = Html::parse_document(html);

    // Collect node IDs to remove
    let mut remove_ids = std::collections::HashSet::new();
    for sel_str in selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                remove_ids.insert(element.id());
            }
        }
    }

    // Rebuild HTML, skipping removed nodes and their descendants
    let mut output = String::new();
    for node in document.tree.nodes() {
        // Skip if this node or any ancestor is in the remove set
        let mut current = Some(node);
        let mut skip = false;
        while let Some(n) = current {
            if remove_ids.contains(&n.id()) {
                skip = true;
                break;
            }
            current = n.parent();
        }
        if skip {
            continue;
        }

        match node.value() {
            scraper::Node::Element(el) => {
                output.push('<');
                output.push_str(el.name());
                for (name, value) in el.attrs() {
                    output.push(' ');
                    output.push_str(name);
                    output.push_str("=\"");
                    output.push_str(&value.replace('"', "&quot;"));
                    output.push('"');
                }
                output.push('>');
            }
            scraper::Node::Text(text) => {
                output.push_str(text);
            }
            _ => {}
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_unknown_domain_passes_through() {
        let html = "<html><body><p>Hello</p></body></html>";
        assert_eq!(filter_html(html, "example.com"), html);
    }

    #[test]
    fn test_filter_hackernews_removes_votelinks() {
        let html = r#"<html><body>
            <table><tr><td class="votelinks"><a href="vote">▲</a></td></tr></table>
            <span class="commtext c00">Actual comment text</span>
        </body></html>"#;
        let filtered = filter_html(html, "news.ycombinator.com");
        assert!(!filtered.contains("▲"));
        assert!(!filtered.contains("votelinks"));
        assert!(filtered.contains("Actual comment text"));
    }

    #[test]
    fn test_filter_hackernews_removes_comhead() {
        let html = r#"<html><body>
            <span class="comhead"><a class="hnuser" href="user?id=foo">foo</a>
            <span class="age">2 hours ago</span></span>
            <span class="commtext c00">Real content here</span>
        </body></html>"#;
        let filtered = filter_html(html, "news.ycombinator.com");
        assert!(!filtered.contains("hnuser"));
        assert!(!filtered.contains("2 hours ago"));
        assert!(filtered.contains("Real content here"));
    }

    #[test]
    fn test_filter_hackernews_keeps_title_and_toptext() {
        let html = r#"<html><body>
            <span class="titleline"><a href="https://example.com">Cool Article</a></span>
            <div class="toptext">Ask HN body text here</div>
            <span class="commtext c00">A comment</span>
        </body></html>"#;
        let filtered = filter_html(html, "news.ycombinator.com");
        assert!(filtered.contains("Cool Article"));
        assert!(filtered.contains("Ask HN body text here"));
        assert!(filtered.contains("A comment"));
    }

    #[test]
    fn test_filter_reddit_removes_sidebar() {
        let html = r#"<html><body>
            <div class="side"><h1>Subreddit rules</h1></div>
            <div class="commentarea"><div class="md"><p>Great comment</p></div></div>
        </body></html>"#;
        let filtered = filter_html(html, "www.reddit.com");
        assert!(!filtered.contains("Subreddit rules"));
        assert!(filtered.contains("Great comment"));
    }

    #[test]
    fn test_filter_reddit_removes_vote_arrows() {
        let html = r#"<html><body>
            <div class="midcol"><div class="arrow up"></div><div class="arrow down"></div></div>
            <div class="entry"><div class="md"><p>Post content here</p></div></div>
        </body></html>"#;
        let filtered = filter_html(html, "www.reddit.com");
        assert!(!filtered.contains("midcol"));
        assert!(filtered.contains("Post content here"));
    }

    #[test]
    fn test_filter_reddit_removes_header() {
        let html = r#"<html><body>
            <div id="header"><a href="/">reddit</a><div class="tabmenu">nav</div></div>
            <div class="content"><div class="md"><p>Actual post</p></div></div>
        </body></html>"#;
        let filtered = filter_html(html, "www.reddit.com");
        assert!(!filtered.contains("tabmenu"));
        assert!(filtered.contains("Actual post"));
    }

    #[test]
    fn test_filter_reddit_keeps_comment_text() {
        let html = r#"<html><body>
            <div class="comment">
                <div class="tagline">user123 5 hours ago</div>
                <div class="md"><p>This is my insightful comment</p></div>
                <ul class="flat-list buttons"><li>reply</li><li>share</li></ul>
            </div>
        </body></html>"#;
        let filtered = filter_html(html, "www.reddit.com");
        assert!(!filtered.contains("user123"));
        assert!(!filtered.contains("reply"));
        assert!(filtered.contains("This is my insightful comment"));
    }

    #[test]
    fn test_filter_reddit_old_domain() {
        let html = r#"<html><body>
            <div class="side"><p>Sidebar</p></div>
            <div class="md"><p>Content</p></div>
        </body></html>"#;
        let filtered = filter_html(html, "old.reddit.com");
        assert!(!filtered.contains("Sidebar"));
        assert!(filtered.contains("Content"));
    }

    #[test]
    fn test_filter_bluesky_removes_nav_and_buttons() {
        let html = r#"<html><head><style>body{}</style></head><body>
            <nav role="navigation"><a href="/home">Home</a></nav>
            <div data-testid="postText">Hello world post</div>
            <button data-testid="likeBtn">Like</button>
            <button data-testid="replyBtn">Reply</button>
            <button data-testid="repostBtn">Repost</button>
            <button data-testid="composeFAB">New Post</button>
            <svg viewBox="0 0 24 24"><path d="M0 0"></path></svg>
        </body></html>"#;
        let filtered = filter_html(html, "bsky.app");
        assert!(!filtered.contains("navigation"));
        assert!(!filtered.contains("likeBtn"));
        assert!(!filtered.contains("replyBtn"));
        assert!(!filtered.contains("repostBtn"));
        assert!(!filtered.contains("composeFAB"));
        assert!(!filtered.contains("<svg"));
        assert!(!filtered.contains("body{}"));
        assert!(filtered.contains("Hello world post"));
    }

    #[test]
    fn test_filter_bluesky_keeps_post_links() {
        let html = r#"<html><body>
            <a href="/profile/user.bsky.social/post/abc123" aria-label="14 March 2026">3h</a>
            <div data-testid="postText">Important post content</div>
            <button data-testid="postShareBtn">Share</button>
        </body></html>"#;
        let filtered = filter_html(html, "bsky.app");
        assert!(filtered.contains("/profile/user.bsky.social/post/abc123"));
        assert!(filtered.contains("Important post content"));
        assert!(!filtered.contains("postShareBtn"));
    }

    #[test]
    fn test_filter_bluesky_removes_feed_tabs() {
        let html = r#"<html><body>
            <div data-testid="homeScreenFeedTabs"><div role="tab">Following</div></div>
            <div data-testid="postText">A post</div>
        </body></html>"#;
        let filtered = filter_html(html, "bsky.app");
        assert!(!filtered.contains("homeScreenFeedTabs"));
        assert!(filtered.contains("A post"));
    }

    #[test]
    fn test_filter_bluesky_removes_avatars_and_counts() {
        let html = r#"<html><body>
            <img data-testid="userAvatarImage" src="avatar.jpg">
            <div data-testid="userAvatarFallback">U</div>
            <div data-testid="likeCount">42</div>
            <div data-testid="repostCount">7</div>
            <div data-testid="postText">Post text here</div>
        </body></html>"#;
        let filtered = filter_html(html, "bsky.app");
        assert!(!filtered.contains("userAvatarImage"));
        assert!(!filtered.contains("userAvatarFallback"));
        assert!(!filtered.contains("likeCount"));
        assert!(!filtered.contains("repostCount"));
        assert!(filtered.contains("Post text here"));
    }
}
