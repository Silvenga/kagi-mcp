use serde::Serialize;

use super::{Lens, Personalizations, SearchExtractConfig};

#[cfg(test)]
use super::{DomainKind, PersonalizationDomain, PersonalizationRegex};

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
    /// Maximum number of results to return. Must be between 1 and 1024. This does not change the amount of results requested, it only limits the maximum amount returned. If omitted, the API always gives you the most results we can get in a single pass.
    #[serde(skip_serializing_if = "Option::is_none")]
    limit: Option<u32>,
    /// Whether safe search is enabled, omitting potentially NSFW content.
    #[serde(skip_serializing_if = "Option::is_none")]
    safe_search: Option<bool>,
    /// Requests results localized to a specific region.
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<String>,
    /// Filters to apply to search results for more targeted queries. NOTE: Any parameter here that overlaps with lenses will take priority over the lens.
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<Filters>,
    /// Lens to apply to the search. Can be a built-in lens's identifier or a lens ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    lens_id: Option<String>,
    /// Inline description of a lens to apply to the search.
    #[serde(skip_serializing_if = "Option::is_none")]
    lens: Option<Lens>,
    /// Configuration for extracting page content from search results.
    #[serde(skip_serializing_if = "Option::is_none")]
    extract: Option<SearchExtractConfig>,
    /// Personalization rules to customize search result ranking.
    #[serde(skip_serializing_if = "Option::is_none")]
    personalizations: Option<Personalizations>,
}

impl SearchRequest {
    /// Create a new search request with the given query.
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
            lens_id: None,
            lens: None,
            extract: None,
            personalizations: None,
        }
    }

    /// The search query string.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The result type filter, if set.
    pub fn workflow(&self) -> Option<&str> {
        self.workflow.as_deref()
    }

    /// The requested response format, if set.
    pub fn format(&self) -> Option<&str> {
        self.format.as_deref()
    }

    /// The timeout for the search, in seconds.
    pub fn timeout_seconds(&self) -> Option<f64> {
        self.timeout_seconds
    }

    /// The page number for paginated results.
    pub fn page(&self) -> Option<u32> {
        self.page
    }

    /// The maximum number of results to return.
    pub fn limit(&self) -> Option<u32> {
        self.limit
    }

    /// Whether safe search is enabled.
    pub fn safe_search(&self) -> Option<bool> {
        self.safe_search
    }

    /// The region filter, if set.
    pub fn region(&self) -> Option<&str> {
        self.region.as_deref()
    }

    /// The search filters, if set.
    pub fn filters(&self) -> Option<&Filters> {
        self.filters.as_ref()
    }

    /// The lens ID to apply, if set.
    pub fn lens_id(&self) -> Option<&str> {
        self.lens_id.as_deref()
    }

    /// The inline lens configuration, if set.
    pub fn lens(&self) -> Option<&Lens> {
        self.lens.as_ref()
    }

    /// The extract configuration, if set.
    pub fn extract(&self) -> Option<&SearchExtractConfig> {
        self.extract.as_ref()
    }

    /// The personalization rules, if set.
    pub fn personalizations(&self) -> Option<&Personalizations> {
        self.personalizations.as_ref()
    }

    /// Set the result type filter.
    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflow = Some(workflow.into());
        self
    }

    /// Set the response format.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set the timeout for the search.
    pub fn with_timeout_seconds(mut self, timeout: f64) -> Self {
        self.timeout_seconds = Some(timeout);
        self
    }

    /// Set the page number for pagination.
    pub fn with_page(mut self, page: u32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the maximum number of results.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Enable or disable safe search.
    pub fn with_safe_search(mut self, safe_search: bool) -> Self {
        self.safe_search = Some(safe_search);
        self
    }

    /// Set the region filter.
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the search filters.
    pub fn with_filters(mut self, filters: Filters) -> Self {
        self.filters = Some(filters);
        self
    }

    /// Set the lens ID to apply.
    pub fn with_lens_id(mut self, lens_id: impl Into<String>) -> Self {
        self.lens_id = Some(lens_id.into());
        self
    }

    /// Set the inline lens configuration.
    pub fn with_lens(mut self, lens: Lens) -> Self {
        self.lens = Some(lens);
        self
    }

    /// Set the extract configuration.
    pub fn with_extract(mut self, extract: SearchExtractConfig) -> Self {
        self.extract = Some(extract);
        self
    }

    /// Set the personalization rules.
    pub fn with_personalizations(mut self, personalizations: Personalizations) -> Self {
        self.personalizations = Some(personalizations);
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
            })
            .with_lens_id("built-in-lens")
            .with_lens(Lens {
                sites_included: Some(vec!["example.com".to_owned()]),
                sites_excluded: None,
                keywords_included: None,
                keywords_excluded: None,
                file_type: None,
                time_after: None,
                time_before: None,
                time_relative: None,
                search_region: None,
            })
            .with_extract(SearchExtractConfig {
                count: Some(5),
                timeout: Some(2.0),
            })
            .with_personalizations(Personalizations {
                domains: Some(vec![PersonalizationDomain {
                    domain: "example.com".to_owned(),
                    kind: DomainKind::Raise,
                }]),
                regexes: None,
            });
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"rust\""));
        assert!(json.contains("\"timeout\":4.0"));
        assert!(!json.contains("\"timeout_seconds\""));
        assert!(json.contains("\"lens_id\":\"built-in-lens\""));
        assert!(json.contains("\"lens\":"));
        assert!(json.contains("\"extract\":"));
        assert!(json.contains("\"personalizations\":"));
    }

    #[test]
    fn when_search_request_with_new_fields_then_should_serialize_correctly() {
        let request = SearchRequest::new("test")
            .with_lens_id("my-lens")
            .with_lens(Lens {
                sites_included: None,
                sites_excluded: None,
                keywords_included: None,
                keywords_excluded: None,
                file_type: None,
                time_after: None,
                time_before: None,
                time_relative: None,
                search_region: None,
            })
            .with_extract(SearchExtractConfig {
                count: Some(3),
                timeout: None,
            })
            .with_personalizations(Personalizations {
                domains: None,
                regexes: Some(vec![PersonalizationRegex {
                    regex: Some(r"^https?://.*".to_owned()),
                    replacement: Some("https://example.com".to_owned()),
                }]),
            });
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"test\""));
        assert!(json.contains("\"lens_id\":\"my-lens\""));
        assert!(json.contains("\"lens\":{}"));
        assert!(json.contains("\"extract\":{\"count\":3}"));
        assert!(json.contains("\"personalizations\":{\"regexes\""));
    }
}
