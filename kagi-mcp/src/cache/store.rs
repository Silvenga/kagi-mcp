use crate::cache::evict::evict_if_needed;
use crate::cache::generate_cid;
use crate::cache::CacheError;
use crate::cache::Cid;
use crate::cache::{ExtractCacheKey, ExtractCachedResult, SearchCacheKey, SearchCachedResult};
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

    pub(crate) async fn open_connection(&self) -> Result<SqliteConnection, CacheError> {
        self.connect_options.connect().await.map_err(Into::into)
    }

    /// Retrieves a cached response by CID, checking TTL expiry.
    pub(crate) async fn get(&self, cid: &Cid) -> Result<Option<Vec<u8>>, CacheError> {
        let mut conn = self.open_connection().await?;
        let row: Option<(Vec<u8>, i64)> =
            match sqlx::query_as("SELECT value, created_at FROM cache WHERE cid = ?")
                .bind(cid.as_slice())
                .fetch_optional(&mut conn)
                .await
            {
                Ok(row) => row,
                Err(e) => {
                    tracing::warn!(cid = ?cid, error = %e, "cache read error");
                    return Err(e.into());
                }
            };

        if let Some((value, created_at)) = row {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            if created_at < now.saturating_sub(self.ttl_seconds as i64) {
                sqlx::query("DELETE FROM cache WHERE cid = ?")
                    .bind(cid.as_slice())
                    .execute(&mut conn)
                    .await?;
                tracing::debug!(cid = ?cid, "cache entry expired, deleted");
                Ok(None)
            } else {
                tracing::debug!(cid = ?cid, "cache hit");
                Ok(Some(value))
            }
        } else {
            tracing::debug!(cid = ?cid, "cache miss");
            Ok(None)
        }
    }

    /// Stores a cached response.
    pub(crate) async fn set(&self, cid: &Cid, type_: &str, value: &[u8]) -> Result<(), CacheError> {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let size_bytes = value.len() as i64;

        let mut conn = self.open_connection().await?;
        let mut tx = conn.begin().await?;

        if let Err(e) = sqlx::query(
            "INSERT OR REPLACE INTO cache (cid, type, created_at, size_bytes, value) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(cid.as_slice())
        .bind(type_)
        .bind(created_at)
        .bind(size_bytes)
        .bind(value)
        .execute(&mut *tx)
        .await
        {
            tracing::warn!(cid = ?cid, error = %e, "cache write error");
            return Err(e.into());
        }

        evict_if_needed(&mut tx, self.max_size_bytes).await?;

        tx.commit().await?;
        tracing::debug!(cid = ?cid, type_ = %type_, size_bytes = size_bytes, "cache set");
        Ok(())
    }

    /// Retrieves a cached search result by key, checking TTL expiry.
    ///
    /// On deserialization failure, logs a warning with the CID and attempts to
    /// delete the corrupt entry (ignoring delete failure), then returns `None`.
    pub async fn get_search_result(&self, key: &SearchCacheKey) -> Option<SearchCachedResult> {
        let cid = generate_cid(key);
        let bytes = match self.get(&cid).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return None,
            Err(_) => return None,
        };

        match serde_json::from_slice::<SearchCachedResult>(&bytes) {
            Ok(result) => Some(result),
            Err(e) => {
                tracing::warn!(cid = ?cid, error = %e, "corrupt search cache entry, deleting");
                let _ = self.delete(&cid).await;
                None
            }
        }
    }

    /// Stores a cached search result by key.
    pub async fn set_search_result(
        &self,
        key: &SearchCacheKey,
        result: &SearchCachedResult,
    ) -> Result<(), CacheError> {
        let bytes = serde_json::to_vec(result)?;
        let cid = generate_cid(key);
        self.set(&cid, "search", &bytes).await
    }

    /// Retrieves a cached extract result by key, checking TTL expiry.
    ///
    /// On deserialization failure, logs a warning with the CID and attempts to
    /// delete the corrupt entry (ignoring delete failure), then returns `None`.
    pub async fn get_extract_result(&self, key: &ExtractCacheKey) -> Option<ExtractCachedResult> {
        let cid = generate_cid(key);
        let bytes = match self.get(&cid).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return None,
            Err(_) => return None,
        };

        match serde_json::from_slice::<ExtractCachedResult>(&bytes) {
            Ok(result) => Some(result),
            Err(e) => {
                tracing::warn!(cid = ?cid, error = %e, "corrupt extract cache entry, deleting");
                let _ = self.delete(&cid).await;
                None
            }
        }
    }

    /// Stores a cached extract result by key.
    pub async fn set_extract_result(
        &self,
        key: &ExtractCacheKey,
        result: &ExtractCachedResult,
    ) -> Result<(), CacheError> {
        let bytes = serde_json::to_vec(result)?;
        let cid = generate_cid(key);
        self.set(&cid, "extract", &bytes).await
    }

    /// Deletes a cache entry by CID.
    pub(crate) async fn delete(&self, cid: &Cid) -> Result<(), CacheError> {
        let mut conn = self.open_connection().await?;
        sqlx::query("DELETE FROM cache WHERE cid = ?")
            .bind(cid.as_slice())
            .execute(&mut conn)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kagi_api::{ExtractData, Meta, SearchData, SearchResponse};

    #[tokio::test]
    async fn when_new_then_creates_directory_and_sqlite_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().join("cache");
        let store = CacheStore::new(&cache_dir, 1.0, 7).await.unwrap();

        assert!(cache_dir.exists());
        assert!(cache_dir.join("cache.db").exists());

        let result = store.get(&[0u8; 16]).await.unwrap();
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

        let result = store.get(&[0u8; 16]).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn when_entry_exists_then_get_should_return_data() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let cid = [1u8; 16];

        store.set(&cid, "search", b"cached_data").await.unwrap();
        let result = store.get(&cid).await.unwrap();

        assert_eq!(result, Some(b"cached_data".to_vec()));
    }

    #[tokio::test]
    async fn when_entry_missing_then_get_should_return_none() {
        let store = CacheStore::open_in_memory().await.unwrap();

        let result = store.get(&[0u8; 16]).await.unwrap();

        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn when_entry_expired_then_get_should_return_none_and_delete() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let mut conn = store.open_connection().await.unwrap();
        let cid = [1u8; 16];

        sqlx::query(
            "INSERT INTO cache (cid, type, created_at, size_bytes, value) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(cid.as_slice())
        .bind("search")
        .bind(0i64)
        .bind(4i64)
        .bind(b"data" as &[u8])
        .execute(&mut conn)
        .await
        .unwrap();

        let result = store.get(&cid).await.unwrap();

        assert_eq!(result, None);

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cache WHERE cid = ?")
            .bind(cid.as_slice())
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert_eq!(count.0, 0);
    }

    #[tokio::test]
    async fn when_set_twice_then_get_should_return_latest() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let cid = [1u8; 16];

        store.set(&cid, "search", b"first").await.unwrap();
        store.set(&cid, "search", b"second").await.unwrap();

        let result = store.get(&cid).await.unwrap();

        assert_eq!(result, Some(b"second".to_vec()));
    }

    #[tokio::test]
    async fn when_set_and_get_then_should_roundtrip() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let data = b"roundtrip_payload";
        let cid = [2u8; 16];

        store.set(&cid, "extract", data).await.unwrap();
        let result = store.get(&cid).await.unwrap();

        assert_eq!(result, Some(data.to_vec()));
    }

    #[tokio::test]
    async fn when_overwrite_then_timestamp_should_update() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let mut conn = store.open_connection().await.unwrap();
        let cid = [1u8; 16];

        sqlx::query(
            "INSERT INTO cache (cid, type, created_at, size_bytes, value) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(cid.as_slice())
        .bind("search")
        .bind(0i64)
        .bind(5i64)
        .bind(b"first" as &[u8])
        .execute(&mut conn)
        .await
        .unwrap();

        store.set(&cid, "search", b"second").await.unwrap();

        let timestamp: (i64,) = sqlx::query_as("SELECT created_at FROM cache WHERE cid = ?")
            .bind(cid.as_slice())
            .fetch_one(&mut conn)
            .await
            .unwrap();
        assert!(timestamp.0 > 0);
    }

    #[tokio::test]
    async fn when_search_result_roundtrip_then_should_return_same_data() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let key = SearchCacheKey {
            query: "rust programming".to_owned(),
            workflow: None,
            page: None,
            limit: None,
            safe_search: None,
            region: None,
            filters: None,
            lens_id: None,
            lens: None,
            personalizations: None,
        };
        let result = SearchCachedResult {
            response: SearchResponse {
                meta: Meta {
                    trace: "trace-123".to_owned(),
                    node: Some("node-1".to_owned()),
                    ms: Some(42),
                },
                data: SearchData {
                    search: None,
                    image: None,
                    video: None,
                    podcast: None,
                    podcast_creator: None,
                    news: None,
                    adjacent_question: None,
                    direct_answer: None,
                    interesting_news: None,
                    interesting_finds: None,
                    infobox: None,
                    code: None,
                    package_tracking: None,
                    public_records: None,
                    weather: None,
                    related_search: None,
                    listicle: None,
                    web_archive: None,
                },
            },
        };

        store.set_search_result(&key, &result).await.unwrap();
        let retrieved = store.get_search_result(&key).await;

        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().response.meta.trace,
            result.response.meta.trace
        );
    }

    #[tokio::test]
    async fn when_extract_result_roundtrip_then_should_return_same_data() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let key = ExtractCacheKey {
            url: "https://example.com".to_owned(),
        };
        let result = ExtractCachedResult {
            data: ExtractData {
                url: "https://example.com".to_owned(),
                markdown: Some("# Hello\n\nWorld.".to_owned()),
                error: None,
            },
        };

        store.set_extract_result(&key, &result).await.unwrap();
        let retrieved = store.get_extract_result(&key).await;

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data.url, result.data.url);
    }

    #[tokio::test]
    async fn when_corrupt_entry_then_get_search_result_should_return_none_and_delete() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let key = SearchCacheKey {
            query: "corrupt test".to_owned(),
            workflow: None,
            page: None,
            limit: None,
            safe_search: None,
            region: None,
            filters: None,
            lens_id: None,
            lens: None,
            personalizations: None,
        };
        let cid = generate_cid(&key);

        store.set(&cid, "search", b"not valid json").await.unwrap();

        let result = store.get_search_result(&key).await;
        assert!(result.is_none());

        let raw = store.get(&cid).await.unwrap();
        assert_eq!(raw, None);
    }

    #[tokio::test]
    async fn when_corrupt_entry_delete_fails_then_should_not_panic() {
        let store = CacheStore::open_in_memory().await.unwrap();
        let key = SearchCacheKey {
            query: "delete fail test".to_owned(),
            workflow: None,
            page: None,
            limit: None,
            safe_search: None,
            region: None,
            filters: None,
            lens_id: None,
            lens: None,
            personalizations: None,
        };
        let cid = generate_cid(&key);

        store.set(&cid, "search", b"not valid json").await.unwrap();

        let result = store.get_search_result(&key).await;
        assert!(result.is_none());
    }
}
