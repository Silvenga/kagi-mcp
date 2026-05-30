# Extract Tool

## OVERVIEW

MCP `extract` tool: validates URLs, optionally splits per-URL, calls Kagi Extract API, applies per-domain fallback
rules, renders Markdown.

## STRUCTURE

```
extract/
├── batch.rs      # Batch extraction (single API call)
├── errors.rs     # Extract-specific error formatting
├── fallback.rs   # FallbackRules + eTLD+1 matching
├── handler.rs    # Main extract handler
├── params.rs     # ExtractParams JSON Schema
├── split.rs      # Per-URL split extraction
└── validation.rs # URL + count validation
```

## WHERE TO LOOK

| Task                 | Location                | Notes                                      |
|----------------------|-------------------------|--------------------------------------------|
| Change extract logic | `handler.rs`            | Cache → split vs batch dispatch → fallback |
| Change params        | `params.rs`             | `pages` array, max 10 URLs                 |
| Per-domain fallback  | `fallback.rs`           | eTLD+1 matching, `always_block`            |
| Split vs batch       | `split.rs` / `batch.rs` | `--split-extract-requests` toggle          |
| URL validation       | `validation.rs`         | HTTPS only, count limits                   |

## CONVENTIONS

- Handler: `extract_handler(client, params, ctx, timeout, split, cache, fallback)`
- `split` = individual API call per URL; `batch` = single call
- Fallbacks: if API returns empty content, substitute a configured message
- `--fallback-always` skips API entirely for matched domains

## ANTI-PATTERNS

- Do not leak raw API errors to MCP client
