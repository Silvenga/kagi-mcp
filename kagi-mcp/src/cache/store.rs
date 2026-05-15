use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::cache::error::CacheError;
use crate::cache::evict::evict_if_needed;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS cache_entries (
    cache_key TEXT PRIMARY KEY,
    tool_type TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    response_json BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_created_at ON cache_entries(created_at);
CREATE INDEX IF NOT EXISTS idx_tool_type ON cache_entries(tool_type);
"#;

/// A SQLite-backed cache store for Kagi API responses.
///
/// Stores the database path rather than a connection to remain `Send + Sync`.
pub struct CacheStore {
    db_path: PathBuf,
    max_size_bytes: u64,
    ttl_seconds: u64,
}

impl CacheStore {
    /// Creates a new `CacheStore` with the given parameters.
    ///
    /// - Creates the cache directory (with 0700 permissions on Unix).
    /// - Opens the SQLite database, enables WAL mode, sets busy timeout.
    /// - Initializes the schema.
    pub fn new(cache_dir: &Path, max_size_gb: f64, ttl_days: u64) -> Result<Self, CacheError> {
        if max_size_gb <= 0.0 {
            return Err(CacheError::InvalidConfig(
                "max_size_gb must be positive".into(),
            ));
        }

        fs::create_dir_all(cache_dir)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o700);
            fs::set_permissions(cache_dir, permissions)?;
        }

        let db_path = cache_dir.join("cache.db");
        let store = Self {
            db_path,
            max_size_bytes: (max_size_gb * 1024.0 * 1024.0 * 1024.0) as u64,
            ttl_seconds: ttl_days * 24 * 60 * 60,
        };

        let conn = store.open_connection()?;
        conn.execute_batch(SCHEMA_SQL)?;

        Ok(store)
    }

    /// Creates an in-memory `CacheStore` for testing.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, CacheError> {
        let tmp = Box::leak(Box::new(tempfile::tempdir()?));
        let cache_dir = tmp.path().join("cache");
        Self::new(&cache_dir, 1.0, 1)
    }

    /// Retrieves a cached response by key, checking TTL expiry.
    pub fn get(&self, cache_key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let conn = self.open_connection()?;
        let mut stmt = conn
            .prepare("SELECT response_json, created_at FROM cache_entries WHERE cache_key = ?")?;

        let result = stmt.query_row(rusqlite::params![cache_key], |row| {
            let response_json: Vec<u8> = row.get(0)?;
            let created_at: i64 = row.get(1)?;
            Ok((response_json, created_at))
        });

        match result {
            Ok((response_json, created_at)) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;

                if created_at < now.saturating_sub(self.ttl_seconds as i64) {
                    conn.execute(
                        "DELETE FROM cache_entries WHERE cache_key = ?",
                        rusqlite::params![cache_key],
                    )?;
                    Ok(None)
                } else {
                    Ok(Some(response_json))
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Stores a cached response with an immediate transaction.
    pub fn set(
        &self,
        cache_key: &str,
        tool_type: &str,
        response_json: &[u8],
    ) -> Result<(), CacheError> {
        let mut conn = self.open_connection()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let size_bytes = response_json.len() as i64;

        tx.execute(
            "INSERT OR REPLACE INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![cache_key, tool_type, created_at, size_bytes, response_json],
        )?;

        evict_if_needed(&tx, self.max_size_bytes)?;

        tx.commit()?;
        Ok(())
    }

    fn open_connection(&self) -> Result<Connection, CacheError> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;",
        )?;
        Ok(conn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_new_then_creates_directory_and_sqlite_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();

        assert!(cache_dir.exists());
        assert!(store.db_path.exists());
    }

    #[test]
    fn when_new_then_sqlite_uses_wal_mode() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let _store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();

        let conn = Connection::open(cache_dir.join("cache.db")).unwrap();
        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .unwrap();

        assert_eq!(journal_mode, "wal");
    }

    #[test]
    #[cfg(unix)]
    fn when_new_then_directory_has_0700_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let _store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();

        let metadata = fs::metadata(&cache_dir).unwrap();
        let permissions = metadata.permissions().mode();

        assert_eq!(permissions & 0o777, 0o700);
    }

    #[test]
    fn when_open_in_memory_then_works() {
        let store = CacheStore::open_in_memory().unwrap();

        assert!(store.db_path.to_string_lossy().contains("cache.db"));
    }

    #[test]
    fn when_new_then_schema_created_successfully() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let store = CacheStore::new(&cache_dir, 1.0, 7).unwrap();

        let conn = store.open_connection().unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='cache_entries';")
            .unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
        let names: Vec<String> = rows.map(|r| r.unwrap()).collect();

        assert_eq!(names, vec!["cache_entries"]);

        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%';")
            .unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(0)).unwrap();
        let names: Vec<String> = rows.map(|r| r.unwrap()).collect();

        assert!(names.contains(&"idx_created_at".to_owned()));
        assert!(names.contains(&"idx_tool_type".to_owned()));
    }

    #[test]
    fn when_entry_exists_then_get_should_return_data() {
        let store = CacheStore::open_in_memory().unwrap();

        store.set("key1", "search", b"cached_data").unwrap();
        let result = store.get("key1").unwrap();

        assert_eq!(result, Some(b"cached_data".to_vec()));
    }

    #[test]
    fn when_entry_missing_then_get_should_return_none() {
        let store = CacheStore::open_in_memory().unwrap();

        let result = store.get("missing_key").unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn when_entry_expired_then_get_should_return_none_and_delete() {
        let store = CacheStore::open_in_memory().unwrap();
        let conn = store.open_connection().unwrap();

        conn.execute(
            "INSERT INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params!["key1", "search", 0i64, 4i64, b"data"],
        )
        .unwrap();

        let result = store.get("key1").unwrap();

        assert_eq!(result, None);

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cache_entries WHERE cache_key = ?",
                rusqlite::params!["key1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn when_set_twice_then_get_should_return_latest() {
        let store = CacheStore::open_in_memory().unwrap();

        store.set("key1", "search", b"first").unwrap();
        store.set("key1", "search", b"second").unwrap();

        let result = store.get("key1").unwrap();

        assert_eq!(result, Some(b"second".to_vec()));
    }

    #[test]
    fn when_set_and_get_then_should_roundtrip() {
        let store = CacheStore::open_in_memory().unwrap();
        let data = b"roundtrip_payload";

        store.set("round_key", "extract", data).unwrap();
        let result = store.get("round_key").unwrap();

        assert_eq!(result, Some(data.to_vec()));
    }

    #[test]
    fn when_overwrite_then_timestamp_should_update() {
        let store = CacheStore::open_in_memory().unwrap();
        let conn = store.open_connection().unwrap();

        conn.execute(
            "INSERT INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json) VALUES (?, ?, ?, ?, ?)",
            rusqlite::params!["key1", "search", 0i64, 5i64, b"first"],
        )
        .unwrap();

        store.set("key1", "search", b"second").unwrap();

        let timestamp: i64 = conn
            .query_row(
                "SELECT created_at FROM cache_entries WHERE cache_key = ?",
                rusqlite::params!["key1"],
                |row| row.get(0),
            )
            .unwrap();
        assert!(timestamp > 0);
    }
}
