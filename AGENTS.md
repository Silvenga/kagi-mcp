# Kagi MCP Server — Project Workspace

## Workspace Structure

```
kagi-mcp/
├── Cargo.toml              # Workspace root
├── AGENTS.md               # This file
├── kagi-api/               # Library crate — Kagi API client
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Module declarations
│       ├── client.rs       # HTTP client (reqwest-based)
│       ├── error.rs        # Domain error types (thiserror)
│       └── types.rs        # Request/response types (serde)
└── kagi-mcp/               # Binary crate — MCP server
    ├── Cargo.toml
    └── src/
        └── main.rs         # Entrypoint (rmcp server)
```

## Build & Test Commands

```bash
# Check entire workspace
cargo check --workspace

# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run clippy
cargo clippy --workspace
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
- `async-trait` enables async trait methods on the client, which `mockall` can mock.

### Error handling
- Domain errors in `kagi-api` use `thiserror` for typed error enums.
- Application-level propagation in `kagi-mcp` uses `anyhow`.
