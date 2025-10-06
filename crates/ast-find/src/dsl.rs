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
    // Future: Or, And, Not combinators
}

/// Simple DSL parser (v1).
/// Supports patterns like: call(callee=/regex/)
pub fn parse_query(input: &str) -> anyhow::Result<Expr> {
    let input = input.trim();

    // Parse: kind(pred=value, ...)
    if let Some(paren_idx) = input.find('(') {
        let kind_str = &input[..paren_idx];
        let kind = match kind_str.trim() {
            "call" => Kind::Call,
            "import" => Kind::Import,
            "def" => Kind::Def,
            other => anyhow::bail!("Unknown kind: {}", other),
        };

        // Extract predicates between ( and )
        let end = input
            .rfind(')')
            .ok_or_else(|| anyhow::anyhow!("Missing closing ')'"))?;
        let preds_str = &input[paren_idx + 1..end];

        let mut preds = Vec::new();
        if !preds_str.trim().is_empty() {
            for part in split_predicates(preds_str) {
                if let Some(eq_idx) = part.find('=') {
                    let field = part[..eq_idx].trim();
                    let value = part[eq_idx + 1..].trim();

                    // Parse regex from /pattern/
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
        }

        Ok(Expr::Node { kind, preds })
    } else {
        anyhow::bail!("Invalid query format: {}", input);
    }
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
}
