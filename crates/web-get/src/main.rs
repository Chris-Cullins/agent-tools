mod convert;
mod extract;
mod fetch;

use anyhow::Result;
use clap::Parser;
use agent_tools_common::{Event, write_ndjson};
use futures::stream::{FuturesUnordered, StreamExt};
use std::io::{self, BufRead};

#[derive(Parser, Debug)]
#[command(name = "web-get", about = "Fetch & sanitize web pages into Markdown")]
struct Opt {
    /// URLs to fetch. If omitted, reads from stdin (one per line).
    urls: Vec<String>,
    /// CSS selector to pick main content (comma-separated OK)
    #[arg(long)]
    selector: Option<String>,
    /// Max bytes to read per response (e.g., 10MB)
    #[arg(long, default_value = "10MB")]
    max_bytes: String,
    /// Request timeout (e.g., 15s)
    #[arg(long, default_value = "15s")]
    timeout: String,
    /// Keep <img> tags when converting to Markdown
    #[arg(long, default_value_t = false)]
    keep_images: bool,
    /// Concurrency for multiple URLs
    #[arg(long, default_value_t = 6)]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("NO_COLOR", "1");
    std::env::set_var("TZ", "UTC");

    let opt = Opt::parse();

    // Parse options
    let timeout = fetch::parse_duration(&opt.timeout)?;
    let max_bytes = fetch::parse_size(&opt.max_bytes)?;

    // Collect URLs (args or stdin)
    let mut urls = opt.urls.clone();
    if urls.is_empty() {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let u = line?;
            if !u.trim().is_empty() {
                urls.push(u.trim().to_string());
            }
        }
    }

    // Create HTTP client
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    // Process URLs with bounded concurrency
    let mut tasks = FuturesUnordered::new();
    for url in urls {
        let client = client.clone();
        let selector = opt.selector.clone();
        let keep_images = opt.keep_images;

        tasks.push(tokio::spawn(async move {
            process_url(&client, &url, &selector, keep_images, timeout, max_bytes).await
        }));

        // Limit concurrency
        while tasks.len() >= opt.concurrency {
            if let Some(result) = tasks.next().await {
                let event = result??;
                write_ndjson(&event)?;
            }
        }
    }

    // Drain remaining tasks
    while let Some(result) = tasks.next().await {
        let event = result??;
        write_ndjson(&event)?;
    }

    Ok(())
}

async fn process_url(
    client: &reqwest::Client,
    url: &str,
    selector: &Option<String>,
    keep_images: bool,
    timeout: std::time::Duration,
    max_bytes: usize,
) -> Result<Event> {
    match process_url_inner(client, url, selector, keep_images, timeout, max_bytes).await {
        Ok(event) => Ok(event),
        Err(e) => Ok(Event::Error {
            code: "E_FETCH".to_string(),
            message: format!("{:#}", e),
            path_or_url: Some(url.to_string()),
        }),
    }
}

async fn process_url_inner(
    client: &reqwest::Client,
    url: &str,
    selector: &Option<String>,
    keep_images: bool,
    timeout: std::time::Duration,
    max_bytes: usize,
) -> Result<Event> {
    // Fetch URL
    let fetch_opts = fetch::FetchOptions {
        timeout,
        max_bytes,
        user_agent: "web-get/0.1".to_string(),
    };

    let fetch_result = fetch::fetch_url(client, url, &fetch_opts).await?;

    // Parse media type
    let media_type = if let Ok(mime) = fetch_result.content_type.parse::<mime::Mime>() {
        mime.essence_str().to_string()
    } else {
        "application/octet-stream".to_string()
    };

    // Handle PDFs as stubs
    if media_type.starts_with("application/pdf") {
        let hash = blake3::hash(&fetch_result.body).to_hex().to_string();
        return Ok(Event::Document {
            url: fetch_result.final_url,
            title: String::new(),
            byline: None,
            text_md: String::new(),
            word_count: 0,
            links: vec![],
            canonical_url: None,
            media_type,
            hash,
        });
    }

    // Decode HTML
    let charset = convert::parse_charset(&fetch_result.content_type);
    let html = convert::decode_to_utf8(&fetch_result.body, charset.as_deref())?;

    // Extract content
    let extract_opts = extract::ExtractOptions {
        selector: selector.clone(),
    };
    let extracted = extract::extract_content(&html, &fetch_result.final_url, &extract_opts)?;

    // Convert to Markdown
    let convert_opts = convert::ConvertOptions {
        keep_images,
        base_url: fetch_result.final_url.clone(),
    };
    let converted = convert::convert_to_markdown(&extracted.main_html, &convert_opts)?;

    let word_count = converted.markdown.split_whitespace().count() as u32;

    Ok(Event::Document {
        url: fetch_result.final_url,
        title: extracted.title,
        byline: extracted.byline,
        text_md: converted.markdown,
        word_count,
        links: converted.links,
        canonical_url: extracted.canonical_url,
        media_type: "text/html".to_string(),
        hash: converted.hash,
    })
}
