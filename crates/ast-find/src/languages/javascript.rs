/// JavaScript/TypeScript language adapter.
use crate::adapter::LangAdapter;
use crate::dsl::{Expr, Kind};
use anyhow::Result;

pub struct JavaScriptAdapter;

impl LangAdapter for JavaScriptAdapter {
    fn name(&self) -> &'static str {
        "javascript"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::language()
    }

    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>> {
        match expr {
            Expr::Node { kind, .. } => {
                let query_str = match kind {
                    Kind::Call => {
                        // Match both identifier calls (foo(...)) and member calls (obj.prop(...))
                        r#"
                        (call_expression
                          function: (identifier) @callee_id
                        ) @call

                        (call_expression
                          function: (member_expression
                            object: (_) @obj
                            property: (property_identifier) @prop
                          )
                        ) @call
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (import_statement
                          source: (string) @module
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (function_declaration
                          name: (identifier) @name
                        ) @def

                        (lexical_declaration
                          (variable_declarator
                            name: (identifier) @name
                            value: [(arrow_function) (function_expression)]
                          )
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

pub struct TypeScriptAdapter;

impl LangAdapter for TypeScriptAdapter {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::language_typescript()
    }

    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>> {
        // For now, use same queries as JavaScript
        match expr {
            Expr::Node { kind, .. } => {
                let query_str = match kind {
                    Kind::Call => {
                        r#"
                        (call_expression
                          function: (identifier) @callee_id
                        ) @call

                        (call_expression
                          function: (member_expression
                            object: (_) @obj
                            property: (property_identifier) @prop
                          )
                        ) @call
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (import_statement
                          source: (string) @module
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (function_declaration
                          name: (identifier) @name
                        ) @def

                        (lexical_declaration
                          (variable_declarator
                            name: (identifier) @name
                            value: [(arrow_function) (function_expression)]
                          )
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
