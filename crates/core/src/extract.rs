//! HTML extraction utilities: HTML → Markdown → plaintext pipeline,
//! title extraction, URL extraction, and domain extraction.

use htmd::HtmlToMarkdown;

/// Convert HTML to Markdown, preserving structure but stripping bold/italic.
pub fn html_to_markdown(html: &str) -> String {
    let converter = HtmlToMarkdown::builder()
        .skip_tags(vec!["script", "style", "nav", "footer", "header"])
        .add_handler(vec!["b", "strong", "i", "em"], |handlers: &dyn htmd::element_handler::Handlers, element: htmd::Element| {
            let content = handlers.walk_children(element.node).content;
            if content.is_empty() {
                None
            } else {
                Some(content.into())
            }
        })
        .build();
    match converter.convert(html) {
        Ok(md) => md,
        Err(e) => {
            tracing::warn!("HTML to markdown conversion failed: {e}");
            String::new()
        }
    }
}

/// Convert Markdown to plain text, stripping all formatting.
pub fn markdown_to_plaintext(md: &str) -> String {
    let text = markdown_to_text::convert(md);
    // Clean up residual table pipe characters
    let text = text.replace(" | ", " ").replace("| ", "").replace(" |", "");
    // Strip any residual HTML tags that htmd passed through unconverted
    let text = strip_html_tags(&text);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove HTML tags from text, preserving the text content between them.
fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

/// Convert HTML to plain text for search indexing.
pub fn html_to_plaintext(html: &str) -> String {
    let md = html_to_markdown(html);
    markdown_to_plaintext(&md)
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
