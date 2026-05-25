# PROJECT KNOWLEDGE BASE

**Generated:** 2026-05-25T21:34:57Z
**Commit:** ef08977
**Branch:** agents-mds

## OVERVIEW
Rust workspace for a Kagi Search API MCP server. `kagi-api` (library) wraps the HTTP API. `kagi-mcp` (binary) exposes search and extract tools via the Model Context Protocol.

## STRUCTURE
```
kagi-mcp/
├── Cargo.toml              # Workspace root — shared deps + clippy lints
├── kagi-api/               # Library crate — Kagi API client
│   └── src/                # Flat public API (all re-exported from root)
└── kagi-mcp/               # Binary crate — MCP server
    └── src/                # server, config, logging, cache, format, tools
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add request/response type | `kagi-api/src/types/` | Derive Serialize/Deserialize + builder |
| Add a tool | `kagi-mcp/src/tools/` | Follow existing handler + params pattern |
| Change output formatting | `kagi-mcp/src/format/` | Markdown via askama templates, JSON via serde |
| Change cache behavior | `kagi-mcp/src/cache/` | See `cache/AGENTS.md` for subsystem rules |
| Change CLI options | `kagi-mcp/src/config.rs` | Env vars auto-wired via clap `env = ...` |
| Mock API for tests | `kagi-api/src/api_trait.rs` | Enable `mock` feature for `MockKagiApi` |

## CONVENTIONS

### Workspace dependencies
- Declare all shared deps in root `Cargo.toml` `[workspace.dependencies]`
- Crates use `dep.workspace = true`; no local version pins

### Error handling
- Library: `thiserror` typed errors with `#[from]` conversions
- Binary: `anyhow` for application-level propagation

### Public API (kagi-api)
- All types/traits re-exported from crate root via `pub use`
- Never import from submodules (e.g., `kagi_api::types::Foo` is private)

### Builder-style config
- `with_*` methods returning `Self` for optional fields
- `build()` validates and returns `Result`

## ANTI-PATTERNS
- Importing from `kagi_api::client` or `kagi_api::types` — use root re-exports only
- Adding application concerns to cache schema — storage-layer only
- `print_stdout` / `print_stderr` (clippy-warn) — use logging instead
- Background tasks in cache — all work must be synchronous

## UNIQUE STYLES

### MCP wiring
- Tools via `rmcp` macros: `#[tool_router(vis = "pub")]` + `#[tool]` + `#[tool_handler]`
- Params derive `schemars::JsonSchema` for automatic JSON Schema

### Cache design
- SQLite on disk with WAL mode, no connection pooling
- FIFO eviction by `created_at`, TTL checked lazily on read
- See `kagi-mcp/src/cache/AGENTS.md` for full invariants

### Fallback rules
- Per-domain fallback messages after extract failures
- `--fallback-always` skips Kagi API entirely for matched domains
- Domain matching uses eTLD+1 (registrable domain, case-insensitive)

## COMMANDS
```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
cargo doc --workspace --no-deps
```

## NOTES
- Rust 1.84+ required (workspace `rust-version`)
- Extensive clippy lint config in root `Cargo.toml`
- MCP transport: `stdio` (default) or `streamable-http` (`--transport streamable-http`)
- Default HTTP bind: `127.0.0.1:3000`
- After editing GitHub Actions, run `npx actions-up --mode patch --recursive --yes` to pin action versions
