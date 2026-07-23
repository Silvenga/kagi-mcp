# kagi-mcp Binary Crate

## OVERVIEW
MCP server binary embedding `kagi-api`. Exposes `search` and `extract` tools via rmcp with stdio or HTTP SSE transport.

## STRUCTURE
```
src/
├── main.rs       # Entrypoint — config parse, cache init, transport dispatch
├── server.rs     # MCP server — tool router + builder config
├── config.rs     # CLI args + env (clap)
├── logging.rs    # Tracing subscriber (daily rotation, PID prefix)
├── cache/        # SQLite cache subsystem
├── format/       # Markdown/JSON renderers
└── tools/        # MCP tool handlers
    ├── search/   # Search tool + dedup
    └── extract/  # Extract tool + batch/split + fallback
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Change tool behavior | `src/tools/search/` or `src/tools/extract/` | Handler + params + submodules |
| Change response format | `src/format/search.rs` or `src/format/extract.rs` | askama templates for Markdown |
| Change cache logic | `src/cache/` | See `src/cache/AGENTS.md` |
| Add CLI flag | `src/config.rs` | `clap` derive with `env =` |
| Logging setup | `src/logging.rs` | Daily rotation, stdout only for HTTP mode |
| Server wiring | `src/server.rs` | `with_*` builder + `#[tool]` macros |

## CONVENTIONS
- Error propagation: `anyhow` at top level, `map_err` to add context
- Tool params derive `schemars::JsonSchema` for MCP schema
- All `Option<String>` tool params must use `deserialize_optional_string_non_empty` — GPT models emit `""` for omitted optional params, which is coerced to `None` at the parse boundary.
- Askama templates in `templates/` for Markdown output
- Test helpers: `#[cfg(test)] mod tests` at bottom of file, AAA style

## LOGGING
- `INFO` — user-facing events (tool invocation, result count, elapsed, cache hit)
- `WARN` — recoverable issues (transient API failure before retry, rate-limit)
- `ERROR` — blocking failures (API unreachable after retries, invalid config)
- `DEBUG` — dev internals (cache hit/miss, request construction)
- `TRACE` — very low-level (raw HTTP headers, serialization)
- **Stdio mode**: logs to file only (stdout reserved for MCP protocol)
- **HTTP mode**: logs to file + stdout
- Default filter: `INFO`; override via `RUST_LOG`
- Daily rotation: `kagi-mcp.log.YYYY-MM-DD` in cache directory
- Custom `PidLineWriter` prefixes each line with `[pid={pid}]`

## ANTI-PATTERNS
- Do not log API keys or full response bodies
- Do not use stderr — stdio mode reserves it for MCP protocol
- Do not introduce shared mutable state in cache — per-operation connections only
