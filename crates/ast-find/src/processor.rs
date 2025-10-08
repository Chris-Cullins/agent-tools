/// File processing logic for ast-find.
use crate::adapter::{CaptureBundle, LangAdapter};
use crate::dsl::{Expr, Pred};
use agent_tools_common::{is_probably_binary, make_chunk_id, slice_with_context, Event, LineIndex};
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use tree_sitter::{Node, Query, QueryCursor, Tree};

#[derive(Debug, Clone)]
pub struct MatchRecord {
    pub lang: Option<String>,
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub chunk_id: String,
    pub excerpt: Option<String>,
    pub capture: serde_json::Value,
}

type MatchMap = BTreeMap<String, MatchRecord>;

impl From<MatchRecord> for Event {
    fn from(record: MatchRecord) -> Self {
        Event::Match {
            lang: record.lang,
            path: record.path,
            start_line: record.start_line,
            end_line: record.end_line,
            chunk_id: record.chunk_id,
            score: 1.0,
            excerpt: record.excerpt,
            capture: record.capture,
        }
    }
}

struct EvalContext<'a> {
    adapter: &'a dyn LangAdapter,
    tree: Tree,
    src: Vec<u8>,
    line_index: LineIndex,
    path: &'a Path,
    context_lines: u32,
    lang_name: &'a str,
    node_cache: HashMap<*const Expr, MatchMap>,
    query_cache: HashMap<*const Expr, Vec<Query>>,
    universe: Option<MatchMap>,
}

impl<'a> EvalContext<'a> {
    fn eval_expr(&mut self, expr: &Expr) -> Result<MatchMap> {
        match expr {
            Expr::Node { .. } => self.eval_node(expr),
            Expr::And(children) => self.eval_and(children),
            Expr::Or(children) => self.eval_or(children),
            Expr::Not(child) => self.eval_not(child),
        }
    }

    fn eval_and(&mut self, children: &[Expr]) -> Result<MatchMap> {
        let mut iter = children.iter();
        let mut result = match iter.next() {
            Some(first) => self.eval_expr(first)?,
            None => MatchMap::new(),
        };

        for child in iter {
            let child_set = self.eval_expr(child)?;
            result.retain(|key, _| child_set.contains_key(key));
        }

        Ok(result)
    }

    fn eval_or(&mut self, children: &[Expr]) -> Result<MatchMap> {
        let mut result = MatchMap::new();
        for child in children {
            let child_set = self.eval_expr(child)?;
            for (key, record) in child_set {
                result.entry(key).or_insert(record);
            }
        }
        Ok(result)
    }

    fn eval_not(&mut self, child: &Expr) -> Result<MatchMap> {
        let child_set = self.eval_expr(child)?;
        let universe = self
            .universe
            .as_ref()
            .cloned()
            .unwrap_or_else(MatchMap::new);

        let mut result = universe;
        for key in child_set.keys() {
            result.remove(key);
        }
        Ok(result)
    }

    fn eval_node(&mut self, expr: &Expr) -> Result<MatchMap> {
        let key = expr as *const Expr;
        if let Some(cached) = self.node_cache.get(&key) {
            return Ok(cached.clone());
        }

        let queries = self
            .query_cache
            .entry(key)
            .or_insert(self.adapter.compile(expr)?);

        let mut map = MatchMap::new();
        for query in queries.iter() {
            let mut cursor = QueryCursor::new();
            let matches = cursor.matches(query, self.tree.root_node(), self.src.as_slice());

            for m in matches {
                let node = m
                    .captures
                    .first()
                    .map(|c| c.node)
                    .unwrap_or(self.tree.root_node());

                let mut bundle = CaptureBundle::new();
                for capture in m.captures {
                    let capture_name = query.capture_names()[capture.index as usize];
                    let text = node_text(&self.src, &capture.node);
                    bundle.insert(capture_name.to_string(), text);
                }

                // Store full node text for multi-line predicates.
                bundle.insert("__node_text", node_text(&self.src, &node));

                if !self.adapter.post_capture_filter(&bundle) {
                    continue;
                }

                let preds = match expr {
                    Expr::Node { preds, .. } => preds,
                    _ => unreachable!(),
                };

                if !apply_predicates(preds, &bundle) {
                    continue;
                }

                let start_line = node.start_position().row as u32 + 1;
                let end_line = node.end_position().row as u32 + 1;
                let chunk_id = make_chunk_id(self.path, start_line, end_line);
                let excerpt = slice_with_context(
                    &self.src,
                    &self.line_index,
                    start_line,
                    end_line,
                    self.context_lines,
                );

                let capture_json = serde_json::json!({
                    "callee": bundle.get("callee_id").or_else(|| bundle.get("prop")),
                    "object": bundle.get("obj"),
                    "attr": bundle.get("attr"),
                    "module": bundle.get("module"),
                    "name": bundle.get("name"),
                });

                let record = MatchRecord {
                    lang: Some(self.lang_name.to_string()),
                    path: self.path.to_string_lossy().to_string(),
                    start_line,
                    end_line,
                    chunk_id: chunk_id.clone(),
                    excerpt,
                    capture: capture_json,
                };

                map.entry(chunk_id).or_insert(record);
            }
        }

        self.node_cache.insert(key, map.clone());
        Ok(map)
    }

    fn compute_universe(&mut self, expr: &Expr) -> Result<MatchMap> {
        let mut universe = MatchMap::new();
        self.collect_node_matches(expr, &mut universe)?;
        Ok(universe)
    }

    fn collect_node_matches(&mut self, expr: &Expr, map: &mut MatchMap) -> Result<()> {
        match expr {
            Expr::Node { .. } => {
                let matches = self.eval_node(expr)?;
                for (key, record) in matches {
                    map.entry(key).or_insert(record);
                }
            }
            Expr::And(children) | Expr::Or(children) => {
                for child in children {
                    self.collect_node_matches(child, map)?;
                }
            }
            Expr::Not(child) => {
                self.collect_node_matches(child, map)?;
            }
        }
        Ok(())
    }
}

/// Process a single file with the given adapter and expression.
pub fn process_file(
    adapter: &dyn LangAdapter,
    path: &Path,
    expr: &Expr,
    context_lines: u32,
) -> Result<Vec<MatchRecord>> {
    let src = std::fs::read(path)?;

    if is_probably_binary(&src) {
        return Ok(vec![]);
    }

    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&adapter.language())?;
    let tree = parser
        .parse(&src, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse file"))?;

    let line_index = LineIndex::new(&src);
    let mut ctx = EvalContext {
        adapter,
        tree,
        src,
        line_index,
        path,
        context_lines,
        lang_name: adapter.name(),
        node_cache: HashMap::new(),
        query_cache: HashMap::new(),
        universe: None,
    };

    let universe = ctx.compute_universe(expr)?;
    ctx.universe = Some(universe);

    let matches = ctx.eval_expr(expr)?;
    Ok(matches.into_values().collect())
}

/// Apply DSL predicates to a capture bundle.
fn apply_predicates(preds: &[Pred], bundle: &CaptureBundle) -> bool {
    for pred in preds {
        let matched = match pred {
            Pred::Callee(re) => {
                bundle
                    .get("callee_id")
                    .map(|t| re.is_match(t))
                    .unwrap_or(false)
                    || bundle.get("prop").map(|t| re.is_match(t)).unwrap_or(false)
            }
            Pred::Name(re) => bundle.get("name").map(|t| re.is_match(t)).unwrap_or(false),
            Pred::Module(re) => bundle
                .get("module")
                .map(|t| re.is_match(t))
                .unwrap_or(false),
            Pred::Prop(re) => bundle.get("prop").map(|t| re.is_match(t)).unwrap_or(false),
            Pred::Arg(_) => {
                // TODO: Implement argument matching
                true
            }
            Pred::Text(re) => bundle
                .get("__node_text")
                .map(|t| re.is_match(t))
                .unwrap_or(false),
        };

        if !matched {
            return false;
        }
    }
    true
}

fn node_text(src: &[u8], node: &Node) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    String::from_utf8_lossy(&src[start..end]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::parse_query;
    use crate::languages::JavaScriptAdapter;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn multi_line_text_predicate_matches() {
        let mut temp = NamedTempFile::new().expect("create temp file");
        write!(
            temp,
            r#"async function test() {{
    const result = axios.get(
        'https://api.example.com/data',
        {{
            headers: {{
                Authorization: 'Bearer token',
            }},
        }},
    );
    return result;
}}
"#
        )
        .expect("write temp file");

        let adapter = JavaScriptAdapter;
        let sanity_expr = parse_query("call(prop=/^get$/)").expect("parse sanity query");
        let sanity_matches = process_file(&adapter, temp.path(), &sanity_expr, 0)
            .expect("process sanity file");
        assert_eq!(sanity_matches.len(), 1);

        let expr = parse_query(r"call(text=/axios\.get\(.*Authorization/)")
            .expect("parse query");
        let matches = process_file(&adapter, temp.path(), &expr, 0)
            .expect("process file");
        assert_eq!(matches.len(), 1);
    }
}
