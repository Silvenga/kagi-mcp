# Search Tool

## OVERVIEW
MCP `search` tool: queries Kagi Search API, caches results, deduplicates by domain, renders Markdown.

## STRUCTURE
```
search/
├── dedup.rs    # Domain-based deduplication
├── group.rs    # Result grouping logic
├── handler.rs  # Main search handler (788 lines)
└── params.rs   # SearchParams JSON Schema
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Change search logic | `handler.rs` | Cache check → API call → dedup → format → truncate |
| Change params | `params.rs` | Derive `schemars::JsonSchema` |
| Domain dedup | `dedup.rs` | `limit_per_domain` feature |
| Result grouping | `group.rs` | Kagi grouping key fallback |

## CONVENTIONS
- Handler: `search_handler(client, params, ctx, config, cache)`
- `OVERFETCH_LIMIT=1024` upstream, dedup to local limit
- Errors mapped via `tools::errors::map_kagi_error`
- Responses truncated to `DEFAULT_MAX_RESPONSE_BYTES`

## ANTI-PATTERNS
- Never modify cache schema for application needs
- Do not leak raw Kagi API errors to MCP client
