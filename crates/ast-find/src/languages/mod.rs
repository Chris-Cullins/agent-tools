/// Language adapters for different programming languages.
pub mod csharp;
pub mod javascript;
pub mod python;

use crate::adapter::LangAdapter;
use phf::phf_map;
use std::sync::Arc;

pub use csharp::CSharpAdapter;
pub use javascript::{JavaScriptAdapter, TypeScriptAdapter};
pub use python::PythonAdapter;

/// Language ID enum for static dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LangId {
    JavaScript,
    TypeScript,
    Python,
    CSharp,
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
};

/// Get a language adapter by ID.
pub fn get_adapter(lang_id: LangId) -> Arc<dyn LangAdapter> {
    match lang_id {
        LangId::JavaScript => Arc::new(JavaScriptAdapter),
        LangId::TypeScript => Arc::new(TypeScriptAdapter),
        LangId::Python => Arc::new(PythonAdapter),
        LangId::CSharp => Arc::new(CSharpAdapter),
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
                _ => None,
            }
        })
        .collect()
}
