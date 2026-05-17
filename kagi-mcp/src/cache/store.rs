use crate::cache::error::CacheError;
use crate::cache::evict::evict_if_needed;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{ConnectOptions, Connection, SqliteConnection};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A SQLite-backed cache store for Kagi API responses.
///
/// Opens a fresh connection for every operation so that multiple
/// processes can safely share the same on-disk cache.
pub struct CacheStore {
    connect_options: SqliteConnectOptions,
    max_size_bytes: u64,
    ttl_seconds: u64,
}

impl CacheStore {
    /// Creates a new `CacheStore` with the given parameters.
    ///
    /// - Creates the cache directory (with 0700 permissions on Unix).
    /// - Opens the SQLite database with WAL mode and busy timeout.
    /// - Runs pending migrations.
    pub async fn new(
        cache_dir: &Path,
        max_size_gb: f64,
        ttl_days: u64,
    ) -> Result<Self, CacheError> {
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

        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5))
            .create_if_missing(true);

        let mut conn = connect_options.connect().await?;
        sqlx::migrate!("./migrations").run(&mut conn).await?;

        Ok(Self {
            connect_options,
            max_size_bytes: (max_size_gb * 1024.0 * 1024.0 * 1024.0) as u64,
            ttl_seconds: ttl_days * 24 * 60 * 60,
        })
    }

    /// Creates an in-memory `CacheStore` for testing.
    #[cfg(test)]
    pub async fn open_in_memory() -> Result<Self, CacheError> {
        let tmp = Box::leak(Box::new(tempfile::tempdir()?));
        let cache_dir = tmp.path().join("cache");
        Self::new(&cache_dir, 1.0, 1).await
    }

    async fn open_connection(&self) -> Result<SqliteConnection, CacheError> {
        self.connect_options.connect().await.map_err(Into::into)
    }

    /// Retrieves a cached response by key, checking TTL expiry.
    pub async fn get(&self, cache_key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let mut conn = self.open_connection().await?;
        let row: Option<(Vec<u8>, i64)> = sqlx::query_as(
            "SELECT response_json, created_at FROM cache_entries WHERE cache_key = ?",
        )
        .bind(cache_key)
        .fetch_optional(&mut conn)
        .await?;

        if let Some((response_json, created_at)) = row {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            if created_at < now.saturating_sub(self.ttl_seconds as i64) {
                sqlx::query("DELETE FROM cache_entries WHERE cache_key = ?")
                    .bind(cache_key)
                    .execute(&mut conn)
                    .await?;
                Ok(None)
            } else {
                Ok(Some(response_json))
            }
        } else {
            Ok(None)
        }
    }

    /// Stores a cached response.
    pub async fn set(
        &self,
        cache_key: &str,
        tool_type: &str,
        response_json: &[u8],
    ) -> Result<(), CacheError> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let size_bytes = response_json.len() as i64;

        let mut conn = self.open_connection().await?;
        let mut tx = conn.begin().await?;

        sqlx::query(
            "INSERT OR REPLACE INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(cache_key)
        .bind(tool_type)
        .bind(created_at)
        .bind(size_bytes)
        .bind(response_json)
        .execute(&mut *tx)
        .await?;

        evict_if_needed(&mut tx, self.max_size_bytes).await?;

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn when_new_then_creates_directory_and_sqlite_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();

        assert!(cache_dir.exists());
        assert!(cache_dir.join("cache.db").exists());

        let result = store.get("any").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn when_new_then_directory_has_0700_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let _store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();

        let metadata = fs::metadata(&cache_dir).unwrap();
        let permissions = metadata.permissions().mode();

        assert_eq!(permissions & 0o777, 0o700);
    }

    #[tokio::test]
    async fn when_open_in_memory_then_works() {
        let store = CacheStore::open_in_memory().await.unwrap();

        let result = store.get("any").await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn when_entry_exists_then_get_should_return_data() {
        let store = CacheStore::open_in_memory().await.unwrap();

        store.set("key1", "search", b"cached_data").await.unwrap();
        let result = store.get("key1").await.unwrap();

        assert_eq!(result, Some(b"cached_data".to_vec()));
    }

    #[tokio::test]
    async fn when_entry_missing_then_get_should_return_none() {
        let store = CacheStore::open_in_memory().await.unwrap();

        let result = store.get("missing_key").await.unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn when_entry_expired_then_get_should_return_none_and_delete() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let mut conn = store.open_connection().await.unwrap();

        sqlx::query(
            "INSERT INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("key1")
        .bind("search")
        .bind(0i64)
        .bind(4i64)
        .bind(b"data" as &[u8])
        .execute(&mut conn)
        .await
        .unwrap();

        let result = store.get("key1").await.unwrap();

        assert_eq!(result, None);

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM cache_entries WHERE cache_key = ?")
                .bind("key1")
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn when_set_twice_then_get_should_return_latest() {
        let store = CacheStore::open_in_memory().await.unwrap();

        store.set("key1", "search", b"first").await.unwrap();
        store.set("key1", "search", b"second").await.unwrap();

        let result = store.get("key1").await.unwrap();

        assert_eq!(result, Some(b"second".to_vec()));
    }

    #[tokio::test]
    async fn when_set_and_get_then_should_roundtrip() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let data = b"roundtrip_payload";

        store.set("round_key", "extract", data).await.unwrap();
        let result = store.get("round_key").await.unwrap();

        assert_eq!(result, Some(data.to_vec()));
    }

    #[tokio::test]
    async fn when_overwrite_then_timestamp_should_update() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let mut conn = store.open_connection().await.unwrap();

        sqlx::query(
            "INSERT INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json) VALUES (?, ?, ?, ?, ?)",
        )
        .bind("key1")
        .bind("search")
        .bind(0i64)
        .bind(5i64)
        .bind(b"first" as &[u8])
        .execute(&mut conn)
        .await
        .unwrap();

        store.set("key1", "search", b"second").await.unwrap();

        let timestamp: (i64,) =
            sqlx::query_as("SELECT created_at FROM cache_entries WHERE cache_key = ?")
                .bind("key1")
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert!(timestamp.0 > 0);
    }
}
