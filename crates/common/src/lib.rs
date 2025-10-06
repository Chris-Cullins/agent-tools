use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// NDJSON event skeleton shared across tools.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum Event {
    #[serde(rename = "match")]
    Match {
        lang: Option<String>,
        path: String,
        start_line: u32,
        end_line: u32,
        chunk_id: String,
        score: f32,
        excerpt: Option<String>,
        capture: serde_json::Value,
    },
    #[serde(rename = "document")]
    Document {
        url: String,
        title: String,
        byline: Option<String>,
        text_md: String,
        word_count: u32,
        links: Vec<String>,
        canonical_url: Option<String>,
        media_type: String,
        hash: String,
    },
    #[serde(rename = "error")]
    Error {
        code: String,
        message: String,
        path_or_url: Option<String>,
    },
    #[serde(rename = "summary")]
    Summary { tool: String, message: String },
}

/// Write a single JSON object as a line (NDJSON). Flushes immediately.
pub fn write_ndjson<T: Serialize>(value: &T) -> Result<()> {
    use std::io::{self, Write};
    serde_json::to_writer(io::stdout(), value)?;
    io::stdout().write_all(b"\n")?;
    io::stdout().flush()?;
    Ok(())
}

/// Deterministic chunk id from path + line range.
pub fn make_chunk_id(path: &Path, s: u32, e: u32) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(path.to_string_lossy().as_bytes());
    hasher.update(b":");
    hasher.update(s.to_string().as_bytes());
    hasher.update(b"-");
    hasher.update(e.to_string().as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Heuristic binary detector (small sample, NUL byte check).
pub fn is_probably_binary(buf: &[u8]) -> bool {
    const N: usize = 1024;
    buf.iter().take(buf.len().min(N)).any(|&b| b == 0)
}

/// Line index for fast line number to byte offset mapping.
/// Stores the byte offset for the start of each line.
pub struct LineIndex {
    /// Byte offsets for the start of each line (0-indexed)
    starts: Vec<usize>,
}

impl LineIndex {
    /// Build a line index by scanning the source for newlines.
    pub fn new(src: &[u8]) -> Self {
        let mut starts = vec![0];
        for (i, &b) in src.iter().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        Self { starts }
    }

    /// Convert 1-based line numbers to byte range.
    /// Returns (start_byte, end_byte) or None if lines are out of bounds.
    pub fn line_range_to_bytes(&self, start_line: u32, end_line: u32) -> Option<(usize, usize)> {
        let start_idx = (start_line.saturating_sub(1)) as usize;
        let end_idx = end_line as usize;

        if start_idx >= self.starts.len() {
            return None;
        }

        let start_byte = self.starts[start_idx];
        let end_byte = if end_idx < self.starts.len() {
            self.starts[end_idx]
        } else {
            // Last line goes to end of file
            return Some((start_byte, usize::MAX));
        };

        Some((start_byte, end_byte))
    }

    /// Get the total number of lines.
    pub fn line_count(&self) -> usize {
        self.starts.len()
    }
}

/// Extract a slice of source with context lines around the target range.
/// Returns a String with the excerpt (1-based line numbers).
pub fn slice_with_context(
    src: &[u8],
    line_index: &LineIndex,
    start_line: u32,
    end_line: u32,
    context: u32,
) -> Option<String> {
    let start_with_ctx = start_line.saturating_sub(context);
    let end_with_ctx = end_line.saturating_add(context);

    let (start_byte, end_byte) = line_index.line_range_to_bytes(start_with_ctx, end_with_ctx)?;

    let slice = if end_byte == usize::MAX {
        &src[start_byte..]
    } else {
        &src[start_byte..end_byte.min(src.len())]
    };

    String::from_utf8(slice.to_vec()).ok()
}
