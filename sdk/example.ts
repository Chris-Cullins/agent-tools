/**
 * Example usage of agent-tools with Claude Agent SDK
 */
import { query } from "@anthropic-ai/claude-agent-sdk";
import createAgentToolsServer from "./src/index.js";

// Create the MCP server
const agentTools = createAgentToolsServer();

// Example 1: Code Search
async function* codeSearchExample() {
  yield {
    role: "user" as const,
    content: "Find all axios.get and axios.post calls in the src/ directory and show me the context",
  };
}

// Example 2: Documentation Retrieval
async function* docRetrievalExample() {
  yield {
    role: "user" as const,
    content: "Fetch the documentation from https://docs.anthropic.com/en/api/getting-started and summarize the key points",
  };
}

// Example 3: Combined Workflow
async function* combinedExample() {
  yield {
    role: "user" as const,
    content: `I need to migrate deprecated API calls.

    First, find all calls to deprecated functions (anything starting with 'legacy') in src/.
    Then fetch the migration guide from https://example.com/migration-guide.
    Finally, suggest how to update each deprecated call based on the guide.`,
  };
}

// Run example
async function main() {
  console.log("Starting Claude Agent SDK example with agent-tools...\n");

  for await (const message of query({
    prompt: codeSearchExample(),
    options: {
      mcpServers: {
        "agent-tools": agentTools,
      },
      allowedTools: [
        "mcp__agent-tools__ast_find",
        "mcp__agent-tools__web_get",
      ],
      maxTurns: 10,
    },
  })) {
    if (message.type === "text") {
      console.log("Claude:", message.text);
    } else if (message.type === "tool_use") {
      console.log(`\n[Using tool: ${message.name}]`);
      console.log("Input:", JSON.stringify(message.input, null, 2));
    } else if (message.type === "tool_result") {
      console.log(`\n[Tool result for: ${message.tool_use_id}]`);
    }
  }
}

// Uncomment to run
// main().catch(console.error);
