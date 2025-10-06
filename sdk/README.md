# Claude Agent SDK Integration for agent-tools

Plug-and-play integration for using `ast-find` and `web-get` with the [Claude Agent SDK](https://docs.claude.com/en/api/agent-sdk).

## Prerequisites

1. Install the CLI tools (from repository root):
   ```bash
   ./install.sh
   ```

2. Verify installation:
   ```bash
   ast-find --version
   web-get --version
   ```

## Installation

```bash
cd sdk
npm install
npm run build
```

## Usage

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";
import createAgentToolsServer from "@agent-tools/claude-sdk";

// Create the MCP server with your tools
const agentTools = createAgentToolsServer();

// Use in a query
async function* generateMessages() {
  yield {
    role: "user" as const,
    content: "Find all axios.get calls in the src/ directory"
  };
}

for await (const message of query({
  prompt: generateMessages(),
  options: {
    mcpServers: {
      "agent-tools": agentTools
    },
    allowedTools: [
      "mcp__agent-tools__ast_find",
      "mcp__agent-tools__web_get"
    ],
    maxTurns: 5
  }
})) {
  if (message.type === "text") {
    console.log(message.text);
  }
}
```

## Available Tools

### `mcp__agent-tools__ast_find`

Structure-aware code search using Tree-sitter AST queries.

**Parameters:**
- `query` (string, required) — DSL query pattern
  - Examples: `call(prop=/^get$/)`, `import(module=/^axios$/)`, `def(name=/^handle/)`
- `languages` (string, default: "js,ts,py,cs") — Comma-separated languages
- `directory` (string, default: ".") — Directory to search
- `maxResults` (number, default: 100) — Max results
- `context` (number, default: 2) — Context lines before/after match

**Example Queries:**
```typescript
// Find all method calls
"call(prop=/^(get|post|put|delete)$/)"

// Find specific imports
"import(module=/^react$/)"

// Find function definitions
"def(name=/^handle/)"
```

### `mcp__agent-tools__web_get`

Fetch web pages and convert to clean Markdown.

**Parameters:**
- `urls` (string[], required) — List of URLs to fetch
- `selector` (string, optional) — CSS selector for content extraction
- `maxBytes` (string, default: "10MB") — Max response size
- `timeout` (string, default: "15s") — Request timeout
- `concurrency` (number, default: 6) — Max parallel requests
- `keepImages` (boolean, default: false) — Preserve image tags

## Custom Binary Paths

If `ast-find` and `web-get` are not in your PATH:

```typescript
const agentTools = createAgentToolsServer({
  astFindPath: "/custom/path/to/ast-find",
  webGetPath: "/custom/path/to/web-get"
});
```

## Example: Code Migration Assistant

```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";
import createAgentToolsServer from "@agent-tools/claude-sdk";

const agentTools = createAgentToolsServer();

async function* generateMessages() {
  yield {
    role: "user" as const,
    content: "Find all deprecated API calls in src/ and fetch the latest documentation from https://api.example.com/docs"
  };
}

for await (const message of query({
  prompt: generateMessages(),
  options: {
    mcpServers: {
      "agent-tools": agentTools
    },
    allowedTools: [
      "mcp__agent-tools__ast_find",
      "mcp__agent-tools__web_get"
    ],
    maxTurns: 10
  }
})) {
  if (message.type === "text") {
    console.log(message.text);
  } else if (message.type === "tool_use") {
    console.log(`Using tool: ${message.name}`);
  }
}
```

## Output Format

Both tools return formatted Markdown:

**ast_find** returns:
```markdown
# Code Search Results (5 matches)

## src/api.ts:42

**Language:** typescript
**Call:** `axios.get`

```typescript
const data = await axios.get(url);
```

---
```

**web_get** returns:
```markdown
# Web Content (1 documents)

## Example Documentation

**URL:** https://example.com/docs
**Word Count:** 1234

---

# Documentation Content

[Converted markdown content...]

### Links (15)

- https://example.com/guide
- https://example.com/api
...
```

## Query DSL Reference (ast-find)

| Pattern | Example | Matches |
|---------|---------|---------|
| `call(callee=/regex/)` | `call(callee=/^fetch$/)` | Function calls |
| `call(prop=/regex/)` | `call(prop=/^(get\|post)$/)` | Method calls |
| `import(module=/regex/)` | `import(module=/^axios/)` | Import statements |
| `def(name=/regex/)` | `def(name=/^handle/)` | Function/class definitions |

## Supported Languages

- JavaScript (`.js`, `.jsx`)
- TypeScript (`.ts`, `.tsx`)
- Python (`.py`)
- C# (`.cs`, `.csx`)

## License

MIT
