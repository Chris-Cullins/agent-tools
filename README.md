# agent_tools

CLI tools for AI agent workflows: structure-aware code search and web content extraction.

See the [Wiki](https://github.com/Chris-Cullins/agent-tools/wiki) for more specifics.

## Tools

### **ast-find** — Structure-Aware Code Search

Tree-sitter powered code search with a simple DSL for finding calls, imports, and definitions across your codebase.

```bash
# Find all axios.get/post calls
ast-find --lang ts,js --query 'call(prop=/^(get|post)$/)' --within src/

# Find all imports of a module
ast-find --lang py --query 'import(module=/^requests$/)'

# Find function definitions
ast-find --lang js --query 'def(name=/^handle/)'
```


### **web-get** — Web Content → Markdown

Fetch web pages, extract main content, and convert to clean Markdown for LLM consumption.

```bash
# Fetch a single page
web-get "https://example.com/article"

# Batch fetch with stdin
cat urls.txt | web-get --concurrency 8

# Use CSS selector for precise extraction
web-get "https://blog.example.com" --selector "article, .post-content"
```


## Installation

Use the bundled installer to build both binaries in release mode and copy them
to `/usr/local/bin` (pass `--prefix`/`--destdir` to customize the target):

```bash
git clone https://github.com/Chris-Cullins/agent-tools.git
cd agent-tools
./install.sh
```

To install each crate manually:

```bash
cargo install --path crates/ast-find
cargo install --path crates/web-get
```


## Output Format

Both tools emit **NDJSON** (newline-delimited JSON) for easy parsing:

**ast-find:**
```json
{"type":"match","lang":"javascript","path":"./src/api.js","start_line":42,"end_line":42,"chunk_id":"abc123...","score":1.0,"excerpt":"...code...","capture":{"callee":"get","object":"axios"}}
```

**web-get:**
```json
{"type":"document","url":"https://example.com","title":"Page Title","text_md":"# Heading\n\nContent...","word_count":523,"links":["https://..."],"hash":"blake3hex"}
```


## Query DSL (ast-find)

For more information on the tree-sitter output that is read, see [The Tree-sitter Docs](https://tree-sitter.github.io/tree-sitter/index.html)

Simple pattern matching with regex predicates:

| Pattern | Example | Matches |
|---------|---------|---------|
| `call(callee=/regex/)` | `call(callee=/^fetch$/)` | Function calls |
| `call(prop=/regex/)` | `call(prop=/^(get\|post)$/)` | Method calls |
| `call(text=/regex/)` | `call(text=/axios\.get\(.*Authorization/)` | Full call text (multi-line) |
| `import(module=/regex/)` | `import(module=/^axios/)` | Import statements |
| `def(name=/regex/)` | `def(name=/^handle/)` | Function/class definitions |

### Multi-line Matching

Multi-line calls, imports, or definitions often defeat plain regex searches. Combine AST matching with the `text`/`code` predicate to stay accurate without giving up span-aware matching:

```bash
ast-find --lang js --query 'call(text=/axios\\.get\\(.*Authorization/)'
```

The query above only returns `call_expression` nodes whose full source (including nested options objects) mentions an `Authorization` header anywhere inside the call. Because the `text` predicate automatically turns on dot-all mode, `.*` will cross line breaks out of the box. The alias `code=/.../` behaves identically if you prefer a more descriptive keyword.

Combine queries using boolean operators:

- `and(expr, ...)` — results present in every operand
- `or(expr, ...)` — results from any operand
- `not(expr)` — removes operand matches from sibling expressions

Example: `and(call(prop=/log/), not(call(object=/console/)))`

## Agent Workflow Examples

**Code Search + Context:**
```bash
ast-find --lang ts --query 'call(prop=/^deprecated/)' --max-results 50 | \
  jq -r '.path' | sort -u > files_to_migrate.txt
```

**Documentation Retrieval:**
```bash
echo "https://docs.example.com/api
https://docs.example.com/auth" | \
  web-get --selector "article" | \
  jq -r 'select(.type=="document") | .text_md' > combined_docs.md
```

**Dependency Audit:**
```bash
ast-find --lang js --query 'import(module=/^[^\\.\/]/)' --max-results 1000 | \
  jq -r '.capture.module' | sort -u > all_dependencies.txt
```


## Roadmap

**ast-find:**
- [x] Go language adapter
- [x] Rust language adapter
- [x] Java language adapter
- [x] Boolean combinators (And, Or, Not)
- [ ] Incremental caching
- [x] Multi-line pattern matching

**web-get:**
- [ ] Advanced Readability scoring
- [ ] PDF text extraction (beyond stubs)
- [ ] Image alt-text extraction
- [ ] Table → Markdown table conversion

## Possible Future Tool Ideas

**git-diff-json** — Structured diffs and hunks  
Emit per-file changes and per-hunk spans between commits/branches/working tree. Avoids fragile parsing of patch text; directly yields file-level and hunk-level structures with stable IDs and spans.

**repo-ls** — Repo-aware file inventory  
Walk repo honoring .gitignore and emit files with metadata: size, lang, hash, line counts. Single pass, deterministic, with language detection and content hashes for deduplication.

**dep-scan** — Dependency manifest normalizer  
Parse manifests and lockfiles (package.json, Cargo.toml, pyproject.toml, etc.) into a unified dependency list. Cross-ecosystem, structured graph with resolved versions and sources.

**doc-index** — Markdown/MDX indexer and link graph  
Parse .md/.mdx to extract headings, anchors, links, and per-section spans. Structured table of contents and document graph; per-section chunking with stable IDs.

**link-check** — Concurrent HTTP link validator  
Verify internal/external links with redirects, status, canonical, content-type. Handles concurrency, timeouts, and outputs structured results.

**api-probe** — JSON API sampler and shape extractor  
Perform HEAD/GET/POST with sample payloads and emit structured response metadata plus inferred JSON shape (keys/types). Quickly map REST endpoint response shapes.

**code-chop** — Deterministic chunker for code/text  
Split files into stable, size-bounded chunks for embedding/context with language-aware boundaries. Stable chunk IDs and boundaries; reproducible spans.

**secret-find** — Deterministic secrets scanner  
Scan repo for likely secrets using curated detectors with entropy and context windows. Structured findings with types and confidence; consistent ignores and sorting.

**task-list** — Discover runnable project tasks  
Enumerate build/test tasks from Makefile, package.json scripts, Cargo.toml workspace, and common CI configs. Normalizes disparate task definitions into a single structured list.

## License

MIT
