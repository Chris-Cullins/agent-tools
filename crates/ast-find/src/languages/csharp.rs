/// C# language adapter.
use crate::adapter::LangAdapter;
use crate::dsl::{Expr, Kind};
use anyhow::Result;

pub struct CSharpAdapter;

impl LangAdapter for CSharpAdapter {
    fn name(&self) -> &'static str {
        "csharp"
    }

    fn language(&self) -> tree_sitter::Language {
        tree_sitter_c_sharp::language()
    }

    fn compile(&self, expr: &Expr) -> Result<Vec<tree_sitter::Query>> {
        match expr {
            Expr::Node { kind, .. } => {
                let query_str = match kind {
                    Kind::Call => {
                        r#"
                        (invocation_expression
                          function: (identifier) @callee_id
                        ) @call

                        (invocation_expression
                          function: (generic_name) @callee_id
                        ) @call

                        (invocation_expression
                          function: (member_access_expression
                            expression: (_) @obj
                            name: (identifier) @prop
                          )
                        ) @call

                        (invocation_expression
                          function: (member_access_expression
                            expression: (_) @obj
                            name: (generic_name) @prop
                          )
                        ) @call

                        (invocation_expression
                          function: (conditional_access_expression
                            condition: (_) @obj
                            (member_binding_expression
                              name: (identifier) @prop
                            )
                          )
                        ) @call

                        (invocation_expression
                          function: (conditional_access_expression
                            condition: (_) @obj
                            (member_binding_expression
                              name: (generic_name) @prop
                            )
                          )
                        ) @call
                        "#
                    }
                    Kind::Import => {
                        r#"
                        (using_directive
                          (_name) @module
                        ) @import

                        (using_directive
                          (type) @module
                        ) @import
                        "#
                    }
                    Kind::Def => {
                        r#"
                        (class_declaration
                          name: (identifier) @name
                        ) @def

                        (struct_declaration
                          name: (identifier) @name
                        ) @def

                        (interface_declaration
                          name: (identifier) @name
                        ) @def

                        (record_declaration
                          name: (identifier) @name
                        ) @def

                        (enum_declaration
                          name: (identifier) @name
                        ) @def

                        (method_declaration
                          name: (identifier) @name
                        ) @def

                        (constructor_declaration
                          name: (identifier) @name
                        ) @def

                        (local_function_statement
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
