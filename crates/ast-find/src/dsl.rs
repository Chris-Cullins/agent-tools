/// DSL types for structure-aware code search.
///
/// Examples:
/// - call(callee=/^axios\.(get|post)$/)
/// - import(module=/^requests$/)
/// - def(name=/^verifyJwt$/)
use regex::Regex;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Call,
    Import,
    Def,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Call => write!(f, "call"),
            Kind::Import => write!(f, "import"),
            Kind::Def => write!(f, "def"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Pred {
    Callee(Regex),
    Name(Regex),
    Module(Regex),
    Prop(Regex),
    Arg(Regex),
}

impl Pred {
    pub fn matches(&self, field: &str, text: &str) -> bool {
        match self {
            Pred::Callee(re) if field == "callee" => re.is_match(text),
            Pred::Name(re) if field == "name" => re.is_match(text),
            Pred::Module(re) if field == "module" => re.is_match(text),
            Pred::Prop(re) if field == "prop" => re.is_match(text),
            Pred::Arg(re) if field == "arg" => re.is_match(text),
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Node { kind: Kind, preds: Vec<Pred> },
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
}

/// Simple DSL parser with boolean combinators.
/// Supports patterns like:
/// - call(callee=/regex/)
/// - and(call(...), not(import(...)))
pub fn parse_query(input: &str) -> anyhow::Result<Expr> {
    let mut parser = Parser::new(input);
    let expr = parser.parse_expr()?;
    parser.skip_ws();
    if !parser.is_eof() {
        anyhow::bail!("Unexpected token at position {}", parser.pos);
    }
    Ok(expr)
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src: src.trim(),
            pos: 0,
        }
    }

    fn parse_expr(&mut self) -> anyhow::Result<Expr> {
        self.skip_ws();

        if self.consume_keyword("and") {
            let items = self.parse_expr_list()?;
            if items.is_empty() {
                anyhow::bail!("and() requires at least one operand");
            }
            return Ok(Expr::And(items));
        }

        if self.consume_keyword("or") {
            let items = self.parse_expr_list()?;
            if items.is_empty() {
                anyhow::bail!("or() requires at least one operand");
            }
            return Ok(Expr::Or(items));
        }

        if self.consume_keyword("not") {
            let mut items = self.parse_expr_list()?;
            if items.len() != 1 {
                anyhow::bail!("not() requires exactly one operand");
            }
            return Ok(Expr::Not(Box::new(items.remove(0))));
        }

        self.parse_node_expr()
    }

    fn parse_expr_list(&mut self) -> anyhow::Result<Vec<Expr>> {
        self.expect('(')?;
        let mut items = Vec::new();

        loop {
            self.skip_ws();
            if self.peek_char() == Some(')') {
                self.pos += 1;
                break;
            }

            let expr = self.parse_expr()?;
            items.push(expr);
            self.skip_ws();

            match self.peek_char() {
                Some(',') => {
                    self.pos += 1;
                }
                Some(')') => {
                    self.pos += 1;
                    break;
                }
                Some(other) => {
                    anyhow::bail!("Unexpected character '{}' in expression list", other);
                }
                None => anyhow::bail!("Unterminated expression list"),
            }
        }

        Ok(items)
    }

    fn parse_node_expr(&mut self) -> anyhow::Result<Expr> {
        self.skip_ws();

        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if ch.is_alphabetic() {
                self.pos += 1;
            } else {
                break;
            }
        }

        let kind_str = &self.src[start..self.pos];
        let kind = match kind_str {
            "call" => Kind::Call,
            "import" => Kind::Import,
            "def" => Kind::Def,
            "" => anyhow::bail!("Expected expression"),
            other => anyhow::bail!("Unknown kind: {}", other),
        };

        let predicates_raw = self.read_group_contents()?;
        let preds = parse_predicates(&predicates_raw)?;
        Ok(Expr::Node { kind, preds })
    }

    fn read_group_contents(&mut self) -> anyhow::Result<String> {
        self.expect('(')?;
        let mut contents = String::new();
        let mut depth = 1;
        let mut in_regex = false;

        while let Some(ch) = self.next_char() {
            match ch {
                '/' => {
                    in_regex = !in_regex;
                    contents.push(ch);
                }
                '(' if !in_regex => {
                    depth += 1;
                    contents.push(ch);
                }
                ')' if !in_regex => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    contents.push(ch);
                }
                _ => contents.push(ch),
            }
        }

        if depth != 0 {
            anyhow::bail!("Unterminated group");
        }

        Ok(contents)
    }

    fn skip_ws(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn consume_keyword(&mut self, kw: &str) -> bool {
        self.skip_ws();
        if self.src[self.pos..].starts_with(kw)
            && self.src[self.pos + kw.len()..]
                .chars()
                .next()
                .map(|ch| ch == '(' || ch.is_whitespace())
                .unwrap_or(false)
        {
            self.pos += kw.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, ch: char) -> anyhow::Result<()> {
        self.skip_ws();
        match self.peek_char() {
            Some(actual) if actual == ch => {
                self.pos += 1;
                Ok(())
            }
            Some(actual) => anyhow::bail!("Expected '{}' but found '{}'", ch, actual),
            None => anyhow::bail!("Expected '{}' but found end of input", ch),
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some(ch) = self.peek_char() {
            self.pos += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.src.len()
    }
}

fn parse_predicates(preds_str: &str) -> anyhow::Result<Vec<Pred>> {
    let mut preds = Vec::new();
    let trimmed = preds_str.trim();
    if trimmed.is_empty() {
        return Ok(preds);
    }

    for part in split_predicates(trimmed) {
        if let Some(eq_idx) = part.find('=') {
            let field = part[..eq_idx].trim();
            let value = part[eq_idx + 1..].trim();

            let pattern = if value.starts_with('/') && value.ends_with('/') {
                &value[1..value.len() - 1]
            } else {
                anyhow::bail!("Expected regex pattern like /.../ for {}", field);
            };

            let re = Regex::new(pattern)
                .map_err(|e| anyhow::anyhow!("Invalid regex for {}: {}", field, e))?;

            let pred = match field {
                "callee" => Pred::Callee(re),
                "name" => Pred::Name(re),
                "module" => Pred::Module(re),
                "prop" => Pred::Prop(re),
                "arg" => Pred::Arg(re),
                _ => anyhow::bail!("Unknown predicate field: {}", field),
            };

            preds.push(pred);
        } else {
            anyhow::bail!("Invalid predicate format: {}", part);
        }
    }

    Ok(preds)
}

/// Split predicates by commas, respecting regex delimiters.
fn split_predicates(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_regex = false;

    for ch in s.chars() {
        match ch {
            '/' => {
                in_regex = !in_regex;
                current.push(ch);
            }
            ',' if !in_regex => {
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_call() {
        let expr = parse_query("call(callee=/^axios\\.get$/)").unwrap();
        if let Expr::Node { kind, preds } = expr {
            assert_eq!(kind, Kind::Call);
            assert_eq!(preds.len(), 1);
        } else {
            panic!("Expected Node");
        }
    }

    #[test]
    fn test_parse_import() {
        let expr = parse_query("import(module=/^requests$/)").unwrap();
        if let Expr::Node { kind, .. } = expr {
            assert_eq!(kind, Kind::Import);
        }
    }

    #[test]
    fn test_parse_multiple_preds() {
        let expr = parse_query("call(callee=/foo/, arg=/bar/)").unwrap();
        if let Expr::Node { preds, .. } = expr {
            assert_eq!(preds.len(), 2);
        }
    }

    #[test]
    fn test_parse_and_expr() {
        let expr = parse_query("and(call(callee=/foo/), not(import(module=/bar/)))").unwrap();
        match expr {
            Expr::And(children) => {
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], Expr::Node { .. }));
                assert!(matches!(children[1], Expr::Not(_)));
            }
            _ => panic!("expected And expression"),
        }
    }

    #[test]
    fn test_parse_or_expr() {
        let expr = parse_query("or(call(callee=/foo/), call(callee=/bar/))").unwrap();
        match expr {
            Expr::Or(children) => {
                assert_eq!(children.len(), 2);
            }
            _ => panic!("expected Or expression"),
        }
    }

    #[test]
    fn test_not_requires_single_operand() {
        assert!(parse_query("not(call(callee=/foo/), call(callee=/bar/))").is_err());
    }
}
