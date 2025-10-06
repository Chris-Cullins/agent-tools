use anyhow::Result;
/// HTML content extraction (Readability-lite heuristic).
use scraper::{Html, Selector};

pub struct ExtractOptions {
    pub selector: Option<String>,
}

pub struct ExtractedContent {
    pub title: String,
    pub byline: Option<String>,
    pub canonical_url: Option<String>,
    pub main_html: String,
}

pub fn extract_content(
    html: &str,
    base_url: &str,
    opts: &ExtractOptions,
) -> Result<ExtractedContent> {
    let document = Html::parse_document(html);

    // Extract metadata
    let title = extract_title(&document);
    let byline = extract_byline(&document);
    let canonical_url = extract_canonical(&document, base_url);

    // Extract main content
    let main_html = if let Some(ref selector_str) = opts.selector {
        extract_by_selector(&document, selector_str)?
    } else {
        extract_by_heuristic(&document)?
    };

    Ok(ExtractedContent {
        title,
        byline,
        canonical_url,
        main_html,
    })
}

fn extract_title(document: &Html) -> String {
    // Try <title> tag first
    if let Ok(selector) = Selector::parse("title") {
        if let Some(title_el) = document.select(&selector).next() {
            let text = title_el.text().collect::<String>();
            if !text.trim().is_empty() {
                return text.trim().to_string();
            }
        }
    }

    // Try meta og:title
    if let Ok(selector) = Selector::parse(r#"meta[property="og:title"]"#) {
        if let Some(meta_el) = document.select(&selector).next() {
            if let Some(content) = meta_el.value().attr("content") {
                if !content.trim().is_empty() {
                    return content.trim().to_string();
                }
            }
        }
    }

    String::new()
}

fn extract_byline(document: &Html) -> Option<String> {
    // Try meta author
    if let Ok(selector) = Selector::parse(r#"meta[name="author"]"#) {
        if let Some(meta_el) = document.select(&selector).next() {
            if let Some(content) = meta_el.value().attr("content") {
                if !content.trim().is_empty() {
                    return Some(content.trim().to_string());
                }
            }
        }
    }

    // Try class="author" or similar
    if let Ok(selector) = Selector::parse(r#"[class*="author"], [class*="byline"]"#) {
        if let Some(author_el) = document.select(&selector).next() {
            let text = author_el.text().collect::<String>();
            if !text.trim().is_empty() {
                return Some(text.trim().to_string());
            }
        }
    }

    None
}

fn extract_canonical(document: &Html, base_url: &str) -> Option<String> {
    if let Ok(selector) = Selector::parse(r#"link[rel="canonical"]"#) {
        if let Some(link_el) = document.select(&selector).next() {
            if let Some(href) = link_el.value().attr("href") {
                // Make absolute
                if href.starts_with("http") {
                    return Some(href.to_string());
                } else if let Ok(base) = url::Url::parse(base_url) {
                    if let Ok(resolved) = base.join(href) {
                        return Some(resolved.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_by_selector(document: &Html, selector_str: &str) -> Result<String> {
    let selector =
        Selector::parse(selector_str).map_err(|e| anyhow::anyhow!("Invalid selector: {:?}", e))?;

    // Find the first matching element with the most text
    let mut best_el = None;
    let mut best_len = 0;

    for el in document.select(&selector) {
        let text = el.text().collect::<String>();
        let len = text.trim().len();
        if len > best_len {
            best_len = len;
            best_el = Some(el);
        }
    }

    if let Some(el) = best_el {
        Ok(el.html())
    } else {
        anyhow::bail!("No matching elements for selector: {}", selector_str)
    }
}

fn extract_by_heuristic(document: &Html) -> Result<String> {
    // Simple heuristic: look for common content containers
    let candidate_selectors = vec![
        "article",
        "main",
        "[role='main']",
        ".main-content",
        ".article-content",
        ".post-content",
        "#content",
        "#main",
    ];

    for selector_str in candidate_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(el) = document.select(&selector).next() {
                let text = el.text().collect::<String>();
                if text.trim().len() > 200 {
                    return Ok(el.html());
                }
            }
        }
    }

    // Fallback: return body
    if let Ok(selector) = Selector::parse("body") {
        if let Some(body) = document.select(&selector).next() {
            return Ok(body.html());
        }
    }

    anyhow::bail!("Could not extract main content")
}
