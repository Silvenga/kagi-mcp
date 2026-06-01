#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct DailyMetrics {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub total_extract_requests: u32,
    pub total_search_requests: u32,
    pub total_extract_urls_from_cache: u32,
    pub total_search_requests_from_cache: u32,
    pub failed_extract_urls: u32,
}
