/// HTTP fetching with size and timeout limits.
use anyhow::Result;
use bytes::Bytes;
use reqwest::Client;
use std::time::Duration;

pub struct FetchOptions {
    pub timeout: Duration,
    pub max_bytes: usize,
    pub user_agent: String,
}

impl Default for FetchOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
            max_bytes: 10 * 1024 * 1024, // 10MB
            user_agent: "web-get/0.1".to_string(),
        }
    }
}

pub struct FetchResult {
    pub url: String,
    pub final_url: String,
    pub content_type: String,
    pub body: Bytes,
    pub truncated: bool,
}

pub async fn fetch_url(client: &Client, url: &str, opts: &FetchOptions) -> Result<FetchResult> {
    let resp = client
        .get(url)
        .header(reqwest::header::USER_AGENT, &opts.user_agent)
        .timeout(opts.timeout)
        .send()
        .await?
        .error_for_status()?;

    let final_url = resp.url().to_string();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // Read body with size limit
    let body = resp.bytes().await?;
    let truncated = body.len() > opts.max_bytes;
    let body = if truncated {
        body.slice(..opts.max_bytes)
    } else {
        body
    };

    Ok(FetchResult {
        url: url.to_string(),
        final_url,
        content_type,
        body,
        truncated,
    })
}

/// Parse a human-readable size string (e.g., "10MB", "1GB").
pub fn parse_size(s: &str) -> Result<usize> {
    let s = s.trim().to_uppercase();
    let (num_part, unit) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        (&s[..pos], &s[pos..])
    } else {
        (s.as_str(), "")
    };

    let num: usize = num_part.parse()?;
    let multiplier = match unit {
        "B" | "" => 1,
        "KB" => 1024,
        "MB" => 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        _ => anyhow::bail!("Unknown size unit: {}", unit),
    };

    Ok(num * multiplier)
}

/// Parse a human-readable duration string (e.g., "15s", "1m").
pub fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    let (num_part, unit) = if let Some(pos) = s.find(|c: char| c.is_alphabetic()) {
        (&s[..pos], &s[pos..])
    } else {
        (s.as_str(), "s")
    };

    let num: u64 = num_part.parse()?;
    let duration = match unit {
        "s" | "sec" => Duration::from_secs(num),
        "m" | "min" => Duration::from_secs(num * 60),
        "h" | "hour" => Duration::from_secs(num * 3600),
        _ => anyhow::bail!("Unknown duration unit: {}", unit),
    };

    Ok(duration)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1KB").unwrap(), 1024);
        assert_eq!(parse_size("10MB").unwrap(), 10 * 1024 * 1024);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("15s").unwrap(), Duration::from_secs(15));
        assert_eq!(parse_duration("1m").unwrap(), Duration::from_secs(60));
    }
}
