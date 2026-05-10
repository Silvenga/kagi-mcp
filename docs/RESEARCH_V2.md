# kagi-mcp - Unified Research & Specification

> This document consolidates the original project specification, API research, MCP SDK analysis, best practices, tool schemas, and all architectural decisions into a single source of truth for implementers.

---

## 1. Project Specification

### 1.1 Context

Kagi is a paid search engine. Kagi recently (5/8/2026) released the v1 of their API in limited beta preview. Search requests are priced at $12/k for search requests and $4/k for extract requests.

### 1.2 Goals

This project is an MCP server for the Kagi API, optimized for agent use, written in Rust.

- Create an `AGENTS.md` file for agent continuity.
- Commit after each change. Origin: `git@github.com:Silvenga/kagi-mcp.git`.
- Use async when possible via tokio.
- Create MCP unit tests on the MCP side with a mock kagi-api client.
- Create MCP integration tests to test calling the MCP server from an MCP client (with mocked kagi-api).
- Create API unit tests using wiremock.
- Use logging via tracing. Log the `meta.trace` string for each Kagi API request at trace level. Collect timing via logs for each request.
- Handle API errors gracefully.

### 1.3 Workspace Crates

This is a Cargo workspace with two crates:

**`kagi-api`** - A library, generated from the OpenAPI spec and a thin wrapper to make usage easier.
- Use builder pattern to create the client, setting reasonable defaults.
- Accept a Kagi API key.
- The base URL should be customizable, e.g. `https://kagi.com/api`.
- Set a User-Agent header, defaulting to `kagi-api/<crate version> (github.com/Silvenga/kagi-mcp)`.

**`kagi-mcp`** - The MCP server, running in STDIO mode.
- Use the official Rust SDK for MCP (`rmcp` and `rmcp-macros`).
- Support MCP cancellation.
- Exposes two tools - `search` and `extract` - with rich and efficient metadata for agent consumption.
- Support retries.
- Configured with either CLI flags or env vars, using `clap` for both.
- Research MCP to determine what rich MCP support would look like (e.g., notification during progress).

### 1.4 Kagi API

Kagi v1 API docs: `https://kagi.redocly.app/openapi`. OpenAPI spec: `https://kagi.redocly.app/_spec/openapi.yaml?download`.

Two APIs: Search API and Extract API.

### 1.5 Limits

- Do not support `lens`/`lens_id` yet.
- Format should be JSON for now.
- Extract via search should not be used by the MCP server.
- Do not support personalizations yet.

---

## 2. Kagi API v1 (Preview)

### 2.1 Endpoints & Base URL

| Endpoint | Method | Base URL |
|----------|--------|----------|
| `/search` | `POST` | `https://kagi.com/api/v1` |
| `/extract` | `POST` | `https://kagi.com/api/v1` |

Authentication is **Bearer token** (`Authorization: Bearer <key>`).

### 2.2 Search API (`POST /search`)

**Required parameter**
- `query` (`string`) - the search query.

**Optional parameters**
- `workflow` (`string`, default `search`) - enum: `search`, `images`, `videos`, `news`, `podcasts`.
- `format` (`string`, default `json`) - enum: `json`, `markdown`. The spec says JSON only for now.
- `timeout` (`number`, min `0.5`, max `4`) - seconds to allow for collecting results.
- `page` (`integer`, min `1`, max `10`) - pagination.
- `limit` (`integer`, min `1`, max `1024`) - max results.
- `safe_search` (`boolean`, default `true`).
- `filters` (`object`) - `region` (ISO-3166-1 Alpha-2), `after` (date), `before` (date).
- `lens_id` (`string`) - **out of scope**.
- `lens` (`object`) - inline lens. **Out of scope**.
- `personalizations` (`object`) - **out of scope**.
- `extract` (`object`) - nested extraction config (`count`, `timeout`) that triggers page extraction *from search results*. **Out of scope** because it incurs extra Extract API cost and is distinct from the standalone `/extract` endpoint.

**Response schema (`200`)**
- `meta` (`object`) - `trace` (string), `node` (string), `ms` (integer). Explicitly documented as unstable/debug-oriented.
- `data` (`object`) - contains many typed arrays:
  - `search`, `image`, `video`, `podcast`, `podcast_creator`, `news`
  - `adjacent_question`, `direct_answer`, `interesting_news`, `interesting_finds`
  - `infobox`, `code`, `package_tracking`, `public_records`, `weather`
  - `related_search`, `listicle`, `web_archive`

Each item in these arrays is a `searchResult`:
- `url` (required)
- `title` (required)
- `snippet` (string, optional)
- `time` (ISO 8601 string, optional)
- `image` (`object` with `url`, `height`, `width`, optional)
- `props` (`object`, arbitrary additional metadata)

**Error responses**
- `400` - invalid request parameters
- `401` - unauthorized
- `403` - forbidden (IP not authorized)
- `429` - rate limited
- `500` - internal server error

All errors use the same envelope:
```json
{
  "meta": { ... },
  "data": null,
  "error": [
    { "code": "string", "url": "string", "message": "string | null", "location": "string | null" }
  ]
}
```

### 2.3 Extract API (`POST /extract`)

**Required parameter**
- `pages` (`array`) - 1-10 items, each with `url` (required, HTTPS URI).

**Optional parameters**
- `timeout` (`number`, float) - seconds.
- `format` (`string`, default `json`) - enum: `json`, `markdown`.

**Response schema (`200`)**
- `meta` - same as Search.
- `data` (`array`) - items with `url` and `markdown` (nullable string).
- `errors` (`array`, optional) - per-URL failure details.

Error HTTP statuses are identical to Search (`400`, `401`, `403`, `429`, `500`).

### 2.4 Pricing & Limits

- Search: `$12 / 1,000 requests`
- Extract: `$4 / 1,000 requests`
- Invoiced every 30 days or when usage reaches `$100`.
- Rate limits are not numerically specified in public docs; `429` is the signal.

### 2.5 Official Rust Client

Kagi publishes an official Rust client generated from the OpenAPI spec:
- Repo: `https://github.com/kagisearch/kagi-openapi-rust`
- Version string in `Cargo.toml`: `"1 (Preiew).0.0"` (note the typo).
- Dependencies: `serde`, `serde_json`, `serde_with`, `serde_repr`, `url`, `chrono`, `reqwest`.
- Features: `native-tls` (default) and `rustls`.

This project will generate a fresh client instead of depending on the official one.

---

## 3. Rust MCP SDK (`rmcp`)

### 3.1 Crates & Versions

- `rmcp` - core protocol implementation (current: `0.16.0`).
- `rmcp-macros` - procedural macros for tools, prompts, resources.

### 3.2 Server Architecture (stdio)

```rust
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());
let service = MyServer.serve(transport).await?;
service.waiting().await?; // blocks until shutdown
```

- `ServerHandler` trait provides lifecycle hooks.
- `#[tool_router]` + `#[tool]` macros auto-implement tool routing.
- `#[tool_handler]` macro auto-implements `ServerHandler` with custom metadata.

### 3.3 Tool Definition Pattern

Tool params MUST derive `schemars::JsonSchema` so the SDK auto-generates input JSON Schema for `list_tools`.

```rust
use rmcp::{tool, tool_router, schemars, ServiceExt, transport::stdio};
use serde::Deserialize;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    query: String,
}

#[derive(Clone)]
struct KagiMcpServer;

#[tool_router(server_handler)]
impl KagiMcpServer {
    #[tool(description = "Search the web using Kagi")]
    async fn search(
        &self,
        Parameters(SearchParams { query }): Parameters,
    ) -> String {
        // ...
    }
}
```

Return types can be `String`, `CallToolResult`, or `Result<CallToolResult, McpError>`.

### 3.4 Cancellation

MCP cancellation is **cooperative** in `rmcp`. Every async handler receives a `RequestContext<RoleServer>` containing a `CancellationToken` (from `tokio_util`). Handlers should use `tokio::select!` to abort in-flight HTTP requests:

```rust
async fn search(&self, ctx: RequestContext<RoleServer>, ...) -> Result<...> {
    let fut = self.api_client.search(...);
    tokio::select! {
        _ = ctx.ct.cancelled() => {
            Err(McpError::internal_error("Cancelled", None))
        }
        res = fut => {
            // process result
        }
    }
}
```

### 3.5 Progress Notifications

```rust
ctx.peer.notify_progress(ProgressNotificationParam {
    progress_token: ProgressToken(NumberOrString::Number(1)),
    progress: 50.0,
    total: Some(100.0),
    message: Some("Fetching results...".into()),
}).await?;
```

`progress_token` is automatically assigned per request and available in `RequestContext.meta.progress_token()`.

### 3.6 Error Handling in MCP

- `McpError::internal_error(message, data)` - generic server error.
- `McpError::invalid_request(message, data)` - bad tool arguments.
- `McpError::Cancelled` - protocol-level cancellation.

There is no native "retryable" or "rate limited" error code in the MCP spec. Kagi HTTP errors are surfaced as `internal_error` per tool call.

---

## 4. MCP Best Practices

### 4.1 Tool Descriptions

- State *what* the tool does, not *when* to use it.
- Be explicit about capabilities and cost.

### 4.2 Input Schema Design

- Use `schemars` descriptions on every field.
- Required fields should be truly required; optional fields should have sensible defaults.
- Add `additionalProperties: false` and `examples`.

### 4.3 Output Formatting

- LLMs consume Markdown most naturally.
- Return concise summaries with key fields (title, url, snippet).
- Offer raw JSON via optional parameter for programmatic access.

---

## 5. MCP Tool Specification

### 5.1 Overview

| Tool Name | Purpose | Kagi API Endpoint |
|-----------|---------|-------------------|
| `search` | Search the web using Kagi | `POST /v1/search` |
| `extract` | Extract clean Markdown from URLs | `POST /v1/extract` |

Both tools return `text` content. Default output format is **structured Markdown**. Callers can request raw JSON via `output_format`.

### 5.2 Search Tool

**Metadata**
- **Name:** `search`
- **Description:** Search the web using Kagi. Returns structured Markdown results optimized for LLM consumption. Supports web pages, images, videos, news, and podcasts. Results are billed at $12 per 1,000 requests.

**Input Schema (JSON)**
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
      "description": "Type of results to return."
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

**Parameter Notes**
- `query` is the only required field.
- `workflow` maps directly to Kagi's `workflow` parameter.
- `after`, `before` map into Kagi's nested `filters` object.
- `output_format` is synthetic (handled by MCP server, not forwarded to Kagi).
- `page`, `limit`, `safe_search`, and `region` are configured at the server level. Defaults: `limit=10`, `safe_search=true`, `region=none`.

**Output Format - Markdown (Default)**

Converts Kagi JSON response into structured Markdown. Categories with no results are omitted.

**Empty results:**
```markdown
No results found.
```

**Templates:**

General (`search`, `news`, `interesting_news`, `interesting_finds`, `code`, `public_records`, `listicle`, `web_archive`):
```markdown
## Web Results

1. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}
```
`snippet` and `time` may be absent; omit the corresponding line if so. `title` and `url` are always present.

Images (`image`):
```markdown
## Images

1. **[Title](URL)**
   - Image: {image.url} ({image.width}x{image.height})
```

Videos (`video`):
```markdown
## Videos

1. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}
```

Podcasts (`podcast`):
```markdown
## Podcasts

1. **[Title](URL)**
   - Snippet: {snippet}
   - Published: {time}
```

Adjacent Questions (`adjacent_question`):
```markdown
## Related Questions

1. **{props.question}**
   - [Answer](URL): {snippet}
```

Direct Answer (`direct_answer`):
```markdown
## Direct Answer

{snippet}
```

Infobox (`infobox`):
```markdown
## Infobox

**[Title](URL)**

{snippet}

{props.infobox items rendered as key-value pairs}
```

Related Searches (`related_search`):
```markdown
## Related Searches

- {title}
- {title}
```

Weather (`weather`):
```markdown
## Weather

{snippet}
```

Package Tracking (`package_tracking`):
```markdown
## Package Tracking

- [Tracking Link](URL)
```

**Output Format - JSON**

Raw Kagi API JSON response as a pretty-printed `text` block.

### 5.3 Extract Tool

**Metadata**
- **Name:** `extract`
- **Description:** Extract clean Markdown content from one or more web pages. Returns extracted Markdown for each URL. Billed at $4 per 1,000 requests (up to 10 URLs per request).

**Input Schema (JSON)**
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
      "description": "Timeout in seconds for each page extraction."
    },
    "output_format": {
      "type": "string",
      "enum": ["markdown", "json"],
      "default": "markdown",
      "description": "Output format. 'markdown' returns extracted Markdown directly. 'json' returns the raw Kagi API JSON response."
    }
  },
  "required": ["pages"],
  "additionalProperties": false
}
```

**Parameter Notes**
- `pages` must contain valid HTTPS URLs.
- `timeout` is forwarded to Kagi as-is.
- `output_format` is handled by the MCP server.
- **Security:** The server must validate that URLs are not private IP ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `127.0.0.0/8`, `169.254.0.0/16`), `localhost`, or link-local addresses.

**Output Format - Markdown (Default)**
```markdown
## Extracted Content

### {url}

{markdown content}

---

### {url}

{markdown content}
```

Failed extraction:
```markdown
### {url}

**Extraction failed:** {error message}
```

**Output Format - JSON**

Raw Kagi API JSON response (including `meta`, `data`, `errors`) as pretty-printed JSON.

### 5.4 Common Behaviors

**Progress Notifications**

Search:
1. At start: `Searching "{query}"`
2. After response: `Query completed.`

Extract:
1. At start: `Extracting {N} pages...`
2. After each URL: `Extracted {i}/{N} pages.`
3. After all: `Extraction completed.`

**Cancellation**
- Abort in-flight Kagi HTTP request by dropping the `reqwest` future (`tokio::select!`).
- Partial results are discarded.
- Return MCP `Cancelled` error (not `internal_error`).

**Error Handling**

All Kagi HTTP errors surfaced as `McpError::internal_error` per tool call:

| Kagi HTTP | MCP Error Message | Notes |
|-----------|-------------------|-------|
| `400` | `Invalid request: {kagi_message}` | Bad parameters |
| `401` | `Unauthorized: Invalid Kagi API key` | Per-tool |
| `403` | `Forbidden: IP address not authorized` | Per-tool |
| `429` | `Rate limited. Please retry later.` | Suggests retry |
| `500` | `Kagi API error. Please retry later.` | Suggests retry |
| Network | `Request failed: {error}` | Handled by `reqwest-retry` |

**Retry Policy**
- Use `reqwest-retry` with exponential backoff.
- Retry on connection errors, `429`, `500`.
- No jitter.
- Default: 3 retries, delays 1s, 2s, 4s.
- Configurable at server level.

**Response Size Guard**
- Default max: 256KB of Markdown text per tool call.
- Truncated content ends with: `\n\n_(Content truncated. Total size: {N} bytes)_`.

**Image Content**
- No MCP `image` content type. Image URLs included as Markdown links.

---

## 6. Architecture & Decisions

### 6.1 OpenAPI Client Generation

**Decision:** Generate a new client inside the workspace using `utoipa`.

**Rationale:** The official `kagi-openapi-rust` client has a typo in its version string (`"1 (Preiew).0.0"`) and unclear licensing. Generating fresh gives full control over the API surface and error types. The OpenAPI spec will be vendored in `docs/openapi.yaml`.

### 6.2 Crate Versioning

**Decision:** Both crates versioned together via `release-please` using Conventional Commits.

**Rationale:** `kagi-api` is an internal workspace crate. It may be reused for a future CLI but is not published independently. Version read from `Cargo.toml`.

### 6.3 Distribution

**Decision:** `cargo install --git https://github.com/Silvenga/kagi-mcp.git`

No `npx`-style, Docker, or crates.io publishing specified yet.

### 6.4 CI/CD

**Decision:** No CI at this time.

---

## 7. Configuration

### 7.1 Environment Variables & CLI Flags

| Flag | Env Var | Description | Default |
|------|---------|-------------|---------|
| `--api-key` | `KAGI_API_KEY` | Kagi API key | *required* |
| `--base-url` | `KAGI_BASE_URL` | Kagi API base URL | `https://kagi.com/api` |
| `--kagi-timeout` | `KAGI_TIMEOUT` | Kagi API request timeout (s) | `4` |
| `--client-timeout` | `KAGI_CLIENT_TIMEOUT` | Client-side HTTP timeout (s) | `10` |
| `--retries` | `KAGI_RETRIES` | Number of retries | `3` |
| `--limit` | `KAGI_LIMIT` | Default result limit | `10` |
| `--safe-search` | `KAGI_SAFE_SEARCH` | Enable safe search | `true` |
| `--region` | `KAGI_REGION` | Default region filter | *none* |

**Precedence:** CLI flags override env vars (via `clap`'s built-in `env` feature).

### 7.2 Timeout Defaults

- Client-side HTTP timeout: **10 seconds** (server-level config).
- Kagi API `timeout` parameter: **4 seconds** default (forwarded to Kagi).

### 7.3 Retry Policy

- Use `reqwest-retry`.
- Retry on: connection errors, `429`, `500`.
- No jitter.
- Default: 3 retries with exponential backoff starting at 1 second (delays: 1s, 2s, 4s).

### 7.4 Server-Level Search Params

`page`, `limit`, `safe_search`, and `region` are not exposed as tool parameters. They are configured at the server level.
- `limit`: default `10`
- `safe_search`: default `true`
- `region`: default none

---

## 8. Testing Strategy

### 8.1 MCP Unit Tests

- Test MCP tool handlers with a mock kagi-api client.
- A trait-based interface for the Kagi API client will be defined for easy mocking.

### 8.2 MCP Integration Tests

- Spawn the `kagi-mcp` binary as a subprocess and communicate over stdio.
- If `rmcp` provides testing utilities, use them.

### 8.3 API Unit Tests (Wiremock)

- Focus on happy path + each error code (`400`, `401`, `403`, `429`, `500`).
- Validate that expected request payloads match the Kagi API schema.

---

## 9. Project Structure

```
kagi-mcp/
|-- Cargo.toml              # Workspace root
|-- README.md
|-- AGENTS.md               # Project-level agent continuity
|-- docs/
|   |-- RESEARCH_V2.md      # This unified document
|   |-- MCP_TOOLS_SPEC.md   # Detailed MCP tool schemas
|   |-- openapi.yaml        # Vendored Kagi OpenAPI spec
|-- kagi-api/
|   |-- Cargo.toml
|   |-- src/
|       |-- lib.rs          # Generated client + thin wrapper
|-- kagi-mcp/
    |-- Cargo.toml
    |-- src/
        |-- main.rs         # MCP server executable
```

### 9.1 Server Metadata

- **Name:** `Kagi`
- **Version:** from `Cargo.toml` (`CARGO_PKG_VERSION`)
- **Instructions:** Reasonable default describing the server's purpose
- **Protocol:** `rmcp` built-in default

### 9.2 Logging

- Use the `tracing` crate and macros.
- Log `meta.trace` at trace level.
- Collect timing via `tracing` spans.
- Using the same tracing infrastructure for MCP `notify_logging_message` is **not a priority**.
