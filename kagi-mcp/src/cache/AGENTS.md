# Cache Module — Architecture & Design

## OVERVIEW
SQLite-backed cache for Kagi API responses. Designed for multi-process safety and crash resilience.

## ARCHITECTURE

### SQLite with WAL Mode
- **Multi-process safety**: Multiple MCP server processes may run concurrently
- **No connection pooling**: Fresh SQLite connection per `get`/`set` operation
- **WAL mode** + 5-second busy timeout for safe writer queuing

### Crash Safety
- WAL mode auto-rolls back incomplete transactions on next open
- All writes in `BEGIN ... COMMIT` transactions covering both insert and eviction
- Database opened with `create_if_missing(true)`

### Size Cap: Safety Valve
- Max size (`--cache-size-gb`, default 5 GB) prevents unbounded growth
- **FIFO eviction** by `created_at` — oldest entries deleted first
- **LFU is undesirable**: no access-pattern tracking, no extra bookkeeping
- Eviction runs synchronously inside every `set()` transaction

### TTL: Freshness, Not Expiry
- TTL checked **lazily on read** (no background threads)
- `INSERT OR REPLACE` resets `created_at` on overwrite
- No per-tool-type or per-entry TTL override

## CONSTRAINTS (when modifying)
1. Keep schema minimal — no `last_accessed`, `hit_count`, `user_id`
2. No shared mutable state in memory — no `Mutex<Vec>`, no in-memory LRU
3. No background tasks — no timers, no async cleanup loops
4. Preserve FIFO eviction contract — deterministic, cheap, no new indexes
5. Store raw `BLOB` JSON only — do not parse or normalize at cache layer

## SCHEMA
- `cid BLOB PRIMARY KEY` — 16-byte XXH3-128 hash with version salt
- `created_at INTEGER` — Unix timestamp
- `type TEXT` — tool type ("search" or "extract")
- `size_bytes INTEGER`
- `value BLOB` — raw JSON response
- Indexes: `idx_created_at`, `idx_type`
