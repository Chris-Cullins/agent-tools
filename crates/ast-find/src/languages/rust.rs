/// Rust language adapter.
use crate::adapter::LangAdapter;
use crate::dsl::{Expr, Kind};
use anyhow::Result;

pub struct RustAdapter;

impl LangAdapter for RustAdapter {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_rust::language()
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
                          function: (scoped_identifier
                            path: (_) @obj
                            name: (identifier) @callee_id
                          )
                        ) @call

                        (call_expression
                          function: (field_expression
                            value: (_) @obj
                            field: (_) @prop
                          )
                        ) @call

                        (call_expression
                          function: (generic_function
                            function: (identifier) @callee_id
                          )
                        ) @call

                        (call_expression
                          function: (generic_function
                            function: (scoped_identifier
                              path: (_) @obj
                              name: (identifier) @callee_id
                            )
                          )
                        ) @call

                        (call_expression
                          function: (generic_function
                            function: (field_expression
                              value: (_) @obj
                              field: (_) @prop
                            )
                          )
                        ) @call
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (use_declaration
                          argument: (_) @module
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (function_item
                          name: (_) @name
                        ) @def

                        (function_signature_item
                          name: (_) @name
                        ) @def

                        (struct_item
                          name: (_) @name
                        ) @def

                        (enum_item
                          name: (_) @name
                        ) @def

                        (union_item
                          name: (_) @name
                        ) @def

                        (trait_item
                          name: (_) @name
                        ) @def

                        (type_item
                          name: (_) @name
                        ) @def

                        (mod_item
                          name: (identifier) @name
                        ) @def

                        (const_item
                          name: (identifier) @name
                        ) @def

                        (static_item
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
