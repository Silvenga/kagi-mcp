use serde::Serialize;

/// A search request to the Kagi Search API.
#[derive(Debug, Clone, Serialize)]
pub struct SearchRequest {
    /// Search query to run.
    query: String,
    /// Type of results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow: Option<String>,
    /// **(EXPERIMENTAL)** Format to serialize the API response as. The exact contents and structure of markdown output is still being worked on - please send your feedback!
    #[serde(skip_serializing_if = "Option::is_none")]
    format: Option<String>,
    /// Number of seconds to allow for collecting search results. Lower values will return results more quickly, but may be lower quality or inconsistent between calls. If omitted, will use the latest recommended value by Kagi.
    #[serde(rename = "timeout")]
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout_seconds: Option<f64>,
    /// Page number for paginated results. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
    /// Maximum number of results to return. Must be between 1 and 1024.
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    /// Whether safe search is enabled, omitting potentially NSFW content.
    #[serde(skip_serializing_if = "Option::is_none")]
    safe_search: Option<bool>,
    /// Requests results localized to a specific region.
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    /// Filters to apply to search results for more targeted queries.
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<Filters>,
}

impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            workflow: None,
            format: None,
            timeout_seconds: None,
            page: None,
            limit: None,
            safe_search: None,
            region: None,
            filters: None,
        }
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn workflow(&self) -> Option<&str> {
        self.workflow.as_deref()
    }

    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    pub fn timeout_seconds(&self) -> Option<f64> {
        self.timeout_seconds
    }

    pub fn page(&self) -> Option<u32> {
        self.page
    }

    pub fn limit(&self) -> Option<u32> {
        self.limit
    }

    pub fn safe_search(&self) -> Option<bool> {
        self.safe_search
    }

    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    pub fn filters(&self) -> Option<&Filters> {
        self.filters.as_ref()
    }

    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflow = Some(workflow.into());
        self
    }

    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn with_timeout_seconds(mut self, timeout: f64) -> Self {
        self.timeout_seconds = Some(timeout);
        self
    }

    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_safe_search(mut self, safe_search: bool) -> Self {
        self.safe_search = Some(safe_search);
        self
    }

    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    pub fn with_filters(mut self, filters: Filters) -> Self {
        self.filters = Some(filters);
        self
    }
}

/// Filters applied to search results.
#[derive(Debug, Clone, Serialize)]
pub struct Filters {
    /// Filter for results published or updated after this date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// Filter for results published or updated before this date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    /// Filter results to a specific region using an ISO 3166-1 Alpha-2 country code. See <https://help.kagi.com/api/regions> for supported codes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_minimal_search_request_then_should_serialize_to_query_only() {
        let request = SearchRequest::new("test");
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(json, r#"{"query":"test"}"#);
    }

    #[test]
    fn when_fully_populated_search_request_then_should_serialize_correctly() {
        let request = SearchRequest::new("rust")
            .with_workflow("search")
            .with_format("json")
            .with_timeout_seconds(4.0)
            .with_page(1)
            .with_limit(10)
            .with_safe_search(true)
            .with_region("us")
            .with_filters(Filters {
                after: Some("2023-01-01".to_owned()),
                before: None,
                region: Some("us".to_owned()),
            });
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"rust\""));
        assert!(json.contains("\"timeout\":4.0"));
        assert!(!json.contains("\"timeout_seconds\""));
    }
}
