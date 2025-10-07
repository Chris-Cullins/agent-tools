/// Language adapters for different programming languages.
pub mod csharp;
pub mod go;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rust;

use crate::adapter::LangAdapter;
use phf::phf_map;
use std::sync::Arc;

pub use csharp::CSharpAdapter;
pub use go::GoAdapter;
pub use java::JavaAdapter;
pub use javascript::{JavaScriptAdapter, TypeScriptAdapter};
pub use python::PythonAdapter;
pub use rust::RustAdapter;

/// Language ID enum for static dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LangId {
    JavaScript,
    TypeScript,
    Python,
    CSharp,
    Rust,
    Go,
    Java,
}

/// Map file extensions to language IDs.
pub static LANG_BY_EXT: phf::Map<&'static str, LangId> = phf_map! {
    "js" => LangId::JavaScript,
    "jsx" => LangId::JavaScript,
    "ts" => LangId::TypeScript,
    "tsx" => LangId::TypeScript,
    "py" => LangId::Python,
    "cs" => LangId::CSharp,
    "csx" => LangId::CSharp,
    "rs" => LangId::Rust,
    "go" => LangId::Go,
    "java" => LangId::Java,
};

/// Get a language adapter by ID.
pub fn get_adapter(lang_id: LangId) -> Arc<dyn LangAdapter> {
    match lang_id {
        LangId::JavaScript => Arc::new(JavaScriptAdapter),
        LangId::TypeScript => Arc::new(TypeScriptAdapter),
        LangId::Python => Arc::new(PythonAdapter),
        LangId::CSharp => Arc::new(CSharpAdapter),
        LangId::Rust => Arc::new(RustAdapter),
        LangId::Go => Arc::new(GoAdapter),
        LangId::Java => Arc::new(JavaAdapter),
    }
}

/// Parse a comma-separated language list (e.g., "py,ts,js").
pub fn parse_lang_list(s: &str) -> Vec<LangId> {
    s.split(',')
        .filter_map(|part| {
            let part = part.trim();
            match part {
                "py" | "python" => Some(LangId::Python),
                "js" | "javascript" => Some(LangId::JavaScript),
                "ts" | "typescript" => Some(LangId::TypeScript),
                "cs" | "csharp" | "c#" => Some(LangId::CSharp),
                "rs" | "rust" => Some(LangId::Rust),
                "go" | "golang" => Some(LangId::Go),
                "java" => Some(LangId::Java),
                _ => None,
            }
        })
        .collect()
}
