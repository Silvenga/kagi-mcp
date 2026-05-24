DROP TABLE IF EXISTS cache_entries;

CREATE TABLE IF NOT EXISTS cache_entries (
    cid BLOB NOT NULL PRIMARY KEY,
    created_at INTEGER NOT NULL,
    type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    value BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_created_at ON cache_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_type ON cache_entries(type);
