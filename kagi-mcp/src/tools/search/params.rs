use crate::tools::output_format::OutputFormat;
use crate::tools::param_serde::{
    deserialize_optional_string_non_empty, option_positive_u32_schema, option_string_schema,
};
use serde::Deserialize;

/// Parameters for the `search` tool.
#[warn(missing_docs)]
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Search query. Supports Kagi operators: site:, "exact phrases", -negation, inurl:.
    pub query: String,
    /// Result type filter. Use 'images', 'videos', 'news', or 'podcasts' to narrow results.
    /// Omit for general web search.
    #[serde(default, deserialize_with = "deserialize_optional_string_non_empty")]
    #[schemars(schema_with = "option_string_schema")]
    pub workflow: Option<String>,
    /// Date filter (YYYY-MM-DD). Use when the query is time-sensitive.
    #[serde(default, deserialize_with = "deserialize_optional_string_non_empty")]
    #[schemars(schema_with = "option_string_schema")]
    pub after: Option<String>,
    /// Date filter (YYYY-MM-DD). Use when the query is time-sensitive.
    #[serde(default, deserialize_with = "deserialize_optional_string_non_empty")]
    #[schemars(schema_with = "option_string_schema")]
    pub before: Option<String>,
    /// Prefer 'markdown' for human-readable results optimized for LLM consumption.
    /// Use 'json' only when the caller explicitly requests raw structured data.
    #[serde(default)]
    pub output_format: OutputFormat,
    /// Max results per domain group. Use when results feel repetitive from the same site.
    /// Must be >= 1 if set.
    #[serde(default)]
    #[schemars(schema_with = "option_positive_u32_schema")]
    pub limit_per_domain: Option<u32>,
    /// Whether to use cached results. Set to false only if freshness is critical.
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub cache: bool,
}

pub fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn when_search_params_deserialized_without_cache_should_default_to_true() {
        let json = r#"{"query": "test"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[test]
    fn when_search_params_deserialized_with_cache_false_should_be_false() {
        let json = r#"{"query": "test", "cache": false}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert!(!params.cache);
    }

    #[test]
    fn when_search_params_deserialized_with_cache_true_should_be_true() {
        let json = r#"{"query": "test", "cache": true}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert!(params.cache);
    }

    #[test]
    fn when_search_params_deserialized_without_output_format_then_should_default_to_markdown() {
        let json = r#"{"query": "test"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.output_format, OutputFormat::Markdown);
    }

    #[test]
    fn when_search_params_has_empty_after_then_should_be_none() {
        let json = r#"{"query": "test", "after": ""}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.after, None);
    }

    #[test]
    fn when_search_params_has_empty_before_then_should_be_none() {
        let json = r#"{"query": "test", "before": ""}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.before, None);
    }

    #[test]
    fn when_search_params_has_empty_workflow_then_should_be_none() {
        let json = r#"{"query": "test", "workflow": ""}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.workflow, None);
    }

    #[test]
    fn when_search_params_has_null_after_then_should_be_none() {
        let json = r#"{"query": "test", "after": null}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.after, None);
    }

    #[test]
    fn when_search_params_has_null_workflow_then_should_be_none() {
        let json = r#"{"query": "test", "workflow": null}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.workflow, None);
    }

    #[test]
    fn when_search_params_omits_optional_strings_then_all_should_be_none() {
        let json = r#"{"query": "test"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.workflow, None);
        assert_eq!(params.after, None);
        assert_eq!(params.before, None);
    }

    #[test]
    fn when_search_params_has_nonempty_workflow_then_should_keep_value() {
        let json = r#"{"query": "test", "workflow": "news"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.workflow, Some("news".to_owned()));
    }

    #[test]
    fn when_search_params_has_nonempty_after_then_should_keep_value() {
        let json = r#"{"query": "test", "after": "2025-01-01"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.after, Some("2025-01-01".to_owned()));
    }
}
