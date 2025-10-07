# Agent Tools — Usage Guide for AI Agents

This document explains how to use `ast-find` and `web-get` in AI agent workflows. Both tools emit **deterministic NDJSON** (newline-delimited JSON) for reliable parsing in automated pipelines.

---

## `ast-find` — Structure-Aware Code Search

**Purpose**: Find code patterns across repositories using Tree-sitter AST queries instead of regex.

### When to Use

- Finding all calls to a specific function (e.g., `axios.get`, `requests.post`)
- Locating imports of a particular module
- Finding function/class definitions
- Building context for code understanding or refactoring tasks

### Basic Usage

```bash
ast-find \
  --lang <languages> \
  --query '<dsl-query>' \
  --within <directory> \
  [--context <lines>] \
  [--max-results <n>]
```

### Query Language (DSL)

The DSL supports three node types with regex predicates:

#### 1. **Function Calls** — `call(...)`

**Predicates:**
- `callee=/regex/` — Match simple function calls by name
- `prop=/regex/` — Match method/property calls (e.g., `obj.method()`)

**Examples:**
```bash
# Find axios.get or axios.post calls
ast-find --lang js,ts --query 'call(prop=/^(get|post)$/)'

# Find all console.log calls
ast-find --lang js --query 'call(prop=/^log$/)'

# Find fetch() calls
ast-find --lang js --query 'call(callee=/^fetch$/)'
```

**Note**: For member expressions like `axios.get()`, the capture returns:
- `capture.object` = `"axios"`
- `capture.callee` = `"get"`

#### 2. **Imports** — `import(...)`

**Predicates:**
- `module=/regex/` — Match import source

**Examples:**
```bash
# Find axios imports
ast-find --lang js,ts --query 'import(module=/axios/)'

# Find all React imports
ast-find --lang js,ts --query 'import(module=/^react$/)'

# Find relative imports
ast-find --lang py --query 'import(module=/^\.\.?\//'
```

#### 3. **Definitions** — `def(...)`

**Predicates:**
- `name=/regex/` — Match function/class name

**Examples:**
```bash
# Find functions starting with 'handle'
ast-find --lang js --query 'def(name=/^handle/)'

# Find a specific function
ast-find --lang py --query 'def(name=/^process_data$/)'
```

### Supported Languages

| Extension | Language ID | Adapter |
|-----------|-------------|---------|
| `.js`, `.jsx` | `javascript`, `js` | JavaScript |
| `.ts`, `.tsx` | `typescript`, `ts` | TypeScript |
| `.py` | `python`, `py` | Python |
| `.cs`, `.csx` | `csharp`, `cs` | C# |
| `.rs` | `rust`, `rs` | Rust |
| `.go` | `go`, `go` | Go |
| `.java` | `java`, `java` | Java |

Specify multiple languages: `--lang py,js,ts,cs,rs,go,java`

### Output Format

Each match emits a JSON object:

```json
{
  "type": "match",
  "lang": "javascript",
  "path": "./src/api.js",
  "start_line": 42,
  "end_line": 42,
  "chunk_id": "abc123...",
  "score": 1.0,
  "excerpt": "...\nconst data = await axios.get(url);\n...",
  "capture": {
    "callee": "get",
    "object": "axios",
    "module": null,
    "name": null,
    "attr": null
  }
}
```

**Key Fields:**
- `chunk_id` — Deterministic hash of `path:start_line-end_line` (stable across runs)
- `excerpt` — Source code with `--context` lines before/after (default: 2)
- `capture` — Extracted AST node texts (varies by query type)

### Agent Workflow Example

```bash
# 1. Find all axios calls in a project
ast-find --lang ts --query 'call(prop=/^(get|post|put|delete)$/)' \
  --within ./src \
  --max-results 100 > axios_calls.jsonl

# 2. Process results with jq
cat axios_calls.jsonl | \
  jq -r 'select(.capture.object == "axios") |
         "\(.path):\(.start_line) - \(.capture.callee)"'

# 3. Extract just the excerpts for LLM context
cat axios_calls.jsonl | \
  jq -r '.excerpt' | \
  head -c 12000  # Stay within token budget
```

### Performance Notes

- Uses `.gitignore` automatically (via `ignore` crate)
- Parallelizes file processing with `rayon`
- Skips binary files automatically
- Outputs results in deterministic order (sorted by path + line)

---

## `web-get` — Web Content → Markdown

**Purpose**: Fetch web pages, extract main content, sanitize HTML, and convert to clean Markdown for LLM consumption.

### When to Use

- Fetching documentation for context
- Extracting article/blog post content
- Building knowledge base from web sources
- Pre-processing URLs for summarization/Q&A

### Basic Usage

```bash
# Single URL
web-get "https://example.com/article"

# Multiple URLs (stdin)
cat urls.txt | web-get

# With CSS selector
web-get "https://blog.example.com/post" --selector "article, main"

# Adjust limits
web-get "https://example.com" \
  --max-bytes 5MB \
  --timeout 30s \
  --concurrency 4
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--selector <css>` | Auto | CSS selector for main content (e.g., `article`, `.post-content`) |
| `--max-bytes <size>` | `10MB` | Max response size (e.g., `5MB`, `1GB`) |
| `--timeout <duration>` | `15s` | Request timeout (e.g., `30s`, `1m`) |
| `--keep-images` | `false` | Preserve `<img>` tags in Markdown |
| `--concurrency <n>` | `6` | Max parallel requests |

### Content Extraction

#### 1. **Selector-Based** (if `--selector` provided)
- Finds first match or largest by text content
- Good for known site structures

#### 2. **Heuristic (Readability-lite)**
If no selector:
- Prioritizes `<article>`, `<main>`, `.content` tags
- Removes navigation, sidebars, ads by class/ID patterns
- Fallback to `<body>` if no candidates

### Output Format

```json
{
  "type": "document",
  "url": "https://example.com/final-url",
  "title": "Page Title",
  "byline": "Author Name",
  "text_md": "# Heading\n\nParagraph...",
  "word_count": 1523,
  "links": [
    "https://example.com/related",
    "https://external.com/ref"
  ],
  "canonical_url": "https://example.com/canonical",
  "media_type": "text/html",
  "hash": "blake3-hex-digest"
}
```

**Key Fields:**
- `text_md` — Sanitized Markdown content
- `hash` — Deterministic content hash (for deduplication)
- `links` — All absolute HTTP(S) links extracted from `<a>` tags
- `canonical_url` — From `<link rel="canonical">` if present

### Error Handling

Errors emit:
```json
{
  "type": "error",
  "code": "E_FETCH",
  "message": "Network timeout",
  "path_or_url": "https://example.com"
}
```

Common error codes:
- `E_FETCH` — Network/HTTP error
- `E_MEDIA` — Unsupported content type (though PDFs return stub documents)

### Agent Workflow Example

```bash
# 1. Fetch multiple docs
echo "https://docs.example.com/guide
https://blog.example.com/tutorial" | \
  web-get --selector "article" --max-bytes 2MB > docs.jsonl

# 2. Extract just markdown for LLM
cat docs.jsonl | \
  jq -r 'select(.type == "document") | .text_md' > combined.md

# 3. Get metadata summary
cat docs.jsonl | \
  jq '{url, title, words: .word_count, links: (.links | length)}'
```

### Charset & Encoding

- Respects `Content-Type` charset header
- Falls back to `chardetng` auto-detection
- Handles UTF-8, Windows-1252, Shift-JIS, etc.

### PDF Handling

PDFs return stub documents with:
```json
{
  "type": "document",
  "media_type": "application/pdf",
  "text_md": "",
  "word_count": 0,
  "hash": "blake3-of-raw-bytes"
}
```

**Recommendation**: Use a separate PDF extraction tool (e.g., `pdftotext`, `pdf-chop`) and feed results back through your pipeline.

---

## Common Patterns for Agents

### 1. **Code Search + Context Packing**

```bash
# Find all error handling
ast-find --lang py --query 'call(callee=/^(raise|except)$/)' \
  --max-results 50 > errors.jsonl

# Extract unique files
cat errors.jsonl | jq -r '.path' | sort -u > files_to_review.txt
```

### 2. **Documentation Retrieval**

```bash
# Fetch API docs
echo "https://api.example.com/docs/auth
https://api.example.com/docs/endpoints" | \
  web-get --selector ".api-content" | \
  jq -s 'map(select(.type == "document")) |
         {combined: (map(.text_md) | join("\n\n---\n\n"))}'
```

### 3. **Change Impact Analysis**

```bash
# Find all usages of a function
ast-find --lang ts --query 'call(callee=/^deprecatedFunction$/)' \
  --within src/ > usages.jsonl

# Count by file
cat usages.jsonl | jq -r '.path' | sort | uniq -c
```

### 4. **Dependency Audit**

```bash
# Find all third-party imports
ast-find --lang js --query 'import(module=/^[^\.\/]/)' \
  --max-results 1000 | \
  jq -r '.capture.module' | sort -u > dependencies.txt
```

---

## Environment & Determinism

Both tools enforce deterministic output:

```bash
export NO_COLOR=1    # Disable ANSI colors
export TZ=UTC        # Stable timestamps
export LC_ALL=C      # Consistent sorting
```

These are set automatically by the tools but can be overridden.

---

## Performance Tuning

### `ast-find`

- **Filter early**: Use `--lang` to skip irrelevant files
- **Limit results**: Set `--max-results` to avoid processing entire repos
- **Reduce context**: Use `--context 0` if excerpts aren't needed

### `web-get`

- **Adjust concurrency**: Higher `--concurrency` for many small pages
- **Set tight timeouts**: Use `--timeout 5s` for responsive sites
- **Limit size**: Use `--max-bytes 1MB` to skip huge pages

---

## Troubleshooting

### `ast-find` returns no results

1. **Check language support**: Only JS/TS/Python implemented (v1)
2. **Verify file extensions**: Must match `.js`, `.ts`, `.py`, etc.
3. **Inspect Tree-sitter errors**: Look for `E_PARSE` in output

### `web-get` extracts wrong content

1. **Inspect with selector**: Try `--selector "article"` or `.main-content`
2. **Check raw HTML**: Fetch with `curl` to verify structure
3. **Review links/canonical**: Check `canonical_url` field for redirects

### High memory usage

- `ast-find`: Collecting too many results; reduce `--max-results`
- `web-get`: Fetching huge pages; lower `--max-bytes`

---

## Extending the Tools

### Adding Languages to `ast-find`

1. Add Tree-sitter grammar to `Cargo.toml`
2. Create adapter in `crates/ast-find/src/languages/`
3. Register in `LANG_BY_EXT` map
4. Write Tree-sitter queries for `call`, `import`, `def`

### Adding Combinators to DSL

Current DSL is simple (v1). Future: `And()`, `Or()`, `Not()`.

See `crates/ast-find/src/dsl.rs` for IR types.

---

## Schema Reference

### NDJSON Event Types (common)

Both tools use `agent_tools_common::Event`:

```rust
enum Event {
    Match { ... },       // ast-find results
    Document { ... },    // web-get results
    Error { ... },       // Errors from either tool
    Summary { ... },     // Deprecated (stub mode only)
}
```

Filter by `type` field: `jq 'select(.type == "match")'`

---

## Version

- **ast-find**: v0.1.0 (Day 1 milestone: call/import/def for JS/TS/Python)
- **web-get**: v0.1.0 (HTML extraction + Markdown conversion)

See `agent_tools_ast-find_web-get_blueprint.md` for roadmap.
