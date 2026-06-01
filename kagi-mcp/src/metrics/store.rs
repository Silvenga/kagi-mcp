use crate::metrics::{DailyMetrics, MetricsError};
use chrono::Datelike;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use sqlx::{ConnectOptions, SqliteConnection};
use std::path::Path;
use std::time::Duration;

pub struct MetricsStore {
    connect_options: SqliteConnectOptions,
}

impl MetricsStore {
    pub async fn new(cache_dir: &Path) -> Result<Self, MetricsError> {
        let db_path = cache_dir.join("cache.db");

        let connect_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5));

        Ok(Self { connect_options })
    }

    #[cfg(test)]
    pub async fn open_in_memory() -> Result<Self, MetricsError> {
        use crate::cache::CacheStore;

        let tmp = Box::leak(Box::new(tempfile::tempdir()?));
        let cache_dir = tmp.path().join("cache");
        let _cache_store = CacheStore::new(&cache_dir, 1.0, 1)
            .await
            .expect("failed to create cache store for test");
        Self::new(&cache_dir).await
    }

    pub(crate) async fn open_connection(&self) -> Result<SqliteConnection, MetricsError> {
        self.connect_options.connect().await.map_err(Into::into)
    }

    pub async fn increment_search_request(&self) {
        let now = chrono::Utc::now();
        let year = now.year() as u32;
        let month = now.month();
        let day = now.day();

        if let Err(e) = self
            .upsert_counter_literal(
                year,
                month,
                day,
                "INSERT INTO metrics (year, month, day, total_search_requests) VALUES (?, ?, ?, 1) \
                 ON CONFLICT(year, month, day) DO UPDATE SET total_search_requests = total_search_requests + 1",
            )
            .await
        {
            tracing::warn!(error = %e, "failed to increment search request metric");
        }
    }

    pub async fn increment_search_cache_hit(&self) {
        let now = chrono::Utc::now();
        let year = now.year() as u32;
        let month = now.month();
        let day = now.day();

        if let Err(e) = self
            .upsert_counter_literal(
                year,
                month,
                day,
                "INSERT INTO metrics (year, month, day, total_search_requests_from_cache) VALUES (?, ?, ?, 1) \
                 ON CONFLICT(year, month, day) DO UPDATE SET total_search_requests_from_cache = total_search_requests_from_cache + 1",
            )
            .await
        {
            tracing::warn!(error = %e, "failed to increment search cache hit metric");
        }
    }

    pub async fn increment_extract_request(&self) {
        let now = chrono::Utc::now();
        let year = now.year() as u32;
        let month = now.month();
        let day = now.day();

        if let Err(e) = self
            .upsert_counter_literal(
                year,
                month,
                day,
                "INSERT INTO metrics (year, month, day, total_extract_requests) VALUES (?, ?, ?, 1) \
                 ON CONFLICT(year, month, day) DO UPDATE SET total_extract_requests = total_extract_requests + 1",
            )
            .await
        {
            tracing::warn!(error = %e, "failed to increment extract request metric");
        }
    }

    pub async fn increment_extract_cache_hits(&self, count: i64) {
        let now = chrono::Utc::now();
        let year = now.year() as u32;
        let month = now.month();
        let day = now.day();

        if let Err(e) = self
            .upsert_counter_literal_with_bind(
                year,
                month,
                day,
                count,
                "INSERT INTO metrics (year, month, day, total_extract_urls_from_cache) VALUES (?, ?, ?, ?) \
                 ON CONFLICT(year, month, day) DO UPDATE SET total_extract_urls_from_cache = total_extract_urls_from_cache + ?",
            )
            .await
        {
            tracing::warn!(error = %e, "failed to increment extract cache hits metric");
        }
    }

    pub async fn increment_extract_failures(&self, count: i64) {
        let now = chrono::Utc::now();
        let year = now.year() as u32;
        let month = now.month();
        let day = now.day();

        if let Err(e) = self
            .upsert_counter_literal_with_bind(
                year,
                month,
                day,
                count,
                "INSERT INTO metrics (year, month, day, failed_extract_urls) VALUES (?, ?, ?, ?) \
                 ON CONFLICT(year, month, day) DO UPDATE SET failed_extract_urls = failed_extract_urls + ?",
            )
            .await
        {
            tracing::warn!(error = %e, "failed to increment extract failures metric");
        }
    }

    async fn upsert_counter_literal(
        &self,
        year: u32,
        month: u32,
        day: u32,
        sql: &'static str,
    ) -> Result<(), MetricsError> {
        let mut conn = self.open_connection().await?;
        sqlx::query(sql)
            .bind(year)
            .bind(month)
            .bind(day)
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    async fn upsert_counter_literal_with_bind(
        &self,
        year: u32,
        month: u32,
        day: u32,
        value: i64,
        sql: &'static str,
    ) -> Result<(), MetricsError> {
        let mut conn = self.open_connection().await?;
        sqlx::query(sql)
            .bind(year)
            .bind(month)
            .bind(day)
            .bind(value)
            .bind(value)
            .execute(&mut conn)
            .await?;

        Ok(())
    }

    pub async fn get_monthly_metrics(
        &self,
        year: u32,
        month: u32,
    ) -> Result<Vec<DailyMetrics>, MetricsError> {
        let mut conn = self.open_connection().await?;
        let rows: Vec<DailyMetrics> = sqlx::query_as(
            "SELECT year, month, day, total_extract_requests, total_search_requests, \
             total_extract_urls_from_cache, total_search_requests_from_cache, failed_extract_urls \
             FROM metrics WHERE year = ? AND month = ? ORDER BY day",
        )
        .bind(year)
        .bind(month)
        .fetch_all(&mut conn)
        .await?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn when_upsert_search_request_then_total_search_requests_increments() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_search_request().await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_search_requests, 1);
    }

    #[tokio::test]
    async fn when_upsert_extract_request_then_total_extract_requests_increments() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_extract_request().await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_extract_requests, 1);
    }

    #[tokio::test]
    async fn when_upsert_search_cache_hit_then_from_cache_increments() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_search_cache_hit().await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_search_requests_from_cache, 1);
    }

    #[tokio::test]
    async fn when_upsert_extract_cache_hits_then_urls_from_cache_increments_by_count() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_extract_cache_hits(3).await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_extract_urls_from_cache, 3);
    }

    #[tokio::test]
    async fn when_upsert_extract_failures_then_failed_extract_urls_increments_by_count() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_extract_failures(2).await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].failed_extract_urls, 2);
    }

    #[tokio::test]
    async fn when_upsert_twice_on_same_day_then_counter_should_be_two() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_search_request().await;
        store.increment_search_request().await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_search_requests, 2);
    }

    #[tokio::test]
    async fn when_get_monthly_metrics_with_data_then_should_return_rows() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_search_request().await;
        store.increment_extract_request().await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].total_search_requests, 1);
        assert_eq!(metrics[0].total_extract_requests, 1);
    }

    #[tokio::test]
    async fn when_get_monthly_metrics_empty_month_then_should_return_empty_vec() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        let metrics = store.get_monthly_metrics(2024, 1).await.unwrap();
        assert!(metrics.is_empty());
    }

    #[tokio::test]
    async fn when_daily_metrics_queried_then_fields_should_match_db_values() {
        let store = MetricsStore::open_in_memory().await.unwrap();

        store.increment_search_request().await;
        store.increment_search_cache_hit().await;
        store.increment_extract_request().await;
        store.increment_extract_cache_hits(5).await;
        store.increment_extract_failures(1).await;

        let now = chrono::Utc::now();
        let metrics = store
            .get_monthly_metrics(now.year() as u32, now.month())
            .await
            .unwrap();
        assert_eq!(metrics.len(), 1);
        let m = &metrics[0];
        assert_eq!(m.total_search_requests, 1);
        assert_eq!(m.total_search_requests_from_cache, 1);
        assert_eq!(m.total_extract_requests, 1);
        assert_eq!(m.total_extract_urls_from_cache, 5);
        assert_eq!(m.failed_extract_urls, 1);
    }
}
