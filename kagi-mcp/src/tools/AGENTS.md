# Tools Module

## OVERVIEW
MCP tool handlers for search and extract. Each tool has params, handler, and supporting modules.

## STRUCTURE
```
tools/
├── search/       # Search tool
│   ├── dedup.rs      # Domain-based deduplication
│   ├── group.rs      # Result grouping logic
│   ├── handler.rs    # Main search handler
│   └── params.rs     # SearchParams struct
└── extract/      # Extract tool
    ├── batch.rs      # Batch extraction (single API call)
    ├── errors.rs     # Extract-specific errors
    ├── fallback.rs   # Per-domain fallback rules
    ├── handler.rs    # Main extract handler
    ├── params.rs     # ExtractParams struct
    ├── split.rs      # Per-URL split extraction
    └── validation.rs # URL validation
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Change search logic | `search/handler.rs` | Cache check, dedup, format, truncate |
| Change search params | `search/params.rs` | Derive `schemars::JsonSchema` |
| Domain dedup | `search/dedup.rs` | `limit_per_domain` feature |
| Result grouping | `search/group.rs` | Kagi grouping key fallback |
| Change extract logic | `extract/handler.rs` | Cache, split vs batch dispatch |
| Change extract params | `extract/params.rs` | `pages` array validation |
| Per-URL fallback | `extract/fallback.rs` | eTLD+1 domain matching |
| Split vs batch | `extract/split.rs` / `extract/batch.rs` | `--split-extract-requests` |
| URL validation | `extract/validation.rs` | HTTPS, page count limits |

## CONVENTIONS
- Handlers take `&dyn KagiApi` client, params, `RequestContext`, config
- Search: `OVERFETCH_LIMIT=1024` upstream, dedup to local limit
- Extract: `split` = individual API call per URL; `batch` = single call
- Errors mapped via `tools::errors::map_kagi_error` to MCP `ErrorData`
- Responses truncated to `DEFAULT_MAX_RESPONSE_BYTES`

## ANTI-PATTERNS
- Never modify cache schema for application needs
- Never use `tokio::spawn` in tool handlers — synchronous execution
- Do not leak raw API errors to MCP client — always map via `map_kagi_error`
