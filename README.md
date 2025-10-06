# agent_tools (workspace scaffold)

This is a ready-to-build Cargo workspace scaffold for two agent-first tools:

- **ast-find** — structure-aware repository search (Tree-sitter powered in the real implementation)
- **web-get** — fetch & sanitize web content into Markdown (Readability-lite in the real implementation)

This scaffold compiles out of the box and prints NDJSON "summary" events.
Use `docs/blueprint.md` for the full implementation plan (APIs, crates, code sketches).

## Quick start

```bash
# unzip and enter the repo root
cargo build
cargo run -p ast-find -- --help
cargo run -p web-get -- --help
```

## Next steps

- Replace the stubs with the Tree-sitter parsing and HTTP/Readability-lite extraction outlined in `docs/blueprint.md`.
- Keep NDJSON output, determinism (UTC/NO_COLOR), and hard limits.
