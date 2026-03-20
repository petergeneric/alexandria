//! HTML extraction utilities: iterative plaintext extraction via `scraper`,
//! title extraction, URL extraction, and domain extraction.

use scraper::{Html, Node};

/// Tags whose text content should be excluded from plaintext output.
const SKIP_TAGS: &[&str] = &["script", "style", "noscript"];

/// Convert HTML to plain text for search indexing.
///
/// Iterates the parsed DOM tree (no recursion) and collects text nodes,
/// skipping `<script>`, `<style>`, and `<noscript>` elements.
pub fn html_to_plaintext(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut parts: Vec<&str> = Vec::new();

    for node in document.tree.nodes() {
        if let Node::Text(text) = node.value() {
            // Skip text inside excluded elements.
            let dominated_by_skip = node.ancestors().any(|ancestor| {
                ancestor
                    .value()
                    .as_element()
                    .is_some_and(|el| SKIP_TAGS.contains(&el.name()))
            });
            if !dominated_by_skip {
                let t = text.trim();
                if !t.is_empty() {
                    parts.push(t);
                }
            }
        }
    }

    parts.join(" ")
}

/// Extract the `<title>` from an HTML document using simple string parsing.
pub fn extract_title(html: &str) -> String {
    extract_title_inner(html).unwrap_or_default()
}

fn extract_title_inner(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title")?;
    let tag_end = lower.get(start..)?.find('>')? + start + 1;
    if tag_end > lower.len() {
        return None;
    }
    let end = lower.get(tag_end..)?.find("</title")? + tag_end;
    if end > html.len() {
        return None;
    }
    let title = html.get(tag_end..end)?.trim();
    Some(title.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Extract a URL from HTML via <link rel="canonical"> or <meta property="og:url">.
pub fn extract_url_from_html(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    extract_canonical(html, &lower).or_else(|| extract_og_url(html, &lower))
}

fn extract_meta_attr(html: &str, lower: &str, selector: &str, attr: &str) -> Option<String> {
    let idx = lower.find(selector)?;
    let tag_start = lower[..idx].rfind('<')?;
    let tag_end = lower[idx..].find('>')? + idx;
    let tag = &html[tag_start..=tag_end];
    extract_attr_value(tag, attr)
}

fn extract_canonical(html: &str, lower: &str) -> Option<String> {
    extract_meta_attr(html, lower, "rel=\"canonical\"", "href")
}

fn extract_og_url(html: &str, lower: &str) -> Option<String> {
    extract_meta_attr(html, lower, "property=\"og:url\"", "content")
}

fn extract_attr_value(tag: &str, attr: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let needle = format!("{attr}=\"");
    let idx = lower.find(&needle)?;
    let val_start = idx + needle.len();
    let val_end = tag[val_start..].find('"')? + val_start;
    let url = tag[val_start..val_end].trim();
    if url.starts_with("http") { Some(url.to_string()) } else { None }
}

/// Extract the domain from a URL, stripping any `www.` prefix.
pub fn extract_domain(url_str: &str) -> String {
    url::Url::parse(url_str)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .map(|h| h.strip_prefix("www.").unwrap_or(&h).to_string())
        .unwrap_or_default()
}

/// Extract the PSL-based registrable domain (eTLD+1) for rollup grouping.
/// Falls back to `extract_domain()` when PSL lookup fails (IPs, localhost).
pub fn extract_site_group(url_str: &str) -> String {
    let domain = extract_domain(url_str);
    if domain.is_empty() {
        return domain;
    }
    match addr::parse_domain_name(&domain) {
        Ok(parsed) => parsed
            .root()
            .map(|r| r.to_string())
            .unwrap_or_else(|| domain.clone()),
        Err(_) => domain,
    }
}
