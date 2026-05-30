# Cache Schema Migrations

## OVERVIEW

SQLite migration files for the cache database.

## SCHEMA

Table: `cache`

- `cid BLOB NOT NULL PRIMARY KEY` — 16-byte XXH3-128 hash with version salt
- `created_at INTEGER NOT NULL` — Unix timestamp
- `type TEXT NOT NULL` — "search" or "extract"
- `size_bytes INTEGER NOT NULL`
- `value BLOB NOT NULL` — raw JSON response

Indexes:

- `idx_created_at ON cache(created_at)`
- `idx_type ON cache(type)`

## RULES

- File format: `YYYYMMDD00000N_migration_name.sql`
- Always run `date` before creating a new migration
- Update this file when adding a new migration
