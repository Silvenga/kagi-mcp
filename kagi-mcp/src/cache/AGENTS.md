# Cache Module — Architecture & Design

> This document describes the design goals and constraints for the `kagi-mcp` cache subsystem. It is intended to guide future changes to the cache module itself, not the calling code.

## Design Goals

The cache exists for two primary reasons:

1. **Cost savings** — Avoid redundant API calls. This is expected to be most effective for `extract` (low cardinality: many agents likely ask about the same URLs). Search cache hits are expected to be rare outside of failure/recovery scenarios (e.g., agent loops, retries).

2. **Future multi-turn interactions** — The cache stores the full raw API response so that future operations (scrolling, summarization, Q&A on extracted text) can re-use the data without re-fetching. The cache must therefore store *raw* responses, not rendered/filtered outputs.

## Architecture Decisions

### SQLite on Disk with WAL Mode

The cache is backed by a SQLite database with WAL (Write-Ahead Logging) enabled.

**Rationale:**
- **Multi-process safety** is a hard requirement. Multiple independent MCP server processes may run concurrently (e.g., different IDE windows, different agents). SQLite with WAL mode allows multiple readers and a single writer without explicit locking from the application layer.
- **No connection pooling.** A fresh SQLite connection is opened for every `get`/`set` operation. This is intentional: it guarantees that each process can safely interact with the same on-disk database even if the process was started in isolation. Connection reuse would introduce cross-process coordination complexity that WAL mode already solves.
- **WAL mode (`SqliteJournalMode::Wal`)** combined with a 5-second busy timeout ensures writers safely queue behind readers without requiring application-level locks.

### Crash Safety

MCP servers are frequently killed without cleanup (agent cancels, IDE closes, process OOMs). The storage layer must be robust to incomplete transactions.

**Rationale:**
- WAL mode is inherently crash-safe: incomplete transactions are rolled back automatically on the next connection open.
- All cache writes use `BEGIN ... COMMIT` transactions encompassing both the `INSERT OR REPLACE` and the eviction pass. This ensures the database is never in a partially-evicted state.
- The database is opened with `create_if_missing(true)`. If the file is corrupted or truncated (e.g., by a brutal process kill), SQLite will detect the WAL mismatch and gracefully recover or recreate.

### Size Cap: Safety Valve, Not a Target

The configurable max size (`--cache-size-gb`, default 5 GB) exists solely to prevent unbounded disk growth. It is **not expected to be hit in normal usage**.

**Rationale:**
- Because the cap is a safety valve, the eviction policy optimizes for simplicity over sophistication: **FIFO by `created_at`** (oldest entries deleted first).
- **LFU (Least Frequently Used) is explicitly undesirable.** Tracking access patterns adds complexity (extra reads/writes per cache hit, bookkeeping tables) with no commensurate benefit given the expected workload. Search queries have high cardinality; extract URLs have high repeatability but low churn. A simple FIFO is sufficient and predictable.
- Eviction runs synchronously inside every `set()` transaction. This is acceptable because eviction is expected to be a no-op in the common case.

### TTL: Freshness, Not Expiry

The configurable TTL (`--cache-ttl-days`, default 7) exists to keep cached data somewhat fresh. Search and extract results can change over time, and stale data is worse than a re-fetch.

**Rationale:**
- TTL is checked **lazily on read** (`get()`), not proactively cleaned. This avoids background threads or timers. Expired entries are deleted at the moment they are accessed.
- `INSERT OR REPLACE` resets `created_at` to `now` on overwrite. A re-fetch of the same query effectively refreshes the TTL — this is desirable behavior.
- The TTL is global and fixed; there is no per-tool-type or per-entry TTL override. This keeps the schema simple. If different freshness needs arise in the future, they should be solved at the calling layer (e.g., callers passing `cache: false` for time-sensitive queries), not in the storage layer.

## Constraints on Future Changes

When modifying this module, keep the following invariants:

1. **Keep the schema minimal.** Every column should have a clear storage-layer purpose. Do not add columns for application concerns (e.g., `last_accessed`, `hit_count`, `user_id`).
2. **Do not introduce shared state in memory.** No `Mutex<Vec<...>>`, no in-memory LRU, no connection pools. The design depends on every operation being self-contained and process-agnostic.
3. **Do not add background tasks.** No timers, no async cleanup loops, no `tokio::spawn` for eviction or TTL expiry. All work must be synchronous within the `get`/`set` call.
4. **Preserve the FIFO eviction contract.** If the eviction policy ever needs to change, it must still be deterministic, cheap to compute from existing columns, and require no new indexes or tables.
5. **Store raw bytes only.** The `value BLOB` must remain opaque JSON bytes. Do not parse, normalize, or filter responses at the cache layer. Future features (scrolling, summarization) depend on having the full original response available.

## What This Module Does NOT Do

- It does not know about `SearchRequest` vs `ExtractRequest` semantics. The `type` column is stored for debugging/diagnostics only; it is not queried.
- It does not handle serialization or deserialization. Callers pass raw bytes; the store treats them as opaque blobs.
- It does not implement cache invalidation by external signals (e.g., "Kagi API updated"). Invalidation is purely TTL-based.
- It does not implement cache warming or pre-population.
