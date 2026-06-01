# Metrics Module — Architecture & Design

## OVERVIEW
SQLite-backed metrics store for tracking daily Kagi API usage. Separate from cache module.

## STRUCTURE
```
metrics/
├── error.rs    # MetricsError enum
├── models.rs   # DailyMetrics struct
└── store.rs    # MetricsStore with UPSERT operations
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add metric counter | store.rs | increment_* methods |
| Query metrics | store.rs | get_monthly_metrics() |
| Change model | models.rs | DailyMetrics fields |

## CONVENTIONS
- MetricsStore owns own SqliteConnectOptions to same cache.db
- Fire-and-forget writes: increment methods return (), errors logged via tracing::warn!
- UTC day boundaries via chrono::Utc::now()
- UPSERT via literal SQL with bind parameters
- Does NOT create dir or run migrations (CacheStore handles that)

## ANTI-PATTERNS
- Do NOT add metrics methods to CacheStore
- Do NOT propagate metrics errors to callers
- Do NOT use dynamic SQL strings (SqlSafeStr lint)
- Do NOT create background tasks for metrics

## CONSTRAINTS
1. Keep schema minimal — no per-hour/per-domain granularity
2. No shared mutable state in memory
3. No background tasks — all work synchronous per-call
4. Metrics failures never break search/extract operations
