/// Go language adapter.
use crate::adapter::LangAdapter;
use crate::dsl::{Expr, Kind};
use anyhow::Result;

pub struct GoAdapter;

impl LangAdapter for GoAdapter {
    fn name(&self) -> &'static str {
        "go"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_go::language()
    }

    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>> {
        match expr {
            Expr::Node { kind, .. } => {
                let query_str = match kind {
                    Kind::Call => {
                        r#"
                        (call_expression
                          function: (identifier) @callee_id
                        ) @call

                        (call_expression
                          function: (selector_expression
                            operand: (_) @obj
                            field: (field_identifier) @prop
                          )
                        ) @call
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (import_spec
                          path: [(interpreted_string_literal) (raw_string_literal)] @module
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (function_declaration
                          name: (identifier) @name
                        ) @def

                        (method_declaration
                          name: (field_identifier) @name
                        ) @def

                        (type_spec
                          name: (type_identifier) @name
                        ) @def

                        (type_alias
                          name: (type_identifier) @name
                        ) @def

                        (const_spec
                          name: (identifier) @name
                        ) @def

                        (var_spec
                          name: (identifier) @name
                        ) @def
                        "#
                    }
                };

                let lang = self.language();
                let query = tree_sitter::Query::new(&lang, query_str)?;
                Ok(vec![query])
            }
            _ => anyhow::bail!("Composite expressions are handled in the processor"),
        }
    }
}
