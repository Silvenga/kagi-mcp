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

Table: `metrics`

- `year INTEGER NOT NULL` — Year component of the date
- `month INTEGER NOT NULL` — Month component (1-12)
- `day INTEGER NOT NULL` — Day component (1-31)
- `total_extract_requests INTEGER NOT NULL DEFAULT 0` — Total extract API calls
- `total_search_requests INTEGER NOT NULL DEFAULT 0` — Total search API calls
- `total_extract_urls_from_cache INTEGER NOT NULL DEFAULT 0` — Extract cache hits
- `total_search_requests_from_cache INTEGER NOT NULL DEFAULT 0` — Search cache hits
- `failed_extract_urls INTEGER NOT NULL DEFAULT 0` — Failed extract URL count

Primary key: `(year, month, day)`

Indexes:

- `idx_metrics_year_month ON metrics(year, month)`

## RULES

- File format: `YYYYMMDD00000N_migration_name.sql`
- Always run `date` before creating a new migration
- Update this file when adding a new migration
