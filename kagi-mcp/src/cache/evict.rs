use crate::cache::CacheError;
use sqlx::SqliteConnection;

/// Evicts the oldest cache entries until the total size is at or below `max_size_bytes`.
///
/// Returns the number of bytes freed. If no eviction is needed, returns 0.
/// A WAL checkpoint is performed after any deletions to reclaim disk space.
pub async fn evict_if_needed(
    conn: &mut SqliteConnection,
    max_size_bytes: u64,
) -> Result<u64, CacheError> {
    let total: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries")
        .fetch_one(&mut *conn)
        .await?;
    let total = total.0 as u64;

    if total <= max_size_bytes {
        return Ok(0);
    }

    let bytes_to_free = total - max_size_bytes;
    let mut freed: u64 = 0;

    let entries: Vec<(String, i64)> =
        sqlx::query_as("SELECT cache_key, size_bytes FROM cache_entries ORDER BY created_at ASC")
            .fetch_all(&mut *conn)
            .await?;

    for (cache_key, size_bytes) in entries {
        if freed >= bytes_to_free {
            break;
        }
        sqlx::query("DELETE FROM cache_entries WHERE cache_key = ?1")
            .bind(&cache_key)
            .execute(&mut *conn)
            .await?;
        freed += size_bytes as u64;
    }

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&mut *conn)
        .await?;

    Ok(freed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::ConnectOptions;

    async fn open_test_conn() -> SqliteConnection {
        SqliteConnectOptions::new().connect().await.unwrap()
    }

    async fn setup_schema(conn: &mut SqliteConnection) {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS cache_entries (
                cache_key TEXT PRIMARY KEY,
                tool_type TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                size_bytes INTEGER NOT NULL,
                response_json BLOB NOT NULL
            )",
        )
        .execute(&mut *conn)
        .await
        .unwrap();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_created_at ON cache_entries(created_at)")
            .execute(&mut *conn)
            .await
            .unwrap();

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tool_type ON cache_entries(tool_type)")
            .execute(&mut *conn)
            .await
            .unwrap();
    }

    async fn insert_entry(
        conn: &mut SqliteConnection,
        key: &str,
        created_at: i64,
        size_bytes: u64,
    ) {
        sqlx::query(
            "INSERT INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json)
             VALUES (?1, 'search', ?2, ?3, '{}')",
        )
        .bind(key)
        .bind(created_at)
        .bind(size_bytes as i64)
        .execute(&mut *conn)
        .await
        .unwrap();
    }

    async fn total_size(conn: &mut SqliteConnection) -> u64 {
        let total: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries")
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        total.0 as u64
    }

    async fn count_entries(conn: &mut SqliteConnection) -> u64 {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cache_entries")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        count.0 as u64
    }

    #[tokio::test]
    async fn when_total_under_limit_then_evict_should_free_zero() {
        let mut conn = open_test_conn().await;
        setup_schema(&mut conn).await;
        insert_entry(&mut conn, "key1", 1000, 100).await;
        insert_entry(&mut conn, "key2", 2000, 100).await;
        insert_entry(&mut conn, "key3", 3000, 100).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 0);
        assert_eq!(count_entries(&mut conn).await, 3);
    }

    #[tokio::test]
    async fn when_over_limit_then_evict_should_remove_oldest_first() {
        let mut conn = open_test_conn().await;
        setup_schema(&mut conn).await;
        insert_entry(&mut conn, "old", 1000, 100).await;
        insert_entry(&mut conn, "mid", 2000, 100).await;
        insert_entry(&mut conn, "new", 3000, 100).await;

        let freed = evict_if_needed(&mut conn, 200).await.unwrap();

        assert_eq!(freed, 100);
        assert_eq!(count_entries(&mut conn).await, 2);
        let exists: (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM cache_entries WHERE cache_key = 'old')")
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert!(!exists.0);
    }

    #[tokio::test]
    async fn when_over_limit_then_evict_should_free_enough_bytes() {
        let mut conn = open_test_conn().await;
        setup_schema(&mut conn).await;
        insert_entry(&mut conn, "a", 1000, 200).await;
        insert_entry(&mut conn, "b", 2000, 200).await;
        insert_entry(&mut conn, "c", 3000, 200).await;

        let freed = evict_if_needed(&mut conn, 300).await.unwrap();

        assert!(freed >= 300);
        assert!(total_size(&mut conn).await <= 300);
    }

    #[tokio::test]
    async fn when_single_entry_exceeds_limit_then_evict_should_remove_it() {
        let mut conn = open_test_conn().await;
        setup_schema(&mut conn).await;
        insert_entry(&mut conn, "big", 1000, 1000).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 1000);
        assert_eq!(count_entries(&mut conn).await, 0);
    }

    #[tokio::test]
    async fn when_store_empty_then_evict_should_not_panic() {
        let mut conn = open_test_conn().await;
        setup_schema(&mut conn).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 0);
    }

    #[tokio::test]
    async fn when_total_equals_limit_then_evict_should_free_zero() {
        let mut conn = open_test_conn().await;
        setup_schema(&mut conn).await;
        insert_entry(&mut conn, "key1", 1000, 250).await;
        insert_entry(&mut conn, "key2", 2000, 250).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 0);
        assert_eq!(count_entries(&mut conn).await, 2);
    }
}
