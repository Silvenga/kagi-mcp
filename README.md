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
| `--overfetch-multiplier` / `KAGI_OVERFETCH_MULTIPLIER` | Over-fetch multiplier when `limit_per_domain` is set | `5` |
| `--overfetch-max` / `KAGI_OVERFETCH_MAX` | Absolute cap on over-fetch request size | `50` |

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
- `limit_per_domain` - Max results per domain group. When set, the server over-fetches from Kagi and deduplicates using Kagi's grouping key (with eTLD+1 domain fallback). Useful to avoid same-domain clutter. Must be >= 1.

### `extract`

Extract clean Markdown content from URLs.

**Parameters:**
- `pages` (required) - Array of 1-10 HTTPS URLs
- `timeout` - Per-page extraction timeout
- `output_format` - `markdown` (default) or `json`

## Agent-Optimized Output

This server formats Kagi responses for efficient LLM consumption. The following details explain how the output differs from raw Kagi JSON.

- **Clean Markdown** — HTML entities decoded (`&#39;` → `'`), title whitespace normalized, snippet ellipsis runs collapsed (`... ... ...` → `...`), ISO timestamps trimmed to `YYYY-MM-DD`.
- **Distinct section headers** — 8 separate result categories now use distinct Markdown headers (Web Results, News, Interesting News, Interesting Finds, Code Results, Public Records, Listicles, Web Archive) — previously all collapsed under "Web Results" causing ambiguous numbering.
- **Domain grouping via `limit_per_domain`** — agents can request at most N results per domain (using Kagi's `props.group_id` with eTLD+1 fallback). The server over-fetches from Kagi to compensate, preserving the user's final result count. Configurable via `KAGI_OVERFETCH_MULTIPLIER` and `KAGI_OVERFETCH_MAX`.
- **Actionable truncation notice** — when output exceeds the byte limit, the trailing notice suggests narrowing the query, reducing `limit`, or switching to `output_format="json"`.
- **Expanded tool description** — the MCP `search` tool description now lists supported operators, workflow values, date filter format, and the new `limit_per_domain` parameter — so agents can select and parameterize the tool correctly without a separate docs lookup.


