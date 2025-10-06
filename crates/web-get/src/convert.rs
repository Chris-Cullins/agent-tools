/// HTML to Markdown conversion with sanitization.

use ammonia::Builder;
use anyhow::Result;
use scraper::{Html, Selector};
use std::collections::HashSet;

pub struct ConvertOptions {
    pub keep_images: bool,
    pub base_url: String,
}

pub struct ConvertedContent {
    pub markdown: String,
    pub links: Vec<String>,
    pub hash: String,
}

pub fn convert_to_markdown(html: &str, opts: &ConvertOptions) -> Result<ConvertedContent> {
    // Sanitize HTML
    let sanitized = sanitize_html(html, &opts.base_url, opts.keep_images);

    // Extract links
    let links = extract_links(&sanitized, &opts.base_url);

    // Convert to Markdown
    let markdown = html2md::parse_html(&sanitized);

    // Hash the markdown
    let hash = blake3::hash(markdown.as_bytes()).to_hex().to_string();

    Ok(ConvertedContent {
        markdown,
        links,
        hash,
    })
}

fn sanitize_html(html: &str, base_url: &str, keep_images: bool) -> String {
    let mut builder = Builder::default();

    // Configure allowed tags
    let mut tags = maplit::hashset! {
        "p", "br", "h1", "h2", "h3", "h4", "h5", "h6",
        "strong", "em", "u", "s", "code", "pre",
        "ul", "ol", "li", "blockquote",
        "a", "table", "thead", "tbody", "tr", "th", "td",
    };

    if keep_images {
        tags.insert("img");
    }

    builder.tags(tags);

    // Configure allowed attributes
    builder.link_rel(None);
    if let Ok(base) = url::Url::parse(base_url) {
        builder.url_relative(ammonia::UrlRelative::RewriteWithBase(base));
    }

    builder.clean(html).to_string()
}

fn extract_links(html: &str, base_url: &str) -> Vec<String> {
    let document = Html::parse_document(html);
    let mut links = HashSet::new();

    if let Ok(selector) = Selector::parse("a[href]") {
        for el in document.select(&selector) {
            if let Some(href) = el.value().attr("href") {
                // Make absolute
                let absolute = if href.starts_with("http") {
                    href.to_string()
                } else if let Ok(base) = url::Url::parse(base_url) {
                    if let Ok(resolved) = base.join(href) {
                        resolved.to_string()
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };

                // Filter out non-http(s) links
                if absolute.starts_with("http://") || absolute.starts_with("https://") {
                    links.insert(absolute);
                }
            }
        }
    }

    let mut links: Vec<_> = links.into_iter().collect();
    links.sort();
    links
}

/// Decode bytes to UTF-8 with charset detection.
pub fn decode_to_utf8(
    bytes: &[u8],
    header_charset: Option<&str>,
) -> Result<String> {
    // Try header charset first
    if let Some(cs) = header_charset {
        if let Some(enc) = encoding_rs::Encoding::for_label(cs.trim().as_bytes()) {
            let (cow, _, _) = enc.decode(bytes);
            return Ok(cow.into_owned());
        }
    }

    // Try charset detection
    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    let encoding = detector.guess(None, true);
    let (cow, _, _) = encoding.decode(bytes);
    Ok(cow.into_owned())
}

/// Parse charset from Content-Type header.
pub fn parse_charset(content_type: &str) -> Option<String> {
    if let Ok(mime) = content_type.parse::<mime::Mime>() {
        mime.get_param(mime::CHARSET)
            .map(|cs| cs.as_str().to_string())
    } else {
        None
    }
}
