import { tool, createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import { z } from "zod";
import { spawn } from "child_process";

/**
 * NDJSON event types from agent-tools
 */
interface MatchEvent {
  type: "match";
  lang: string;
  path: string;
  start_line: number;
  end_line: number;
  chunk_id: string;
  score: number;
  excerpt: string;
  capture: {
    callee?: string | null;
    object?: string | null;
    module?: string | null;
    name?: string | null;
    attr?: string | null;
  };
}

interface DocumentEvent {
  type: "document";
  url: string;
  title: string;
  byline?: string | null;
  text_md: string;
  word_count: number;
  links: string[];
  canonical_url?: string | null;
  media_type: string;
  hash: string;
}

interface ErrorEvent {
  type: "error";
  code: string;
  message: string;
  path_or_url: string;
}

type AgentToolEvent = MatchEvent | DocumentEvent | ErrorEvent;

/**
 * Execute a command and parse NDJSON output
 */
async function execCommand(
  command: string,
  args: string[],
  stdin?: string
): Promise<AgentToolEvent[]> {
  return new Promise((resolve, reject) => {
    const proc = spawn(command, args, {
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    proc.stderr.on("data", (data) => {
      stderr += data.toString();
    });

    proc.on("close", (code) => {
      if (code !== 0) {
        reject(
          new Error(`${command} exited with code ${code}:\n${stderr}`)
        );
        return;
      }

      try {
        const events = stdout
          .trim()
          .split("\n")
          .filter((line) => line.length > 0)
          .map((line) => JSON.parse(line) as AgentToolEvent);
        resolve(events);
      } catch (err) {
        reject(new Error(`Failed to parse NDJSON output: ${err}`));
      }
    });

    proc.on("error", (err) => {
      reject(new Error(`Failed to spawn ${command}: ${err}`));
    });

    if (stdin) {
      proc.stdin.write(stdin);
      proc.stdin.end();
    }
  });
}

/**
 * Format events as markdown for Claude
 */
function formatMatchEvents(events: AgentToolEvent[]): string {
  const matches = events.filter(
    (e): e is MatchEvent => e.type === "match"
  );
  const errors = events.filter(
    (e): e is ErrorEvent => e.type === "error"
  );

  let output = "";

  if (matches.length > 0) {
    output += `# Code Search Results (${matches.length} matches)\n\n`;
    for (const match of matches) {
      output += `## ${match.path}:${match.start_line}\n\n`;
      output += `**Language:** ${match.lang}\n\n`;
      if (match.capture.object && match.capture.callee) {
        output += `**Call:** \`${match.capture.object}.${match.capture.callee}\`\n\n`;
      } else if (match.capture.callee) {
        output += `**Function:** \`${match.capture.callee}\`\n\n`;
      } else if (match.capture.module) {
        output += `**Module:** \`${match.capture.module}\`\n\n`;
      } else if (match.capture.name) {
        output += `**Definition:** \`${match.capture.name}\`\n\n`;
      }
      output += "```" + match.lang + "\n" + match.excerpt + "\n```\n\n";
      output += "---\n\n";
    }
  }

  if (errors.length > 0) {
    output += `# Errors (${errors.length})\n\n`;
    for (const error of errors) {
      output += `- **${error.code}**: ${error.message} (${error.path_or_url})\n`;
    }
  }

  return output || "No matches found.";
}

function formatDocumentEvents(events: AgentToolEvent[]): string {
  const docs = events.filter(
    (e): e is DocumentEvent => e.type === "document"
  );
  const errors = events.filter(
    (e): e is ErrorEvent => e.type === "error"
  );

  let output = "";

  if (docs.length > 0) {
    output += `# Web Content (${docs.length} documents)\n\n`;
    for (const doc of docs) {
      output += `## ${doc.title || "Untitled"}\n\n`;
      output += `**URL:** ${doc.url}\n\n`;
      if (doc.byline) {
        output += `**Author:** ${doc.byline}\n\n`;
      }
      output += `**Word Count:** ${doc.word_count}\n\n`;
      if (doc.canonical_url && doc.canonical_url !== doc.url) {
        output += `**Canonical URL:** ${doc.canonical_url}\n\n`;
      }
      output += "---\n\n";
      output += doc.text_md;
      output += "\n\n";
      if (doc.links.length > 0) {
        output += `### Links (${doc.links.length})\n\n`;
        output += doc.links.slice(0, 20).map((l) => `- ${l}`).join("\n");
        if (doc.links.length > 20) {
          output += `\n\n... and ${doc.links.length - 20} more`;
        }
        output += "\n\n";
      }
      output += "---\n\n";
    }
  }

  if (errors.length > 0) {
    output += `# Errors (${errors.length})\n\n`;
    for (const error of errors) {
      output += `- **${error.code}**: ${error.message} (${error.path_or_url})\n`;
    }
  }

  return output || "No documents retrieved.";
}

/**
 * Create an MCP server with ast-find and web-get tools
 */
export function createAgentToolsServer(options?: {
  astFindPath?: string;
  webGetPath?: string;
}) {
  const astFindCmd = options?.astFindPath || "ast-find";
  const webGetCmd = options?.webGetPath || "web-get";

  return createSdkMcpServer({
    name: "agent-tools",
    version: "0.1.0",
    tools: [
      tool(
        "ast_find",
        "Search for code patterns using Tree-sitter AST queries. Supports finding function calls, imports, and definitions across JavaScript, TypeScript, Python, and C# codebases.",
        {
          query: z
            .string()
            .describe(
              "DSL query pattern. Examples: call(prop=/^get$/), import(module=/^axios$/), def(name=/^handle/)"
            ),
          languages: z
            .string()
            .default("js,ts,py,cs")
            .describe(
              "Comma-separated list of languages to search (js, ts, py, cs)"
            ),
          directory: z
            .string()
            .default(".")
            .describe("Directory to search within"),
          maxResults: z
            .number()
            .default(100)
            .describe("Maximum number of results to return"),
          context: z
            .number()
            .default(2)
            .describe("Number of context lines before/after each match"),
        },
        async (args) => {
          const cmdArgs = [
            "--lang",
            args.languages,
            "--query",
            args.query,
            "--within",
            args.directory,
            "--context",
            String(args.context),
            "--max-results",
            String(args.maxResults),
          ];

          try {
            const events = await execCommand(astFindCmd, cmdArgs);
            const formatted = formatMatchEvents(events);

            return {
              content: [
                {
                  type: "text" as const,
                  text: formatted,
                },
              ],
            };
          } catch (err) {
            return {
              content: [
                {
                  type: "text" as const,
                  text: `Error running ast-find: ${err}`,
                },
              ],
              isError: true,
            };
          }
        }
      ),

      tool(
        "web_get",
        "Fetch web pages and convert to clean Markdown. Extracts main content, handles charset detection, and sanitizes HTML.",
        {
          urls: z
            .array(z.string().url())
            .describe("List of URLs to fetch"),
          selector: z
            .string()
            .optional()
            .describe("Optional CSS selector for content extraction"),
          maxBytes: z
            .string()
            .default("10MB")
            .describe("Max response size (e.g., 5MB, 1GB)"),
          timeout: z
            .string()
            .default("15s")
            .describe("Request timeout (e.g., 30s, 1m)"),
          concurrency: z
            .number()
            .default(6)
            .describe("Max parallel requests"),
          keepImages: z
            .boolean()
            .default(false)
            .describe("Preserve image tags in Markdown"),
        },
        async (args) => {
          const cmdArgs = [
            "--max-bytes",
            args.maxBytes,
            "--timeout",
            args.timeout,
            "--concurrency",
            String(args.concurrency),
          ];

          if (args.selector) {
            cmdArgs.push("--selector", args.selector);
          }

          if (args.keepImages) {
            cmdArgs.push("--keep-images");
          }

          const stdin = args.urls.join("\n");

          try {
            const events = await execCommand(webGetCmd, cmdArgs, stdin);
            const formatted = formatDocumentEvents(events);

            return {
              content: [
                {
                  type: "text" as const,
                  text: formatted,
                },
              ],
            };
          } catch (err) {
            return {
              content: [
                {
                  type: "text" as const,
                  text: `Error running web-get: ${err}`,
                },
              ],
              isError: true,
            };
          }
        }
      ),
    ],
  });
}

/**
 * Default export for convenience
 */
export default createAgentToolsServer;
