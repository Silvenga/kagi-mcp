#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct DailyMetrics {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub total_extract_requests: i64,
    pub total_search_requests: i64,
    pub total_extract_urls_from_cache: i64,
    pub total_search_requests_from_cache: i64,
    pub failed_extract_urls: i64,
}
