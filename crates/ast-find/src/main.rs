mod adapter;
mod dsl;
mod languages;
mod processor;

use agent_tools_common::{write_ndjson, Event};
use anyhow::Result;
use clap::Parser;
use ignore::WalkBuilder;
use languages::{get_adapter, parse_lang_list, LangId, LANG_BY_EXT};
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Parser, Debug)]
#[command(name = "ast-find", about = "Structure-aware repository search")]
struct Opt {
    /// Directory to search (repo root)
    #[arg(long, default_value = ".")]
    within: String,
    /// Comma-separated language list (e.g., py,ts,js)
    #[arg(long)]
    lang: Option<String>,
    /// DSL query, e.g., call(callee=/^axios\.(get|post)$/)
    #[arg(long, default_value = "call(callee=/^foo$/)")]
    query: String,
    /// Lines of context to include in excerpts
    #[arg(long, default_value_t = 2)]
    context: u32,
    /// Maximum number of results
    #[arg(long, default_value_t = 5000)]
    max_results: usize,
}

fn main() -> Result<()> {
    // Deterministic environment
    std::env::set_var("NO_COLOR", "1");
    std::env::set_var("TZ", "UTC");

    let opt = Opt::parse();

    // Parse DSL query
    let expr = dsl::parse_query(&opt.query)?;

    // Parse language filter
    let lang_filter: Option<Vec<LangId>> = opt.lang.as_ref().map(|s| parse_lang_list(s));

    // Walk the directory and collect files
    let mut files = Vec::new();
    let walker = WalkBuilder::new(&opt.within)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker {
        let entry = entry?;
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            if let Some(ext) = entry.path().extension() {
                if let Some(lang_id) = LANG_BY_EXT.get(ext.to_str().unwrap_or("")) {
                    // Apply language filter
                    if let Some(ref filter) = lang_filter {
                        if !filter.contains(lang_id) {
                            continue;
                        }
                    }
                    files.push((entry.path().to_path_buf(), *lang_id));
                }
            }
        }
    }

    // Sort files for deterministic output
    files.sort_by(|a, b| a.0.cmp(&b.0));

    // Process files in parallel and collect results
    let results = Arc::new(Mutex::new(BTreeMap::new()));
    let max_results = opt.max_results;
    let context = opt.context;

    files.par_iter().for_each(|(path, lang_id)| {
        let adapter = get_adapter(*lang_id);
        match processor::process_file(adapter.as_ref(), path, &expr, context) {
            Ok(events) => {
                let mut results = results.lock().unwrap();
                for event in events {
                    if results.len() >= max_results {
                        return;
                    }
                    // Key by (path, start_line) for deterministic ordering
                    if let Event::Match {
                        ref path,
                        start_line,
                        ..
                    } = event
                    {
                        results.insert((path.clone(), start_line), event);
                    }
                }
            }
            Err(e) => {
                let err_event = Event::Error {
                    code: "E_PARSE".to_string(),
                    message: format!("{:#}", e),
                    path_or_url: Some(path.to_string_lossy().to_string()),
                };
                let mut results = results.lock().unwrap();
                results.insert((path.to_string_lossy().to_string(), 0), err_event);
            }
        }
    });

    // Output results in sorted order
    let results = results.lock().unwrap();
    for (_, event) in results.iter() {
        write_ndjson(event)?;
    }

    Ok(())
}
