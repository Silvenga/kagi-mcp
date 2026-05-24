# Kagi MCP Server - Project Workspace

## Workspace Structure

```
kagi-mcp/
├── Cargo.toml              # Workspace root
├── AGENTS.md               # This file
├── kagi-api/               # Library crate | Kagi API client
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module declarations + re-exports
│       ├── api_trait.rs    # Async trait for testability (mockall)
│       ├── builder.rs      # KagiClientBuilder with defaults
│       ├── client.rs       # HTTP client (reqwest + middleware)
│       ├── error.rs        # Domain error types (thiserror)
│       └── types/
│           ├── search_request.rs
│           ├── search_response.rs
│           ├── extract_request.rs
│           ├── extract_response.rs
│           └── error_response.rs
└── kagi-mcp/               # Binary crate | MCP server
    ├── Cargo.toml
    └── src/
        └── main.rs         # Entrypoint (rmcp server)
```

## Build & Test Commands

```bash
# Check
cargo check --workspace

# Format
cargo fmt

# Build
cargo build --workspace

# Test
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets --all-features

# Docs
cargo doc --workspace --no-deps
```

## Key Architecture Decisions

### Two-crate split (`kagi-api` + `kagi-mcp`)
- `kagi-api` is a pure library that wraps the Kagi Search and Extract HTTP APIs.
- `kagi-mcp` is the binary that embeds `kagi-api` and exposes it via the Model Context Protocol.
- Separation allows `kagi-api` to be reused or published independently if desired.

### MCP framework: `rmcp` (v1.6)
- `rmcp` is the canonical Rust MCP implementation, maintained by the MCP team.
- Features enabled: `server`, `transport-io`, `schemars`.
- Uses `schemars` for JSON Schema generation from Rust types (used for tool argument schemas).

### HTTP client: `reqwest` with middleware
- `reqwest` core client with `reqwest-middleware` for composable middleware.
- `reqwest-retry` provides automatic retry with exponential backoff for transient failures.

### Testing strategy
- `mockall` for trait-based mocking of the API client in unit tests.
- `wiremock` for HTTP-level integration tests (stubbing Kagi API endpoints).

### Error handling
- Domain errors in `kagi-api` use `thiserror` for typed error enums.
- Application-level propagation in `kagi-mcp` uses `anyhow`.

## Logging

### Log levels
- `INFO` — user-facing events (tool invocation, result count, elapsed time, cache hit).
- `WARN` — recoverable issues (transient API failure before retry, rate-limit approaching).
- `ERROR` — blocking failures (API unreachable after retries, invalid config, internal panic).
- `DEBUG` — developer internals (cache store hit/miss, request construction, middleware steps).
- `TRACE` — very low-level (raw HTTP headers, serialization details, loop iterations).

### Log format
- Compact single-line format with timestamp, level, target, and message.
- No ANSI escape codes in file output.
- Example: `2026-05-24T10:30:00.123Z INFO kagi_mcp::tools::search Handler::call - query="rust" elapsed=342ms cache=hit`

### Log location
- Written to the cache directory (see `--cache-dir` / `KAGI_CACHE_DIR`).
- Daily rotation with filename pattern `kagi-mcp.log.YYYY-MM-DD`.
- Old logs are not automatically pruned; the cache TTL/size limits do not apply to log files.

### Transport behavior
- **Stdio** (`--transport stdio`): logs are written to file only. Stdio is reserved for MCP protocol messages.
- **StreamableHttp** (`--transport streamable-http`): logs are written to file **and** stdout. No stderr logging in any mode.

### Filtering
- Default filter level: `INFO` (shows INFO, WARN, ERROR; hides DEBUG and TRACE).
- Override via `RUST_LOG` environment variable (e.g., `RUST_LOG=debug`, `RUST_LOG=kagi_mcp::cache=trace`).

### Log ownership (no duplicates)
- **Handlers** (`search`, `extract`) own INFO-level timing and cache-hit logging — one log per invocation.
- **Cache store** owns DEBUG-level hit/miss logging — not duplicated at INFO.
- **Client layer** does not log timing (handlers own that).
- No log is emitted at multiple levels for the same event.

### What to log
- Tool inputs (query, URL, parameters) at INFO on entry.
- Elapsed time and result metadata at INFO on completion.
- Cache hit/miss at DEBUG in the cache store.
- Error context (status code, retry attempt, truncated response) at the appropriate level.

### What NOT to log
- API keys or sensitive configuration values.
- Full response bodies (use result metadata instead).
- Timing that duplicates handler-level INFO logs.

All changes must be submitted via Pull Requests.

### Branching
- Create a feature branch from `origin/master` for every change set.
- Before starting work, fetch the latest changes and rebase your branch onto `origin/master`.

### PR Lifecycle
- Open a single PR when work is ready for review.
- Address review feedback by committing and pushing additional changes to the same branch; they will automatically appear in the PR.
- PRs are merged with **squash merge**.
- The PR title must follow **Conventional Commits** style (e.g., `feat:`, `fix:`, `docs:`), because the squashed commit message is derived from the PR title.

## CI/CD Maintenance

After making changes to GitHub Actions workflows, run the following command to ensure all actions are pinned to their latest versions:

```bash
npx actions-up --mode patch --recursive --yes
```

This updates action references to the latest compatible versions and pins them to immutable SHAs.
