use crate::cache::Cid;
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
    let total: (i64,) = sqlx::query_as("SELECT COALESCE(SUM(size_bytes), 0) FROM cache")
        .fetch_one(&mut *conn)
        .await?;
    let total = total.0 as u64;

    if total <= max_size_bytes {
        return Ok(0);
    }

    let bytes_to_free = total - max_size_bytes;
    let mut freed: u64 = 0;

    let entries: Vec<(Vec<u8>, i64)> =
        sqlx::query_as("SELECT cid, size_bytes FROM cache ORDER BY created_at ASC")
            .fetch_all(&mut *conn)
            .await?;

    for (cid_vec, size_bytes) in entries {
        let cid: Cid = cid_vec
            .try_into()
            .map_err(|_| CacheError::CorruptEntry("cid is not 16 bytes".into()))?;
        if freed >= bytes_to_free {
            break;
        }
        sqlx::query("DELETE FROM cache WHERE cid = ?1")
            .bind(&cid[..])
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

    async fn insert_entry(
        conn: &mut SqliteConnection,
        key: &Cid,
        created_at: i64,
        size_bytes: u64,
    ) {
        sqlx::query(
            "INSERT INTO cache (cid, type, created_at, size_bytes, value)
             VALUES (?1, 'search', ?2, ?3, X'')",
        )
        .bind(&key[..])
        .bind(created_at)
        .bind(size_bytes as i64)
        .execute(&mut *conn)
        .await
        .unwrap();
    }

    async fn total_size(conn: &mut SqliteConnection) -> u64 {
        let total: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(size_bytes), 0) FROM cache")
                .fetch_one(&mut *conn)
                .await
                .unwrap();
        total.0 as u64
    }

    async fn count_entries(conn: &mut SqliteConnection) -> u64 {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cache")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        count.0 as u64
    }

    #[tokio::test]
    async fn when_total_under_limit_then_evict_should_free_zero() {
        let mut conn = open_test_conn().await;
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
        insert_entry(&mut conn, &[0u8; 16], 1000, 100).await;
        insert_entry(&mut conn, &[1u8; 16], 2000, 100).await;
        insert_entry(&mut conn, &[2u8; 16], 3000, 100).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 0);
        assert_eq!(count_entries(&mut conn).await, 3);
    }

    #[tokio::test]
    async fn when_over_limit_then_evict_should_remove_oldest_first() {
        let mut conn = open_test_conn().await;
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
        insert_entry(&mut conn, &[0u8; 16], 1000, 100).await;
        insert_entry(&mut conn, &[1u8; 16], 2000, 100).await;
        insert_entry(&mut conn, &[2u8; 16], 3000, 100).await;

        let freed = evict_if_needed(&mut conn, 200).await.unwrap();

        assert_eq!(freed, 100);
        assert_eq!(count_entries(&mut conn).await, 2);
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM cache WHERE cid = ?1)")
                .bind(&[0u8; 16][..])
                .fetch_one(&mut conn)
                .await
                .unwrap();
        assert!(!exists);
    }

    #[tokio::test]
    async fn when_over_limit_then_evict_should_free_enough_bytes() {
        let mut conn = open_test_conn().await;
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
        insert_entry(&mut conn, &[0u8; 16], 1000, 200).await;
        insert_entry(&mut conn, &[1u8; 16], 2000, 200).await;
        insert_entry(&mut conn, &[2u8; 16], 3000, 200).await;

        let freed = evict_if_needed(&mut conn, 300).await.unwrap();

        assert!(freed >= 300);
        assert!(total_size(&mut conn).await <= 300);
    }

    #[tokio::test]
    async fn when_single_entry_exceeds_limit_then_evict_should_remove_it() {
        let mut conn = open_test_conn().await;
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
        insert_entry(&mut conn, &[0u8; 16], 1000, 1000).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 1000);
        assert_eq!(count_entries(&mut conn).await, 0);
    }

    #[tokio::test]
    async fn when_store_empty_then_evict_should_not_panic() {
        let mut conn = open_test_conn().await;
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 0);
    }

    #[tokio::test]
    async fn when_total_equals_limit_then_evict_should_free_zero() {
        let mut conn = open_test_conn().await;
        sqlx::migrate!("./migrations").run(&mut conn).await.unwrap();
        insert_entry(&mut conn, &[0u8; 16], 1000, 250).await;
        insert_entry(&mut conn, &[1u8; 16], 2000, 250).await;

        let freed = evict_if_needed(&mut conn, 500).await.unwrap();

        assert_eq!(freed, 0);
        assert_eq!(count_entries(&mut conn).await, 2);
    }
}
