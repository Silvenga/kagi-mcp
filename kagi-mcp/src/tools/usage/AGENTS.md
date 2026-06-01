# Usage Tool

## OVERVIEW

MCP `usage` tool: queries the metrics store for daily Kagi API usage statistics for a given month and renders them as a Markdown table.

## STRUCTURE

```
usage/
├── handler.rs  # Main usage handler
├── mod.rs      # Module entrypoint and exports
└── params.rs   # UsageParams JSON Schema
```

## WHERE TO LOOK

| Task                | Location     | Notes                                              |
|---------------------|--------------|----------------------------------------------------|
| Change usage logic  | `handler.rs` | Parse month → query metrics → format Markdown      |
| Change params       | `params.rs`  | `month` parameter (YYYY-MM format)                 |

## CONVENTIONS

- Handler: `usage_handler(metrics_store, params)`
- Month format: `YYYY-MM` (validated via regex)
- Defaults to current UTC month if no month is provided
- Renders output using `format_usage_markdown`

## ANTI-PATTERNS

- Do not query the cache database directly for metrics
- Do not propagate internal database errors to the MCP client
