# sentry-mcp-rs

A minimal [MCP](https://modelcontextprotocol.io/) server for Sentry, written in Rust.

## Features

This MCP server provides tools to interact with Sentry's API:

- **get_issue_details** - Retrieve detailed information about a Sentry issue including metadata, tags, stacktraces, and optionally a specific event
- **get_trace_details** - Retrieve trace details including span tree and timing information for distributed tracing analysis
- **search_issue_events** - Search events within an issue using Sentry's query syntax

## Installation

```bash
cargo install --git https://github.com/utapyngo/sentry-mcp-rs.git
```

This installs the `sentry-rs` binary to `~/.cargo/bin/`.

## Configuration

Required environment variables:
- `SENTRY_AUTH_TOKEN` - Your Sentry API authentication token
- `SENTRY_HOST` - Your Sentry instance hostname (e.g., `sentry.io`)

## MCP Client Configuration

Add to your MCP client configuration:

```json
{
  "mcpServers": {
    "sentry": {
      "command": "sentry-rs",
      "env": {
        "SENTRY_AUTH_TOKEN": "your_token_here",
        "SENTRY_HOST": "sentry.io"
      }
    }
  }
}
```

## Development

Clone the repository and create a `.env` file:

```bash
git clone https://github.com/utapyngo/sentry-mcp-rs.git
cd sentry-mcp-rs
cp .env.example .env
# Edit .env with your credentials
```

Build and test with MCP Inspector:

```bash
cargo build --release
npx @modelcontextprotocol/inspector ./run.sh
```

Or configure MCP client to use the script:

```json
{
  "mcpServers": {
    "sentry": {
      "command": "/path/to/sentry-mcp-rs/run.sh"
    }
  }
}
```

## Tools

### get_issue_details

Retrieve detailed information about a specific Sentry issue.

**Parameters:**
- `issue_url` - Full Sentry issue URL (alternative to the parameters below)
- `organization_slug` - Organization slug
- `issue_id` - Issue ID

### get_trace_details

Retrieve trace details for distributed tracing analysis.

**Parameters:**
- `organization_slug` - Organization slug
- `trace_id` - 32-character hex trace ID

### search_issue_events

Search events within an issue using Sentry's query syntax.

**Parameters:**
- `organization_slug` - Organization slug
- `issue_id` - Issue ID (e.g., `PROJECT-123`)
- `query` - Optional Sentry search query
- `limit` - Maximum events to return (default: 10, max: 100)
- `sort` - Sort order: `newest` (default) or `oldest`
