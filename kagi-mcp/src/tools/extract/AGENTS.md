# Extract Tool

## OVERVIEW

MCP `extract` tool: validates URLs, calls Kagi Extract API in batch mode, applies per-domain fallback
rules, renders Markdown.

## STRUCTURE

```
extract/
├── batch.rs      # Batch extraction (single API call)
├── errors.rs     # Extract-specific error formatting
├── fallback.rs   # FallbackRules + eTLD+1 matching
├── handler.rs    # Main extract handler
├── params.rs     # ExtractParams JSON Schema
└── validation.rs # URL + count validation
```

## WHERE TO LOOK

| Task                 | Location                | Notes                                      |
|----------------------|-------------------------|--------------------------------------------|
| Change extract logic | `handler.rs`            | Cache → batch dispatch → fallback |
| Change params        | `params.rs`             | `pages` array, max 10 URLs          |
| Per-domain fallback  | `fallback.rs`           | eTLD+1 matching, `always_block`     |
| URL validation       | `validation.rs`         | HTTPS only, count limits            |

## CONVENTIONS

- Handler: `extract_handler(client, params, ctx, timeout, cache, fallback)`
- Fallbacks: if API returns empty content, substitute a configured message
- `--fallback-always` skips API entirely for matched domains

## ANTI-PATTERNS

- Do not leak raw API errors to MCP client
