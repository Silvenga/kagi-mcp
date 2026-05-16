use crate::cache::error::CacheError;
use rusqlite::Connection;

/// Evicts the oldest cache entries until the total size is at or below `max_size_bytes`.
///
/// Returns the number of bytes freed. If no eviction is needed, returns 0.
/// A WAL checkpoint is performed after any deletions to reclaim disk space.
pub fn evict_if_needed(conn: &Connection, max_size_bytes: u64) -> Result<u64, CacheError> {
    let total: u64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
        [],
        |row| row.get::<_, i64>(0),
    )? as u64;

    if total <= max_size_bytes {
        return Ok(0);
    }

    let bytes_to_free = total - max_size_bytes;
    let mut freed: u64 = 0;

    let mut stmt =
        conn.prepare("SELECT cache_key, size_bytes FROM cache_entries ORDER BY created_at ASC")?;
    let entries: Vec<(String, u64)> = stmt
        .query_map([], |row| {
            let size_bytes: i64 = row.get(1)?;
            Ok((row.get::<_, String>(0)?, size_bytes as u64))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    for (cache_key, size_bytes) in entries {
        if freed >= bytes_to_free {
            break;
        }
        conn.execute(
            "DELETE FROM cache_entries WHERE cache_key = ?1",
            [&cache_key],
        )?;
        freed += size_bytes;
    }

    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

    Ok(freed)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn open_test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn
    }

    fn insert_entry(conn: &Connection, key: &str, created_at: i64, size_bytes: u64) {
        conn.execute(
            "INSERT INTO cache_entries (cache_key, tool_type, created_at, size_bytes, response_json)
             VALUES (?1, 'search', ?2, ?3, '{}')",
            (key, created_at, size_bytes as i64),
        )
        .unwrap();
    }

    fn total_size(conn: &Connection) -> u64 {
        conn.query_row(
            "SELECT COALESCE(SUM(size_bytes), 0) FROM cache_entries",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap() as u64
    }

    fn count_entries(conn: &Connection) -> u64 {
        conn.query_row("SELECT COUNT(*) FROM cache_entries", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap() as u64
    }

    #[test]
    fn when_total_under_limit_then_evict_should_free_zero() {
        let conn = open_test_conn();
        insert_entry(&conn, "key1", 1000, 100);
        insert_entry(&conn, "key2", 2000, 100);
        insert_entry(&conn, "key3", 3000, 100);

        let freed = evict_if_needed(&conn, 500).unwrap();

        assert_eq!(freed, 0);
        assert_eq!(count_entries(&conn), 3);
    }

    #[test]
    fn when_over_limit_then_evict_should_remove_oldest_first() {
        let conn = open_test_conn();
        insert_entry(&conn, "old", 1000, 100);
        insert_entry(&conn, "mid", 2000, 100);
        insert_entry(&conn, "new", 3000, 100);

        let freed = evict_if_needed(&conn, 200).unwrap();

        assert_eq!(freed, 100);
        assert_eq!(count_entries(&conn), 2);
        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM cache_entries WHERE cache_key = 'old')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);
    }

    #[test]
    fn when_over_limit_then_evict_should_free_enough_bytes() {
        let conn = open_test_conn();
        insert_entry(&conn, "a", 1000, 200);
        insert_entry(&conn, "b", 2000, 200);
        insert_entry(&conn, "c", 3000, 200);

        let freed = evict_if_needed(&conn, 300).unwrap();

        assert!(freed >= 300);
        assert!(total_size(&conn) <= 300);
    }

    #[test]
    fn when_single_entry_exceeds_limit_then_evict_should_remove_it() {
        let conn = open_test_conn();
        insert_entry(&conn, "big", 1000, 1000);

        let freed = evict_if_needed(&conn, 500).unwrap();

        assert_eq!(freed, 1000);
        assert_eq!(count_entries(&conn), 0);
    }

    #[test]
    fn when_store_empty_then_evict_should_not_panic() {
        let conn = open_test_conn();

        let freed = evict_if_needed(&conn, 500).unwrap();

        assert_eq!(freed, 0);
    }

    #[test]
    fn when_total_equals_limit_then_evict_should_free_zero() {
        let conn = open_test_conn();
        insert_entry(&conn, "key1", 1000, 250);
        insert_entry(&conn, "key2", 2000, 250);

        let freed = evict_if_needed(&conn, 500).unwrap();

        assert_eq!(freed, 0);
        assert_eq!(count_entries(&conn), 2);
    }
}
