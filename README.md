# kagi-mcp

An MCP (Model Context Protocol) server for the [Kagi Search API](https://kagi.com/api), written in Rust.

## Features

- Search: Search the web with no rate limits and no funky scraping.
- Extract: Extract URL's into clean Markdown.
- Caching: Automatic local caching of search and extract results with configurable size and TTL.
- MCP Featureful: Supports MCP features like cancellation and notifications.
- Robust: Automatic retries, automatic truncation of long responses, and graceful error handling.

## Agent Skill

A complementary agent skill is included at [`.agents/skills/kagi/SKILL.md`](.agents/skills/kagi/SKILL.md), designed to help AI agents use this MCP server efficiently. It teaches cost-aware search behavior (search budgeting, serialized queries, batched extraction) and documents all Kagi search operators and extract failure modes. Install it by copying the `.agents/skills/kagi/` directory into your project's `.agents/skills/` or `~/.config/opencode/skills/`.

## Installation

Linux and Windows binaries can be downloaded from [releases](https://github.com/Silvenga/kagi-mcp/releases).

```bash
curl -L https://github.com/Silvenga/kagi-mcp/releases/latest/download/kagi-mcp -o kagi-mcp
install kagi-mcp ~/.local/bin/
rm kagi-mcp
```

```pwsh
Invoke-WebRequest `
    -Uri https://github.com/Silvenga/kagi-mcp/releases/latest/download/kagi-mcp.exe `
    -UseBasicParsing `
    -OutFile kagi-mcp.exe
# And add to your PATH.
```

Linux container images are also available (defaults to running in HTTP SSE mode):

```bash
docker run -it \
  --rm \
  -p 3000:3000 \
  ghcr.io/silvenga/kagi-mcp:latest
```

And building from source is always possible:

```bash
cargo install --git https://github.com/Silvenga/kagi-mcp.git
```

## Usage

A [Kagi API key](https://kagi.com/api) is required. Configuration can be done via environment variables or CLI flags.

```bash
export KAGI_API_KEY="your-api-key"
kagi-mcp

# Or

kagi-mcp --api-key "your-api-key"
```

But you likely want to configure it in your MCP client:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "kagi": {
      "type": "local",
      "command": [
        "kagi-mcp"
      ],
      "environment": {
        "KAGI_API_KEY": "your-api-key"
      }
    }
  }
}
```

## Options

| Flag / Env Var                                 | Description                                            | Default                |
|------------------------------------------------|--------------------------------------------------------|------------------------|
| `--api-key` / `KAGI_API_KEY`                   | Kagi API key                                           | *required*             |
| `--base-url` / `KAGI_BASE_URL`                 | Kagi API base URL                                      | `https://kagi.com/api` |
| `--search-timeout` / `KAGI_SEARCH_TIMEOUT`     | Search API request timeout (seconds)                   | `4`                    |
| `--extract-timeout` / `KAGI_EXTRACT_TIMEOUT`   | Extract API request timeout (seconds)                  | `10`                   |
| `--client-timeout` / `KAGI_CLIENT_TIMEOUT`     | Client-side HTTP timeout (seconds)                     | `12`                   |
| `--retries` / `KAGI_RETRIES`                   | Number of retries for transient failures               | `3`                    |
| `--limit` / `KAGI_LIMIT`                       | Default result limit for search                        | `10`                   |
| `--safe-search` / `KAGI_SAFE_SEARCH`           | Enable safe search                                     | `true`                 |
| `--region` / `KAGI_REGION`                     | Default region filter (ISO 3166-1 alpha-2)             | *none*                 |
| `--cache-dir` / `KAGI_CACHE_DIR`               | Directory for the local response cache                 | *See below*            |
| `--cache-size-gb` / `KAGI_CACHE_SIZE_GB`       | Maximum cache size in gigabytes                        | `5.0`                  |
| `--cache-ttl-days` / `KAGI_CACHE_TTL_DAYS`     | Cache entry TTL in days                                | `7`                    |
| `--transport` / `KAGI_TRANSPORT`               | Transport mode: `stdio` or `streamable-http`           | `stdio`                |
| `--bind` / `KAGI_BIND`                         | Bind address for HTTP transport                        | `127.0.0.1:3000`       |
| `--fallback-message` / `KAGI_FALLBACK_MESSAGE` | Per-domain fallback message (format: `domain=message`) | *none*                 |
| `--fallback-always` / `KAGI_FALLBACK_ALWAYS`   | Always-block domains (skip extraction)                 | *none*                 |

The default cache directory depends on the platform:

- **Linux**: `$XDG_CACHE_HOME/kagi-mcp` or `~/.cache/kagi-mcp`
- **macOS**: `~/Library/Caches/kagi-mcp`
- **Windows**: `%LOCALAPPDATA%\kagi-mcp`

Override with `--cache-dir` or `KAGI_CACHE_DIR`.

## Tools

### `search`

Search the web using Kagi. Returns structured Markdown results optimized for LLMs.

**Parameters:**

- `query` (required) - Search query. Supports advanced operators: `site:`, `"exact phrases"`, `-negation`, etc.
- `workflow` - Result type: `search`, `images`, `videos`, `news`, `podcasts`
- `after`, `before` - Date filters (YYYY-MM-DD)
- `output_format` - `markdown` (default) or `json`
- `limit_per_domain` - Max results per domain group. When set, the server over-fetches from Kagi and deduplicates using
  Kagi's grouping key (with eTLD+1 domain fallback). Useful to avoid same-domain clutter. Must be >= 1.
- `cache` - Whether to use cached results. Default: `true`.

### `extract`

Extract clean Markdown content from URLs.

**Parameters:**

- `pages` (required) - Array of 1-10 HTTPS URLs
- `output_format` - `markdown` (default) or `json`
- `cache` - Whether to use cached results. Default: `true`.

## Additional Metadata in Markdown Output

When using the default Markdown output, the server enriches results with additional metadata to help LLMs interpret the
content:

Search results may include:

- Paywall indicator - Flags results that are behind a paywall.
- AI content labels - Marks results as "generated" or "possibly AI-generated" when detected.
- Language - Non-English results include a language code (e.g., `ja`, `fr`).
- Duration - For video and podcast results, the runtime is included.
- Image dimensions - Width and height are provided for image results.
- Related questions - Suggested follow-up questions with links.
- Direct answers - Inline answer snippets.

Extract results include per-page Markdown content with explicit error messages for any URLs that could not be extracted.

## Fallback Messages

When extraction fails for certain domains (e.g., rate-limited sites), you can configure per-domain fallback messages to
return custom text to the LLM instead of empty or failed content.

### Post-Extract Fallback

Triggered when the Kagi API returns empty content (`markdown: null`, empty string, or whitespace-only) for a matched
domain. The configured message is substituted into the result.

```bash
kagi-mcp --fallback-message github.com="Use github-mcp instead"
```

### Pre-Extract Skip (Always Block)

Skips the Kagi API call entirely for matched domains and returns the fallback message immediately. Useful for domains
that should never be extracted.

```bash
kagi-mcp --fallback-always github.com --fallback-message github.com="Use github-mcp instead"
```

### Environment Variables

Both options support comma-separated values via environment variables:

```bash
export KAGI_FALLBACK_MESSAGE="github.com=Use github-mcp,reddit.com=Use reddit search"
export KAGI_FALLBACK_ALWAYS="github.com"
kagi-mcp
```

### Domain Matching

Domains are matched using eTLD+1 (registrable domain) extraction. Subdomains are automatically resolved to their
registrable domain (e.g., `www.github.com` matches `github.com`). Matching is case-insensitive.

## Known Issues

- The Kagi Extract API uses a cumulative timeout, not per-page. If the cumulative timeout is exceeded during a
  multipage extraction, a blank result may be returned. This has been reported to Kagi.
