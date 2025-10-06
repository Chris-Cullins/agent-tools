/// Language adapter trait for translating DSL queries into Tree-sitter queries.

use crate::dsl::Expr;
use anyhow::Result;
use std::collections::HashMap;

/// Trait for language-specific adapters.
pub trait LangAdapter: Send + Sync {
    /// Language identifier (e.g., "python", "javascript").
    fn name(&self) -> &'static str;

    /// Tree-sitter language object.
    fn language(&self) -> tree_sitter::Language;

    /// Compile the DSL expression into Tree-sitter query strings.
    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>>;

    /// Optional post-processing filter on captures.
    /// Return true if the capture should be included in results.
    fn post_capture_filter(&self, _caps: &CaptureBundle) -> bool {
        true
    }
}

/// Bundle of capture texts from a Tree-sitter match.
#[derive(Debug)]
pub struct CaptureBundle {
    pub texts: HashMap<String, String>,
}

impl CaptureBundle {
    pub fn new() -> Self {
        Self {
            texts: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.texts.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.texts.get(key).map(|s| s.as_str())
    }
}

impl Default for CaptureBundle {
    fn default() -> Self {
        Self::new()
    }
}
