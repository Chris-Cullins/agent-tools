/// File processing logic for ast-find.

use crate::adapter::{CaptureBundle, LangAdapter};
use crate::dsl::{Expr, Pred};
use agent_tools_common::{is_probably_binary, make_chunk_id, slice_with_context, Event, LineIndex};
use anyhow::Result;
use std::path::Path;
use tree_sitter::{Node, QueryCursor};

/// Process a single file with the given adapter and expression.
pub fn process_file(
    adapter: &dyn LangAdapter,
    path: &Path,
    expr: &Expr,
    context_lines: u32,
) -> Result<Vec<Event>> {
    let src = std::fs::read(path)?;

    // Skip binary files
    if is_probably_binary(&src) {
        return Ok(vec![]);
    }

    // Parse the file
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&adapter.language())?;

    let tree = parser
        .parse(&src, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;

    // Compile the query
    let queries = adapter.compile(expr)?;
    let line_index = LineIndex::new(&src);
    let mut events = Vec::new();

    for query in &queries {
        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(query, tree.root_node(), src.as_slice());

        for m in matches {
            // Find the main node (usually the first capture)
            let node = m
                .captures
                .first()
                .map(|c| c.node)
                .unwrap_or(tree.root_node());

            // Build capture bundle for filtering
            let mut bundle = CaptureBundle::new();
            for capture in m.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                let text = node_text(&src, &capture.node);
                bundle.insert(capture_name.to_string(), text);
            }

            // Apply post-capture filter
            if !adapter.post_capture_filter(&bundle) {
                continue;
            }

            // Apply predicate filters from DSL
            if let Expr::Node { preds, .. } = expr {
                if !apply_predicates(preds, &bundle) {
                    continue;
                }
            }

            // Extract position information
            let start_line = node.start_position().row as u32 + 1;
            let end_line = node.end_position().row as u32 + 1;
            let chunk_id = make_chunk_id(path, start_line, end_line);

            // Extract excerpt with context
            let excerpt = slice_with_context(&src, &line_index, start_line, end_line, context_lines);

            // Build capture JSON
            let capture_json = serde_json::json!({
                "callee": bundle.get("callee_id").or_else(|| bundle.get("prop")),
                "object": bundle.get("obj"),
                "attr": bundle.get("attr"),
                "module": bundle.get("module"),
                "name": bundle.get("name"),
            });

            events.push(Event::Match {
                lang: Some(adapter.name().to_string()),
                path: path.to_string_lossy().to_string(),
                start_line,
                end_line,
                chunk_id,
                score: 1.0,
                excerpt,
                capture: capture_json,
            });
        }
    }

    Ok(events)
}

/// Apply DSL predicates to a capture bundle.
fn apply_predicates(preds: &[Pred], bundle: &CaptureBundle) -> bool {
    for pred in preds {
        let matched = match pred {
            Pred::Callee(re) => {
                bundle.get("callee_id").map(|t| re.is_match(t)).unwrap_or(false)
                    || bundle.get("prop").map(|t| re.is_match(t)).unwrap_or(false)
            }
            Pred::Name(re) => bundle.get("name").map(|t| re.is_match(t)).unwrap_or(false),
            Pred::Module(re) => bundle.get("module").map(|t| re.is_match(t)).unwrap_or(false),
            Pred::Prop(re) => bundle.get("prop").map(|t| re.is_match(t)).unwrap_or(false),
            Pred::Arg(_) => {
                // TODO: Implement argument matching
                true
            }
        };

        if !matched {
            return false;
        }
    }
    true
}

/// Extract text from a node.
fn node_text(src: &[u8], node: &Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    String::from_utf8_lossy(&src[start..end]).to_string()
}
