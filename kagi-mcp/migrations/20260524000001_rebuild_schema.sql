DROP TABLE IF EXISTS cache;

CREATE TABLE IF NOT EXISTS cache (
    cid BLOB NOT NULL PRIMARY KEY,
    created_at INTEGER NOT NULL,
    type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    value BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_created_at ON cache(created_at);
CREATE INDEX IF NOT EXISTS idx_type ON cache(type);
