# Kagi MCP Server

An MCP (Model Context Protocol) server for the [Kagi Search API](https://kagi.com/api), written in Rust.

## Features

- **Search** - Web, images, videos, news, and podcasts via Kagi's premium search engine.
- **Extract** - Clean Markdown extraction from up to 10 URLs per call.
- **Agent-optimized** - Results are formatted as structured Markdown for easy LLM consumption. Raw JSON available on request.
- **Async** - Built on Tokio for high-performance, non-blocking I/O.
- **Cancellation** - Cooperative MCP cancellation aborts in-flight Kagi requests.
- **Retries** - Automatic retries on transient failures with exponential backoff.

## Installation

```bash
cargo install --git https://github.com/Silvenga/kagi-mcp.git
```

## Configuration

The server requires a Kagi API key. Set it via environment variable or CLI flag:

```bash
# Environment variable
export KAGI_API_KEY="your-api-key"
kagi-mcp

# CLI flag
kagi-mcp --api-key "your-api-key"
```

### Options

| Flag / Env Var | Description | Default |
|----------------|-------------|---------|
| `--api-key` / `KAGI_API_KEY` | Kagi API key | *required* |
| `--base-url` / `KAGI_BASE_URL` | Kagi API base URL | `https://kagi.com/api` |
| `--kagi-timeout` / `KAGI_TIMEOUT` | Kagi API request timeout (seconds) | `4` |
| `--client-timeout` / `KAGI_CLIENT_TIMEOUT` | Client-side HTTP timeout (seconds) | `10` |
| `--retries` / `KAGI_RETRIES` | Number of retries for transient failures | `3` |
| `--limit` / `KAGI_LIMIT` | Default result limit for search | `10` |
| `--safe-search` / `KAGI_SAFE_SEARCH` | Enable safe search | `true` |
| `--region` / `KAGI_REGION` | Default region filter (ISO 3166-1 alpha-2) | *none* |

## Usage

The server runs in stdio mode and is designed to be connected to an MCP client (e.g., Claude Desktop, Cursor, or any MCP-compatible host).

```json
{
  "mcpServers": {
    "kagi": {
      "command": "kagi-mcp",
      "env": {
        "KAGI_API_KEY": "your-api-key"
      }
    }
  }
}
```

## Tools

### `search`

Search the web using Kagi. Returns structured Markdown results optimized for LLMs.

**Parameters:**
- `query` (required) - Search query. Supports advanced operators: `site:`, `"exact phrases"`, `-negation`, etc.
- `workflow` - Result type: `search`, `images`, `videos`, `news`, `podcasts`
- `after`, `before` - Date filters (YYYY-MM-DD)
- `output_format` - `markdown` (default) or `json`

### `extract`

Extract clean Markdown content from URLs.

**Parameters:**
- `pages` (required) - Array of 1-10 HTTPS URLs
- `timeout` - Per-page extraction timeout
- `output_format` - `markdown` (default) or `json`



