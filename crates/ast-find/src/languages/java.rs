/// Java language adapter.
use crate::adapter::LangAdapter;
use crate::dsl::{Expr, Kind};
use anyhow::Result;

pub struct JavaAdapter;

impl LangAdapter for JavaAdapter {
    fn name(&self) -> &'static str {
        "java"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_java::language()
    }

    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>> {
        match expr {
            Expr::Node { kind, .. } => {
                let query_str = match kind {
                    Kind::Call => {
                        r#"
                        (method_invocation
                          object: (_) @obj
                          name: (identifier) @prop
                        ) @call

                        (method_invocation
                          name: (identifier) @callee_id
                        ) @call
                        (#not-match? @call "\.")
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (import_declaration
                          [(scoped_identifier) (identifier)] @module
                          (asterisk)?
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (class_declaration
                          name: (identifier) @name
                        ) @def

                        (interface_declaration
                          name: (identifier) @name
                        ) @def

                        (enum_declaration
                          name: (identifier) @name
                        ) @def

                        (record_declaration
                          name: (identifier) @name
                        ) @def

                        (annotation_type_declaration
                          name: (identifier) @name
                        ) @def

                        (method_declaration
                          name: (identifier) @name
                        ) @def

                        (constructor_declaration
                          name: (identifier) @name
                        ) @def

                        (compact_constructor_declaration
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
