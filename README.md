# agent_tools

CLI tools for AI agent workflows: structure-aware code search and web content extraction.

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

**Features:**
- ✅ JavaScript, TypeScript, Python, C# support
- ✅ Deterministic NDJSON output
- ✅ Parallel file processing with rayon
- ✅ Git-aware file walking (.gitignore respected)
- ✅ Context-aware excerpts with configurable line range

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

**Features:**
- ✅ Readability-lite content extraction
- ✅ HTML sanitization with ammonia
- ✅ Charset auto-detection
- ✅ Configurable size/timeout limits
- ✅ Bounded concurrency for batch fetching
- ✅ Link extraction and canonical URL support

## Installation

Use the bundled installer to build both binaries in release mode and copy them
to `/usr/local/bin` (pass `--prefix`/`--destdir` to customize the target):

```bash
git clone https://github.com/Chris-Cullins/agent-tools.git
cd agent-tools
./install.sh
```

Prefer to install each crate manually? You can still use `cargo install`:

```bash
cargo install --path crates/ast-find
cargo install --path crates/web-get
```

## Quick Start

```bash
# Clone and build
git clone https://github.com/Chris-Cullins/agent-tools.git
cd agent-tools
cargo build --release

# Test ast-find
echo 'import axios from "axios"; axios.get(url);' > test.js
cargo run -p ast-find -- --lang js --query 'call(prop=/^get$/)' --within .

# Test web-get
cargo run -p web-get -- "https://example.com"
```

## Documentation

- **[AGENTS.md](./AGENTS.md)** — Complete usage guide for AI agents
- **[agent_tools_ast-find_web-get_blueprint.md](./agent_tools_ast-find_web-get_blueprint.md)** — Implementation blueprint and design doc

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

Simple pattern matching with regex predicates:

| Pattern | Example | Matches |
|---------|---------|---------|
| `call(callee=/regex/)` | `call(callee=/^fetch$/)` | Function calls |
| `call(prop=/regex/)` | `call(prop=/^(get\|post)$/)` | Method calls |
| `import(module=/regex/)` | `import(module=/^axios/)` | Import statements |
| `def(name=/regex/)` | `def(name=/^handle/)` | Function/class definitions |

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

## Architecture

**Workspace Structure:**
```
agent_tools/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── ast-find/              # Tree-sitter based code search
│   │   ├── src/
│   │   │   ├── dsl.rs         # Query DSL parser
│   │   │   ├── adapter.rs     # Language adapter trait
│   │   │   ├── languages/     # JS, TS, Python adapters
│   │   │   ├── processor.rs   # File processing logic
│   │   │   └── main.rs        # CLI & orchestration
│   ├── web-get/               # Web content extraction
│   │   ├── src/
│   │   │   ├── fetch.rs       # HTTP with limits
│   │   │   ├── extract.rs     # Content extraction
│   │   │   ├── convert.rs     # HTML → Markdown
│   │   │   └── main.rs        # Async CLI
│   └── common/                # Shared utilities
│       └── src/lib.rs         # NDJSON events, LineIndex, helpers
```

## Contributing

See [agent_tools_ast-find_web-get_blueprint.md](./agent_tools_ast-find_web-get_blueprint.md) for:
- Detailed implementation notes
- Adding new languages to ast-find
- Extending the query DSL
- Performance optimization strategies

## Roadmap

**ast-find:**
- [x] Go language adapter
- [x] Rust language adapter
- [x] Java language adapter
- [x] Boolean combinators (And, Or, Not)
- [ ] Incremental caching
- [ ] Multi-line pattern matching

**web-get:**
- [ ] Advanced Readability scoring
- [ ] PDF text extraction (beyond stubs)
- [ ] Image alt-text extraction
- [ ] Table → Markdown table conversion

## License

MIT

## Acknowledgments

Built following agent-first design principles:
- Deterministic output (NO_COLOR, UTC, sorted results)
- NDJSON for reliable parsing
- Hard limits (size, timeout, max results)
- Stable chunk IDs for content addressing
