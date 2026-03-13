// HTML extraction utilities

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
    converter.convert(html).unwrap_or_default()
}

/// Convert Markdown to plain text, stripping all formatting.
pub fn markdown_to_plaintext(md: &str) -> String {
    let text = markdown_to_text::convert(md);
    // Clean up residual table pipe characters
    let text = text.replace(" | ", " ").replace("| ", "").replace(" |", "");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Convert HTML to plain text for search indexing.
pub fn html_to_plaintext(html: &str) -> String {
    let md = html_to_markdown(html);
    markdown_to_plaintext(&md)
}

/// Extract the <title> from an HTML document using simple string parsing.
pub fn extract_title(html: &str) -> String {
    extract_title_inner(html).unwrap_or_default()
}

fn extract_title_inner(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start = lower.find("<title")?;
    let tag_end = lower[start..].find('>')? + start + 1;
    let end = lower[tag_end..].find("</title")? + tag_end;
    let title = html[tag_end..end].trim();
    Some(title.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Extract the domain from a URL.
pub fn extract_domain(url_str: &str) -> String {
    url::Url::parse(url_str)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .unwrap_or_default()
}
