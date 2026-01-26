# sentry-mcp: A Rust-powered MCP server for Sentry

I built a fast, lightweight MCP (Model Context Protocol) server for Sentry in Rust and published it on [crates.io](https://crates.io/crates/sentry-mcp).

## What is it?

sentry-mcp lets your AI assistant (Claude, Cursor, etc.) interact directly with Sentry issues. Ask questions like "What's causing this error?" and your assistant can fetch the stacktrace, browse related events, and analyze distributed traces — all without leaving your chat.

## Why Rust?

The official [mcp-server-sentry](https://www.npmjs.com/package/@sentry/mcp-server) works, but:

| Metric | Node.js | Rust |
|--------|---------|------|
| Memory | ~80 MB | ~8 MB |
| Startup | ~500ms cold start | instant |
| Disk | node_modules + runtime | single 8 MB binary |
| Tools | 16+ tools | 3 focused tools |

Fewer tools means a smaller context window footprint and less confusion for the LLM.

## Tools

- **get_issue_details** — Full issue info: metadata, tags, stacktrace, contexts. Accepts issue ID or Sentry URL.
- **get_trace_details** — Span tree with timing for distributed tracing analysis.
- **search_issue_events** — Search events within an issue using Sentry query syntax.

## Installation

```bash
# From crates.io
cargo install sentry-mcp

# Or using mise
mise use -g github:utapyngo/sentry-mcp-rs
```

## Configuration

```json
{
  "mcpServers": {
    "sentry": {
      "command": "sentry-mcp",
      "env": {
        "SENTRY_AUTH_TOKEN": "your_token",
        "SENTRY_HOST": "sentry.io"
      }
    }
  }
}
```

## Links

- GitHub: https://github.com/utapyngo/sentry-mcp-rs
- Crates.io: https://crates.io/crates/sentry-mcp
- MCP Protocol: https://modelcontextprotocol.io/

Feedback and contributions welcome!
