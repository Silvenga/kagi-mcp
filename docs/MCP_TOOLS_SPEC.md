# Kagi MCP Server — Tool Specification

> This document defines the MCP tool schemas (input parameters and output format) for the
> `kagi-mcp` server. It serves as the contract for future implementers and consumers.

---

## Overview

The server exposes two tools:

| Tool Name | Purpose | Kagi API Endpoint |
|-----------|---------|-------------------|
| `search` | Search the web using Kagi | `POST /v1/search` |
| `extract` | Extract clean Markdown from URLs | `POST /v1/extract` |

Both tools return `text` content. The default output format is **structured Markdown** for easy LLM consumption. Callers can request raw JSON via an optional `output_format` parameter.

---

## 1. Search Tool

### 1.1 Tool Metadata

- **Name:** `search`
- **Description:** Search the web using Kagi. Returns structured Markdown results optimized for LLM consumption. Supports web pages, images, videos, news, and podcasts. Results are billed at $12 per 1,000 requests.

### 1.2 Input Schema (JSON Schema)

```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "minLength": 1,
      "maxLength": 2048,
      "description": "The search query to execute."
    },
    "workflow": {
      "type": "string",
      "enum": ["search", "images", "videos", "news", "podcasts"],
      "default": "search",
      "description": "Type of results to return. 'search' for web pages, 'images' for images, 'videos' for videos, 'news' for news articles, 'podcasts' for podcasts."
    },
    "after": {
      "type": "string",
      "format": "date",
      "description": "Filter for results published or updated after this date (YYYY-MM-DD)."
    },
    "before": {
      "type": "string",
      "format": "date",
      "description": "Filter for results published or updated before this date (YYYY-MM-DD)."
    },
    "output_format": {
      "type": "string",
      "enum": ["markdown", "json"],
      "default": "markdown",
      "description": "Output format. 'markdown' returns a human-readable Markdown summary. 'json' returns the raw Kagi API JSON response."
    }
  },
  "required": ["query"],
  "additionalProperties": false,
  "examples": [
    { "query": "rust async programming", "workflow": "search" },
    { "query": "aurora borealis", "workflow": "images" },
    { "query": "\"exact phrase match\"", "workflow": "search" },
    { "query": "site:github.com rust mcp server", "workflow": "search" },
    { "query": "kagi search -ads", "workflow": "search" }
  ]
}
```

### 1.3 Parameter Notes

- `query` is the only required field.
- `workflow` maps directly to Kagi's `workflow` parameter.
- `after`, `before` are convenience fields that map into Kagi's nested `filters` object.
- `output_format` is a synthetic parameter handled by the MCP server, not forwarded to Kagi.
- `page`, `limit`, `safe_search`, and `region` are configured at the server level and are not exposed as tool parameters. Defaults: `limit=10`, `safe_search=true`, `region=none`.

### 1.4 Output Format — Markdown (Default)

When `output_format` is `"markdown"`, the server converts the Kagi JSON response into structured Markdown. The output is organized by result category. Categories with no results are omitted.

**Empty results:** When no results are returned for any category, the output is:
```markdown
No results found.
```

#### General Result Template (used for `search`, `news`, `interesting_news`, `interesting_finds`, `code`, `public_records`, `listicle`, `web_archive`)

```markdown
## Web Results

1. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}

2. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}
```

**Field nullability:** `snippet` and `time` may be absent from Kagi results. If absent, the corresponding line is omitted from the template. `title` and `url` are always present per Kagi's schema.

#### Image Results (`image`)

```markdown
## Images

1. **[Title](URL)**
   - Image: {image.url} ({image.width}x{image.height})
```

#### Video Results (`video`)

```markdown
## Videos

1. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}
```

#### Podcast Results (`podcast`)

```markdown
## Podcasts

1. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}
```

#### Adjacent Questions (`adjacent_question`)

```markdown
## Related Questions

1. **{props.question}**
   - [Answer](URL): {snippet}
```

#### Direct Answer (`direct_answer`)

```markdown
## Direct Answer

{snippet}
```

#### Infobox (`infobox`)

```markdown
## Infobox

**[Title](URL)**

{snippet}

{props.infobox items rendered as key-value pairs}
```

#### Related Searches (`related_search`)

```markdown
## Related Searches

- {title}
- {title}
```

#### Weather (`weather`)

```markdown
## Weather

{snippet}
```

#### Package Tracking (`package_tracking`)

```markdown
## Package Tracking

- [Tracking Link](URL)
```

### 1.5 Output Format — JSON

When `output_format` is `"json"`, the raw Kagi API JSON response is returned as a single `text` block (formatted with `serde_json::to_string_pretty`). This is useful when the caller needs programmatic access to all fields, including `props`.

### 1.6 Example Output (Markdown)

```markdown
## Web Results

1. **[Steve Jobs - Wikipedia](https://en.wikipedia.org/wiki/Steve_Jobs)**
   - Snippet: Steven Paul Jobs (February 24, 1955 – October 5, 2011) was an American businessman, inventor, and investor...
   - Published: 2024-11-29T03:54:26Z

2. **[iPhone Turns 10: Watch Steve Jobs Introduce Apple's 'Revolutionary...](https://www.billboard.com/pro/iphone-turns-10-steve-jobs-introduction-apple-ads/)**
   - Snippet: iPhone Turns 10: Watch Steve Jobs Introduce Apple's Revolutionary...
   - Published: 2017-01-09T14:49:00Z

## Videos

1. **[Steve Jobs' 2005 Stanford Commencement Address](https://www.youtube.com/watch?v=UF8uR6Z6KLc)**
   - Snippet: Steve Jobs' 2005 Stanford Commencement Address · Comments. 27K...
   - Published: 2024-11-29T03:54:26Z

## Related Questions

1. **What is Steve Jobs' 10 minute rule?**
   - [Answer](https://www.ceotodaymagazine.com/2025/02/steve-jobs-10-minute-rule-you-can-implement/): Steve Jobs was a visionary, innovator, and problem-solver...
```

---

## 2. Extract Tool

### 2.1 Tool Metadata

- **Name:** `extract`
- **Description:** Extract clean Markdown content from one or more web pages. Returns extracted Markdown for each URL. Billed at $4 per 1,000 requests (up to 10 URLs per request).

### 2.2 Input Schema (JSON Schema)

```json
{
  "type": "object",
  "properties": {
    "pages": {
      "type": "array",
      "items": {
        "type": "string",
        "format": "uri",
        "pattern": "^https://"
      },
      "minItems": 1,
      "maxItems": 10,
      "description": "List of HTTPS URLs to extract content from. Maximum 10 URLs per call."
    },
    "timeout": {
      "type": "number",
      "minimum": 0.5,
      "maximum": 60,
      "description": "Timeout in seconds for each page extraction. If omitted, uses the server default."
    },
    "output_format": {
      "type": "string",
      "enum": ["markdown", "json"],
      "default": "markdown",
      "description": "Output format. 'markdown' returns the extracted Markdown content directly. 'json' returns the raw Kagi API JSON response."
    }
  },
  "required": ["pages"],
  "additionalProperties": false
}
```

### 2.3 Parameter Notes

- `pages` must contain valid HTTPS URLs. The MCP server validates this before sending to Kagi.
- `timeout` is forwarded to Kagi as-is.
- `output_format` is handled by the MCP server, not forwarded to Kagi.
- **Security:** The server must validate that extracted URLs are not private IP ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`, `169.254.0.0/16`), `localhost`, or link-local addresses. This prevents SSRF against internal services.

### 2.4 Output Format — Markdown (Default)

When `output_format` is `"markdown"`, each extracted page is rendered as a Markdown section:

```markdown
## Extracted Content

### {url}

{markdown content}

---

### {url}

{markdown content}
```

If a page fails to extract, it is rendered as:

```markdown
### {url}

**Extraction failed:** {error message}
```

### 2.5 Output Format — JSON

When `output_format` is `"json"`, the raw Kagi API JSON response (including `meta`, `data`, and `errors`) is returned as a pretty-printed JSON text block.

### 2.6 Example Output (Markdown)

```markdown
## Extracted Content

### https://example.com/article1

# Article Title

This is the extracted content in clean Markdown...

---

### https://example.com/article2

## Another Title

More extracted content here...
```

---

## 3. Common Behaviors

### 3.1 Progress Notifications

The server sends progress notifications per tool call:

**Search:**
1. **At invocation start:** `Searching "{query}"`
2. **After response received:** `Query completed.`

**Extract:**
1. **At invocation start:** `Extracting {N} pages...`
2. **After each URL completes:** `Extracted {i}/{N} pages.`
3. **After all URLs complete:** `Extraction completed.`

These use the automatic `ProgressToken` provided by `rmcp` via `RequestContext.meta.progress_token()`.

### 3.2 Cancellation

MCP cancellation is cooperative. On cancellation:
- The in-flight Kagi HTTP request is aborted by dropping the `reqwest` future (cancel-on-drop via `tokio::select!`).
- Partial results are **discarded**.
- The handler returns an MCP `Cancelled` error (protocol-level cancellation), not `internal_error`.

### 3.3 Error Handling

All Kagi API HTTP errors are surfaced as MCP `ErrorData::internal_error` per tool call.

**Retry policy:** The server uses `reqwest-retry` with exponential backoff. Retries are triggered on connection errors, `429` (rate limited), and `500` (internal server error). Default: 3 retries with delays of 1s, 2s, 4s. No jitter. Retries are configurable at the server level.

| Kagi HTTP Status | MCP Error Message | Notes |
|------------------|-------------------|-------|
| `400` | `Invalid request: {kagi_message}` | Bad parameters forwarded from Kagi |
| `401` | `Unauthorized: Invalid Kagi API key` | Per-tool error |
| `403` | `Forbidden: IP address not authorized` | Per-tool error |
| `429` | `Rate limited. Please retry later.` | Suggests retry |
| `500` | `Kagi API error. Please retry later.` | Suggests retry |
| Network/Timeout | `Request failed: {error}` | Suggests retry (handled by `reqwest-retry`) |

### 3.4 Response Size Guard

Extracted content from 10 URLs could exceed MCP message size limits or LLM context windows. The server should enforce a configurable maximum response size (default: 256KB of Markdown text per tool call). Content exceeding this limit is truncated with a notice: `\n\n_(Content truncated. Total size: {N} bytes)_`.

### 3.5 Image Content

No image content is returned as MCP `image` content type. Image URLs are included as Markdown links in text output. Image handling may be added in a future revision.
