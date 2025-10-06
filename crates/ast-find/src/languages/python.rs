/// Python language adapter.
use crate::adapter::LangAdapter;
use crate::dsl::{Expr, Kind};
use anyhow::Result;

pub struct PythonAdapter;

impl LangAdapter for PythonAdapter {
    fn name(&self) -> &'static str {
        "python"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_python::language()
    }

    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>> {
        match expr {
            Expr::Node { kind, .. } => {
                let query_str = match kind {
                    Kind::Call => {
                        r#"
                        (call
                          function: (identifier) @callee_id
                        ) @call

                        (call
                          function: (attribute
                            object: (_) @obj
                            attribute: (identifier) @attr
                          )
                        ) @call
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (import_statement
                          name: (dotted_name) @module
                        ) @import

                        (import_from_statement
                          module_name: (dotted_name) @module
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (function_definition
                          name: (identifier) @name
                        ) @def

                        (class_definition
                          name: (identifier) @name
                        ) @def
                        "#
                    }
                };

                let lang = self.language();
                let query = tree_sitter::Query::new(&lang, query_str)?;
                Ok(vec![query])
            }
        }
    }
}
