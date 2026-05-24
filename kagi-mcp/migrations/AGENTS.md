# Cache Schema Migrations

This directory contains the database migrations for the Kagi MCP server cache.

## Current Schema Version

`20260524000001_rebuild_schema.sql`

## Schema Definition

The `cache` table stores cached responses from the Kagi API.

### Columns

* `cid BLOB NOT NULL PRIMARY KEY`: 16-byte content ID (XXH3-128 hash with version salt)
* `created_at INTEGER NOT NULL`: Unix timestamp when the entry was created
* `type TEXT NOT NULL`: Tool type for debugging, such as "search" or "extract"
* `size_bytes INTEGER NOT NULL`: Size of the cached value in bytes
* `value BLOB NOT NULL`: Raw JSON response bytes

### Indexes

* `idx_created_at ON cache(created_at)`
* `idx_type ON cache(type)`

## Rules

- Update this file whenever a new migration is created.
- Always run `date` to get the current date. The format of the migration file is `YYYYMMDD00000N_migration_name.sql`.