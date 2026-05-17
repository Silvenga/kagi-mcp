CREATE TABLE IF NOT EXISTS cache_entries (
    cache_key TEXT PRIMARY KEY,
    tool_type TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    response_json BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_created_at ON cache_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_tool_type ON cache_entries(tool_type);
