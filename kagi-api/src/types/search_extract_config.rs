use serde::Serialize;

/// Configuration for extracting page content from search results. When
/// provided, the API will fetch and extract the content from the specified
/// number of result pages.
///
/// The resulting page markdown will update the value of the `snippet` field on
/// the respective result item.
///
/// **NOTE:** Use of this option incurs additional cost, billed at your
/// account's rate for the Extract API based on the number of units requested.
/// You will not be charged if there were no results to extract.
#[derive(Debug, Clone, Serialize)]
pub struct SearchExtractConfig {
    /// Number of search results to extract content from. Must be between 1 and
    /// 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// Timeout in seconds for extraction of each page. If omitted, uses the
    /// default timeout. This time budget is in addition to the allocated
    /// top-level search timeout, so that you can control both independently.
    #[serde(rename = "timeout")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_search_extract_config_minimal_then_should_serialize_to_empty_object() {
        let config = SearchExtractConfig {
            count: None,
            timeout: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert_eq!(json, r#"{}"#);
    }

    #[test]
    fn when_search_extract_config_fully_populated_then_should_serialize_correctly() {
        let config = SearchExtractConfig {
            count: Some(5),
            timeout: Some(2.0),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(r#""count":5"#));
        assert!(json.contains(r#""timeout":2.0"#));
    }
}
