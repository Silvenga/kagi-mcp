# Tools Module

## OVERVIEW

MCP tool handlers: `search` and `extract`. Each tool has its own AGENTS.md.

## SUBMODULES

| Tool       | AGENTS.md           |
|------------|---------------------|
| `search/`  | `search/AGENTS.md`  |
| `extract/` | `extract/AGENTS.md` |

## SHARED HELPERS

| File          | Purpose                                      |
|---------------|----------------------------------------------|
| `errors.rs`   | `map_kagi_error` — KagiError → MCP ErrorData |
| `progress.rs` | MCP progress notifications                   |
| `truncate.rs` | Response truncation at UTF-8 boundary        |
| `domain.rs`   | eTLD+1 extraction, domain matching           |

## ANTI-PATTERNS

- Do not modify cache schema for application concerns
- Do not leak raw Kagi API errors to MCP client
